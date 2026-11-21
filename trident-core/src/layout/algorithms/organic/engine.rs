//! The organic layout engine, working on an abstract graph of boxes.
//!
//! Pipeline:
//! 1. Leaves hanging off a node with other connections (ER attributes, single-use
//!    helper classes, ...) are peeled off as *satellites* of that anchor node.
//!    The anchor reserves a halo of space for them.
//! 2. Each connected component of the remaining *core* is laid out with stress
//!    majorization: graph distances (scaled by box sizes) become geometric distances.
//!    It starts from classical MDS, so it is deterministic and needs no randomness.
//! 3. Crossings and edges running through boxes are removed with local moves
//!    (re-placing a node around its neighbours, or swapping two nodes), followed by
//!    stress polishing that is not allowed to bring the crossings back.
//! 4. Satellites are fanned out into the widest free angular gaps around their anchor.
//! 5. Components are packed into rows, and any remaining box overlaps are pushed apart.
//!
//! Fixed (locked) boxes never move; the free boxes are laid out around them.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::f64::consts::{PI, TAU};

use super::geom::{box_overlap, ray_exit, segment_hits_box, segments_cross, V2};

pub struct Input<'a> {
    /// Width and height of every box.
    pub sizes: &'a [(f64, f64)],
    /// Center position of boxes that must not move.
    pub fixed: &'a [Option<V2>],
    /// Undirected connections between boxes (duplicates and self loops are fine).
    pub edges: &'a [(usize, usize)],
    /// `(upper, lower)` pairs that should preferably point downward (inheritance).
    pub downward: &'a [(usize, usize)],
    /// Where the free boxes would be if nothing were fixed. Used as the starting point
    /// so that locking (dragging) one box leaves the rest of the layout where it was.
    pub initial: Option<&'a [V2]>,
}

#[derive(Debug, Clone)]
pub struct Params {
    /// Minimum clearance between any two boxes.
    pub gap: f64,
    /// Clearance added to the sum of radii to get the ideal edge length.
    pub edge_gap: f64,
    /// Clearance between an anchor and its satellites.
    pub satellite_gap: f64,
    /// Spacing between packed components.
    pub component_gap: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self { gap: 16.0, edge_gap: 44.0, satellite_gap: 22.0, component_gap: 60.0 }
    }
}

const STRESS_WEIGHT: f64 = 1.0;
const CROSS_COST: f64 = 3.0;
const THROUGH_COST: f64 = 4.0;
const OVERLAP_COST: f64 = 6.0;
/// An edge passing close by a box it doesn't belong to.
const NEAR_COST: f64 = 1.2;
/// Two edges leaving a box at almost the same angle.
const ANGLE_COST: f64 = 1.5;
const MIN_ANGLE: f64 = PI / 7.0;
/// How strongly boxes stick to their unlocked position when something is locked,
/// relative to the pull of all other boxes together.
const TETHER: f64 = 0.4;
/// Moves that improve the energy less than this aren't worth disturbing the layout for.
const MIN_GAIN: f64 = 0.05;
/// Components bigger than this only get one starting shape.
const MULTI_START_LIMIT: usize = 60;
/// Roughly how many segment tests untangling may spend per component before it stops
/// early. Counted rather than timed, so the result stays deterministic.
const UNTANGLE_BUDGET: usize = 2_000_000;
/// Edges may graze the corners of a box's bounding rectangle: shapes such as diamonds
/// and ellipses don't fill them.
const THROUGH_INSET: f64 = 0.8;
/// Clearance an edge should keep from boxes it passes by.
const NEAR_MARGIN: f64 = 14.0;

/// What is wrong around one node.
#[derive(Debug, Clone, Copy, Default)]
struct Conflicts {
    /// Edge crossings.
    cross: usize,
    /// Edges running through a box.
    through: usize,
    /// Edges passing uncomfortably close to a box.
    near: usize,
    /// Box overlap, roughly one per overlapping pair.
    overlap: f64,
}

impl Conflicts {
    /// Hard problems, the ones polishing must never make worse.
    fn hard(&self) -> usize {
        self.cross + self.through
    }

    fn is_clean(&self) -> bool {
        self.cross == 0 && self.through == 0 && self.near == 0 && self.overlap <= 0.0
    }

    fn cost(&self) -> f64 {
        CROSS_COST * self.cross as f64 + THROUGH_COST * self.through as f64 + NEAR_COST * self.near as f64 + OVERLAP_COST * self.overlap
    }
}

/// Lay out the graph; returns the center of every box.
pub fn layout(input: &Input, params: &Params) -> Vec<V2> {
    let mut engine = Engine::new(input, params);
    if let Some(initial) = input.initial {
        for v in 0..engine.n {
            if engine.is_free(v) {
                engine.pos[v] = initial[v];
            }
        }
    }
    engine.run(input.initial.is_some());
    engine.pos
}

struct Engine<'a> {
    p: &'a Params,
    n: usize,
    half: Vec<V2>,
    fixed: Vec<Option<V2>>,
    nbrs: Vec<Vec<usize>>,
    edges: Vec<(usize, usize)>,
    downward: Vec<(usize, usize)>,
    /// For satellites: the node they hang off.
    anchor: Vec<Option<usize>>,
    satellites: Vec<Vec<usize>>,
    /// Radius used for spacing core nodes, including room for satellites.
    radius: Vec<f64>,
    pos: Vec<V2>,

    // The "scene" conflicts are counted against: which nodes and edges exist right now.
    active: Vec<bool>,
    scene_edges: Vec<(usize, usize)>,
    incident: Vec<Vec<usize>>,
    /// Energy evaluations spent on the current component.
    evaluations: usize,
    /// The unlocked layout; set while stress should stay close to it.
    initial: Option<Vec<V2>>,
    tether: Option<Vec<V2>>,
}

/// Ideal distances between the core nodes of one component.
struct Distances {
    local: Vec<usize>,
    m: usize,
    d: Vec<f64>,
    /// Stress weights, `1 / d²`.
    w: Vec<f64>,
}

impl Distances {
    fn get(&self, a: usize, b: usize) -> f64 {
        self.d[self.local[a] * self.m + self.local[b]]
    }

    fn weight(&self, a: usize, b: usize) -> f64 {
        self.w[self.local[a] * self.m + self.local[b]]
    }
}

impl<'a> Engine<'a> {
    fn new(input: &Input, p: &'a Params) -> Self {
        let n = input.sizes.len();
        let half: Vec<V2> = input.sizes.iter().map(|&(w, h)| V2::new(w / 2.0, h / 2.0)).collect();

        let mut nbrs = vec![Vec::new(); n];
        for &(a, b) in input.edges {
            if a != b && a < n && b < n && !nbrs[a].contains(&b) {
                nbrs[a].push(b);
                nbrs[b].push(a);
            }
        }
        for list in &mut nbrs {
            list.sort_unstable();
        }
        let edges: Vec<(usize, usize)> = (0..n).flat_map(|a| nbrs[a].iter().filter(move |&&b| a < b).map(move |&b| (a, b))).collect();

        let anchor: Vec<Option<usize>> = (0..n)
            .map(|i| match nbrs[i].as_slice() {
                [a] if input.fixed[i].is_none() && nbrs[*a].len() > 1 => Some(*a),
                _ => None,
            })
            .collect();
        let mut satellites = vec![Vec::new(); n];
        for i in 0..n {
            if let Some(a) = anchor[i] {
                satellites[a].push(i);
            }
        }

        let radius = (0..n)
            .map(|i| {
                // Between the inscribed and circumscribed circle: shapes rarely fill their box.
                let own = 0.5 * (half[i].len() + half[i].x.max(half[i].y));
                // Busy nodes need room around them to keep their edges apart.
                let own = own + 6.0 * nbrs[i].len().saturating_sub(3) as f64;
                let sats = &satellites[i];
                if sats.is_empty() {
                    return own;
                }
                let sat_r = sats.iter().map(|&s| half[s].len()).fold(0.0, f64::max);
                own + (2.0 * sat_r + p.satellite_gap) * (0.12 + 0.05 * sats.len().min(5) as f64)
            })
            .collect();

        Self {
            p,
            n,
            half,
            fixed: input.fixed.to_vec(),
            nbrs,
            edges,
            downward: input.downward.to_vec(),
            anchor,
            satellites,
            radius,
            pos: input.fixed.iter().map(|f| f.unwrap_or_default()).collect(),
            active: vec![false; n],
            scene_edges: Vec::new(),
            incident: vec![Vec::new(); n],
            evaluations: 0,
            initial: input.initial.map(<[V2]>::to_vec),
            tether: None,
        }
    }

    fn is_core(&self, i: usize) -> bool {
        self.anchor[i].is_none()
    }

    fn is_free(&self, i: usize) -> bool {
        self.fixed[i].is_none()
    }

    fn run(&mut self, has_initial: bool) {
        let components = self.core_components();
        for core in &components {
            if has_initial && core.iter().all(|&v| self.is_free(v)) {
                continue; // Nothing fixed here: it would come out exactly as it starts.
            }
            self.layout_core(core);
            self.place_satellites(core);
        }
        let full: Vec<Vec<usize>> = components
            .iter()
            .map(|core| {
                let mut all: Vec<usize> = core.iter().flat_map(|&c| std::iter::once(c).chain(self.satellites[c].iter().copied())).collect();
                all.sort_unstable();
                all
            })
            .collect();
        if !has_initial {
            self.pack(&full);
        }
        self.remove_overlaps();
    }

    /// Connected components of the core graph, each sorted, in order of their first node.
    fn core_components(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.n];
        let mut out = Vec::new();
        for start in 0..self.n {
            if seen[start] || !self.is_core(start) {
                continue;
            }
            seen[start] = true;
            let mut comp = vec![start];
            let mut i = 0;
            while i < comp.len() {
                let v = comp[i];
                i += 1;
                for &u in &self.nbrs[v] {
                    if !seen[u] && self.is_core(u) {
                        seen[u] = true;
                        comp.push(u);
                    }
                }
            }
            comp.sort_unstable();
            out.push(comp);
        }
        out
    }

    // ------------------------------------------------------------------
    // Core layout
    // ------------------------------------------------------------------

    fn layout_core(&mut self, core: &[usize]) {
        let has_fixed = core.iter().any(|&v| !self.is_free(v));
        if core.len() == 1 {
            return; // Either fixed already, or placed at the origin and packed later.
        }

        let dist = self.ideal_distances(core);
        self.set_scene(core);
        if has_fixed {
            // Start from where the boxes already are (the unlocked layout, when known) or
            // grow outwards from the fixed boxes, whichever untangles better. Ties keep
            // the boxes where they were.
            let current: Vec<V2> = core.iter().map(|&v| self.pos[v]).collect();
            self.tether = self.initial.clone();
            self.stress(core, &dist, 300, false);
            let kept = (self.tangle(core), core.iter().map(|&v| self.pos[v]).collect::<Vec<_>>());
            for (&v, &p) in core.iter().zip(&current) {
                self.pos[v] = p;
            }
            let tether = self.tether.take();
            self.init_around_fixed(core, &dist);
            self.stress(core, &dist, 300, false);
            if self.tangle(core) >= kept.0 {
                for (&v, &p) in core.iter().zip(&kept.1) {
                    self.pos[v] = p;
                }
                self.tether = tether;
            }
        } else {
            // Stress only finds a local optimum, so try a few starting shapes and keep the
            // one that ends up least tangled. Ties go to the first, which keeps the
            // choice from flipping back and forth on small edits.
            let mut best: Option<(usize, Vec<V2>)> = None;
            let mut starts = self.mds_starts(core, &dist);
            if core.len() > MULTI_START_LIMIT {
                starts.truncate(1); // Too expensive to try them all on big graphs.
            }
            for start in starts {
                for (&v, &p) in core.iter().zip(&start) {
                    self.pos[v] = p;
                }
                self.stress(core, &dist, 80, false);
                let tangle = self.tangle(core);
                if best.as_ref().is_none_or(|(t, _)| tangle < *t) {
                    best = Some((tangle, core.iter().map(|&v| self.pos[v]).collect()));
                }
            }
            for (&v, &p) in core.iter().zip(&best.unwrap().1) {
                self.pos[v] = p;
            }
            self.stress(core, &dist, 220, false);
            self.orient(core);
        }

        for _ in 0..3 {
            let improved = self.untangle(core, &dist);
            self.stress(core, &dist, 30, true);
            if !improved {
                break;
            }
        }
        self.tether = None;
    }

    /// All-pairs shortest paths where an edge is as long as its endpoints need to be apart.
    fn ideal_distances(&self, core: &[usize]) -> Distances {
        let m = core.len();
        let mut local = vec![usize::MAX; self.n];
        for (li, &v) in core.iter().enumerate() {
            local[v] = li;
        }
        let edge_len = |a: usize, b: usize| self.radius[a] + self.radius[b] + self.p.edge_gap;

        let mut d = vec![f64::INFINITY; m * m];
        for (si, &s) in core.iter().enumerate() {
            let row = &mut d[si * m..(si + 1) * m];
            let mut heap = BinaryHeap::new();
            row[si] = 0.0;
            heap.push((Reverse(0u64), s));
            while let Some((Reverse(bits), v)) = heap.pop() {
                let dv = f64::from_bits(bits);
                if dv > row[local[v]] {
                    continue;
                }
                for &u in &self.nbrs[v] {
                    if local[u] == usize::MAX {
                        continue;
                    }
                    let du = dv + edge_len(v, u);
                    if du < row[local[u]] {
                        row[local[u]] = du;
                        heap.push((Reverse(du.to_bits()), u));
                    }
                }
            }
        }
        // Non-adjacent boxes must at least clear each other.
        for i in 0..m {
            for j in 0..m {
                if i != j {
                    let min = self.radius[core[i]] + self.radius[core[j]] + self.p.gap;
                    d[i * m + j] = d[i * m + j].max(min);
                }
            }
        }
        let w = d.iter().map(|d| if *d > 0.0 { 1.0 / (d * d) } else { 0.0 }).collect();
        Distances { local, m, d, w }
    }

    /// Classical multidimensional scaling: deterministic, globally sensible starts.
    /// Projects onto the top three eigenvector pairs: (1,2), (1,3) and (2,3).
    fn mds_starts(&self, core: &[usize], dist: &Distances) -> Vec<Vec<V2>> {
        let m = core.len();
        let mut b = vec![0.0; m * m];
        for i in 0..m {
            for j in 0..m {
                b[i * m + j] = dist.d[i * m + j].powi(2);
            }
        }
        let row_mean: Vec<f64> = (0..m).map(|i| b[i * m..(i + 1) * m].iter().sum::<f64>() / m as f64).collect();
        let total_mean = row_mean.iter().sum::<f64>() / m as f64;
        for i in 0..m {
            for j in 0..m {
                b[i * m + j] = -0.5 * (b[i * m + j] - row_mean[i] - row_mean[j] + total_mean);
            }
        }

        let mul = |v: &[f64]| -> Vec<f64> { (0..m).map(|i| (0..m).map(|j| b[i * m + j] * v[j]).sum()).collect() };
        let normalize = |v: &mut Vec<f64>| {
            let len = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
            v.iter_mut().for_each(|x| *x /= len);
        };
        // Power iteration, deflated against the eigenvectors found so far.
        let mut eigen: Vec<(Vec<f64>, f64)> = Vec::new();
        for seed in [37, 53, 71] {
            let mut v: Vec<f64> = (0..m).map(|i| ((i * seed + 11) % 97) as f64 / 97.0 - 0.5).collect();
            normalize(&mut v);
            for _ in 0..120 {
                let mut next = mul(&v);
                for (u, _) in &eigen {
                    let dot: f64 = next.iter().zip(u).map(|(a, b)| a * b).sum();
                    next.iter_mut().zip(u).for_each(|(a, b)| *a -= dot * b);
                }
                normalize(&mut next);
                v = next;
            }
            let lambda = v.iter().zip(&mul(&v)).map(|(a, b)| a * b).sum::<f64>().max(0.0);
            eigen.push((v, lambda.sqrt()));
        }

        let scale = dist.d.iter().cloned().fold(0.0, f64::max).max(1.0);
        let mut starts = Vec::new();
        for (a, c) in [(0, 1), (0, 2), (1, 2)] {
            let ((va, sa), (vc, sc)) = (&eigen[a], &eigen[c]);
            if *sc < 1e-6 * scale {
                continue; // The graph is too small or too flat for this pair.
            }
            starts.push(
                (0..m)
                    .map(|i| {
                        // A tiny deterministic wobble keeps degenerate (collinear) starts unfoldable.
                        let wobble = (((i * 7919) % 31) as f64 / 31.0 - 0.5) * 1e-3 * scale;
                        V2::new(va[i] * sa + wobble, vc[i] * sc - wobble)
                    })
                    .collect(),
            );
        }
        if starts.is_empty() {
            starts.push((0..m).map(|i| V2::new(eigen[0].0[i] * eigen[0].1, (((i * 7919) % 31) as f64 / 31.0 - 0.5) * 1e-3 * scale)).collect());
        }
        starts
    }

    /// Grow the free nodes outwards from the fixed ones, breadth first.
    fn init_around_fixed(&mut self, core: &[usize], dist: &Distances) {
        let mut placed: Vec<bool> = (0..self.n).map(|v| !self.is_free(v)).collect();
        let mut remaining: Vec<usize> = core.iter().copied().filter(|&v| self.is_free(v)).collect();
        let golden = PI * (3.0 - 5f64.sqrt());
        let mut k = 0usize;
        while !remaining.is_empty() {
            let ready: Vec<usize> = remaining.iter().copied().filter(|&v| self.nbrs[v].iter().any(|&u| placed[u] && dist.local[u] != usize::MAX)).collect();
            for &v in &ready {
                let around: Vec<usize> = self.nbrs[v].iter().copied().filter(|&u| placed[u] && dist.local[u] != usize::MAX).collect();
                let center = around.iter().fold(V2::default(), |acc, &u| acc + self.pos[u]) * (1.0 / around.len() as f64);
                let reach = around.iter().map(|&u| dist.get(u, v)).sum::<f64>() / around.len() as f64;
                k += 1;
                self.pos[v] = center + V2::from_angle(golden * k as f64) * reach;
            }
            for &v in &ready {
                placed[v] = true;
            }
            remaining.retain(|v| !placed[*v]);
        }
    }

    /// Localized stress majorization (Gauss–Seidel). With `guarded`, a node's update is
    /// rejected when it would add crossings or run edges through boxes.
    fn stress(&mut self, core: &[usize], dist: &Distances, iterations: usize, guarded: bool) {
        let scale = dist.d.iter().cloned().fold(0.0, f64::max).max(1.0);
        for _ in 0..iterations {
            let mut moved = 0.0;
            for &v in core {
                if !self.is_free(v) {
                    continue;
                }
                let pv = self.pos[v];
                let (mut num, mut den) = (V2::default(), 0.0);
                for &u in core {
                    if u == v {
                        continue;
                    }
                    let (d, w) = (dist.get(v, u), dist.weight(v, u));
                    let delta = pv - self.pos[u];
                    let len = delta.len();
                    let dir = if len > 1e-9 { delta * (1.0 / len) } else { V2::from_angle((v * 31 + u) as f64) };
                    num = num + (self.pos[u] + dir * d) * w;
                    den += w;
                }
                if let Some(home) = self.tether.as_ref().map(|t| t[v]) {
                    // Pull towards where the box used to be, so it only moves as far as needed.
                    let pull = TETHER * den;
                    num = num + home * pull;
                    den += pull;
                }
                let target = num * (1.0 / den);
                if guarded {
                    let before = self.conflicts(v);
                    self.pos[v] = target;
                    let after = self.conflicts(v);
                    if after.hard() > before.hard() || after.overlap > before.overlap + 1e-9 {
                        self.pos[v] = pv;
                        continue;
                    }
                } else {
                    self.pos[v] = target;
                }
                moved += (target - pv).len();
            }
            if moved / (core.len() as f64) < 1e-4 * scale {
                break;
            }
        }
    }

    /// Rotate the principal axis horizontal; flip so inheritance points down and
    /// earlier-declared nodes end up top-left.
    fn orient(&mut self, nodes: &[usize]) {
        let count = nodes.len() as f64;
        let centroid = nodes.iter().fold(V2::default(), |acc, &v| acc + self.pos[v]) * (1.0 / count);
        let (mut cxx, mut cyy, mut cxy) = (0.0, 0.0, 0.0);
        for &v in nodes {
            let d = self.pos[v] - centroid;
            cxx += d.x * d.x;
            cyy += d.y * d.y;
            cxy += d.x * d.y;
        }
        let theta = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        let (s, c) = (-theta).sin_cos();
        for &v in nodes {
            let d = self.pos[v] - centroid;
            self.pos[v] = V2::new(d.x * c - d.y * s, d.x * s + d.y * c);
        }

        let in_comp = |v: usize| nodes.binary_search(&v).is_ok();
        let down: f64 = self.downward.iter().filter(|(a, b)| in_comp(*a) && in_comp(*b)).map(|&(a, b)| self.pos[b].y - self.pos[a].y).sum();
        let order_x: f64 = nodes.iter().enumerate().map(|(rank, &v)| (rank as f64 - count / 2.0) * self.pos[v].x).sum();
        let order_y: f64 = nodes.iter().enumerate().map(|(rank, &v)| (rank as f64 - count / 2.0) * self.pos[v].y).sum();
        let flip_y = if down.abs() > 1e-6 { down < 0.0 } else { order_y < 0.0 };
        let flip_x = order_x < 0.0;
        for &v in nodes {
            if flip_x {
                self.pos[v].x = -self.pos[v].x;
            }
            if flip_y {
                self.pos[v].y = -self.pos[v].y;
            }
        }
    }

    // ------------------------------------------------------------------
    // Conflict scoring
    // ------------------------------------------------------------------

    /// Make `nodes` (and the edges among them) the scene that conflicts are counted in.
    fn set_scene(&mut self, nodes: &[usize]) {
        self.active.iter_mut().for_each(|a| *a = false);
        for &v in nodes {
            self.active[v] = true;
        }
        self.scene_edges = self.edges.iter().copied().filter(|&(a, b)| self.active[a] && self.active[b]).collect();
        self.rebuild_incident();
    }

    fn add_to_scene(&mut self, v: usize) {
        self.active[v] = true;
        for &u in &self.nbrs[v] {
            if self.active[u] {
                self.scene_edges.push((v.min(u), v.max(u)));
            }
        }
        self.rebuild_incident();
    }

    fn rebuild_incident(&mut self) {
        self.incident.iter_mut().for_each(|l| l.clear());
        for (ei, &(a, b)) in self.scene_edges.iter().enumerate() {
            self.incident[a].push(ei);
            self.incident[b].push(ei);
        }
    }

    /// Crossings plus edges through boxes, over the whole scene.
    fn tangle(&self, nodes: &[usize]) -> usize {
        self.scene_edge_crossings() + nodes.iter().map(|&v| self.conflicts(v).through).sum::<usize>()
    }

    /// Total number of crossings among the scene's edges.
    fn scene_edge_crossings(&self) -> usize {
        let edges = &self.scene_edges;
        let mut count = 0;
        for (i, &(a, b)) in edges.iter().enumerate() {
            for &(c, d) in &edges[i + 1..] {
                if a != c && a != d && b != c && b != d && segments_cross(self.pos[a], self.pos[b], self.pos[c], self.pos[d]) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Crossings, edges through or near boxes and box overlaps involving node `v`.
    fn conflicts(&self, v: usize) -> Conflicts {
        let mut c = Conflicts::default();
        let margin = V2::new(NEAR_MARGIN, NEAR_MARGIN);
        let check_box = |c: &mut Conflicts, pa: V2, pb: V2, k: usize| {
            // The near box contains the through box, so most boxes are ruled out in one test.
            if segment_hits_box(pa, pb, self.pos[k], self.half[k] + margin) {
                if segment_hits_box(pa, pb, self.pos[k], self.half[k] * THROUGH_INSET) {
                    c.through += 1;
                } else {
                    c.near += 1;
                }
            }
        };
        for &ei in &self.incident[v] {
            let (a, b) = self.scene_edges[ei];
            let (pa, pb) = (self.pos[a], self.pos[b]);
            for &(e, f) in &self.scene_edges {
                if e == a || e == b || f == a || f == b {
                    continue;
                }
                if segments_cross(pa, pb, self.pos[e], self.pos[f]) {
                    c.cross += 1;
                }
            }
            for k in 0..self.n {
                if self.active[k] && k != a && k != b {
                    check_box(&mut c, pa, pb, k);
                }
            }
        }
        for &(e, f) in &self.scene_edges {
            if e != v && f != v {
                check_box(&mut c, self.pos[e], self.pos[f], v);
            }
        }

        let pad = V2::new(self.p.gap / 2.0, self.p.gap / 2.0);
        for k in 0..self.n {
            if k != v && self.active[k] {
                let o = box_overlap(self.pos[v], self.half[v] + pad, self.pos[k], self.half[k] + pad);
                if o.x > 0.0 && o.y > 0.0 {
                    let smaller = (self.half[v].x * self.half[v].y).min(self.half[k].x * self.half[k].y).max(1.0);
                    c.overlap += 1.0 + (o.x * o.y / (4.0 * smaller)).min(1.0);
                }
            }
        }
        c
    }

    /// Penalty for edges at `v`, and edges into `v` at its neighbours, that leave a box
    /// at nearly the same angle.
    fn angle_penalty(&self, v: usize) -> f64 {
        let squeeze = |diff: f64| {
            let diff = diff.abs().rem_euclid(TAU);
            let diff = diff.min(TAU - diff);
            if diff < MIN_ANGLE { (1.0 - diff / MIN_ANGLE).powi(2) } else { 0.0 }
        };
        let other = |ei: usize, at: usize| {
            let (a, b) = self.scene_edges[ei];
            if a == at { b } else { a }
        };
        let mut penalty = 0.0;
        let at_v: Vec<f64> = self.incident[v].iter().map(|&ei| (self.pos[other(ei, v)] - self.pos[v]).angle()).collect();
        for i in 0..at_v.len() {
            for j in i + 1..at_v.len() {
                penalty += squeeze(at_v[i] - at_v[j]);
            }
        }
        for &ei in &self.incident[v] {
            let u = other(ei, v);
            let toward_v = (self.pos[v] - self.pos[u]).angle();
            for &ej in &self.incident[u] {
                if ej != ei {
                    penalty += squeeze(toward_v - (self.pos[other(ej, u)] - self.pos[u]).angle());
                }
            }
        }
        penalty
    }

    /// The stress part of [`Self::energy`]: how far `v` is from its ideal distances.
    /// Everything else in the energy is non-negative, so this is a lower bound for it.
    fn stress_of(&self, v: usize, dist: &Distances, core: &[usize]) -> f64 {
        let mut stress = 0.0;
        for &u in core {
            if u != v {
                let e = (self.pos[v] - self.pos[u]).len() - dist.get(v, u);
                stress += e * e * dist.weight(v, u);
            }
        }
        STRESS_WEIGHT * stress
    }

    fn energy(&mut self, v: usize, dist: &Distances, core: &[usize]) -> f64 {
        self.evaluations += 1;
        self.stress_of(v, dist, core) + self.conflicts(v).cost() + ANGLE_COST * self.angle_penalty(v)
    }

    // ------------------------------------------------------------------
    // Untangling
    // ------------------------------------------------------------------

    fn untangle(&mut self, core: &[usize], dist: &Distances) -> bool {
        let mut any = false;
        // Nodes that found nothing better, and nothing has moved near them since.
        let mut settled = vec![false; self.n];
        for _ in 0..8 {
            if self.evaluations > self.evaluation_budget() {
                break;
            }
            let mut improved = false;
            for &v in core {
                if self.evaluations > self.evaluation_budget() {
                    break;
                }
                if !self.is_free(v) || settled[v] {
                    continue;
                }
                let conflicts = self.conflicts(v);
                if conflicts.is_clean() && self.angle_penalty(v) == 0.0 {
                    continue;
                }
                let before = self.pos[v];
                let moved = if self.try_relocate(v, core, dist) {
                    Some((v, None))
                } else if conflicts.hard() > 0 {
                    self.try_swap(v, core, dist).map(|u| (v, Some(u)))
                } else {
                    None
                };
                match moved {
                    None => settled[v] = true,
                    Some((v, u)) => {
                        improved = true;
                        let mut spots = vec![before, self.pos[v]];
                        if let Some(u) = u {
                            spots.push(self.pos[u]);
                        }
                        self.wake_around(&spots, v, core, dist, &mut settled);
                        if let Some(u) = u {
                            self.wake_around(&spots, u, core, dist, &mut settled);
                        }
                    }
                }
            }
            if !improved {
                break;
            }
            any = true;
        }
        any
    }

    /// Un-settle everything that `moved` (now or before) could affect: its neighbours'
    /// nodes and whatever lies within reach of the `spots` it moved between.
    fn wake_around(&self, spots: &[V2], moved: usize, core: &[usize], dist: &Distances, settled: &mut [bool]) {
        let reach = self.nbrs[moved].iter().filter(|&&u| dist.local[u] != usize::MAX).map(|&u| dist.get(moved, u)).fold(0.0, f64::max) * 1.5;
        for &k in core {
            if settled[k] && (self.nbrs[moved].contains(&k) || spots.iter().any(|&p| (self.pos[k] - p).len() < reach)) {
                settled[k] = false;
            }
        }
        for &u in &self.nbrs[moved] {
            for &k in &self.nbrs[u] {
                settled[k] = false;
            }
        }
    }

    /// How many energy evaluations fit in [`UNTANGLE_BUDGET`] for the current scene.
    fn evaluation_budget(&self) -> usize {
        let nodes = self.active.iter().filter(|a| **a).count().max(1);
        let edges = self.scene_edges.len();
        let per_eval = (2 * edges / nodes + 1) * (edges + nodes);
        UNTANGLE_BUDGET / per_eval.max(1)
    }

    fn candidates(&self, v: usize, dist: &Distances) -> Vec<V2> {
        let around: Vec<usize> = self.nbrs[v].iter().copied().filter(|&u| dist.local[u] != usize::MAX).collect();
        if around.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let center = around.iter().fold(V2::default(), |acc, &u| acc + self.pos[u]) * (1.0 / around.len() as f64);
        let reach = around.iter().map(|&u| dist.get(u, v)).sum::<f64>() / around.len() as f64;
        let radii: &[f64] = if around.len() == 1 { &[1.0] } else { &[0.35, 0.8, 1.4] };
        for &r in radii {
            for k in 0..16 {
                out.push(center + V2::from_angle(TAU * k as f64 / 16.0) * (reach * r));
            }
        }
        out.push(center * 2.0 - self.pos[v]);
        for &u in around.iter().take(4) {
            let d = dist.get(u, v);
            for k in 0..8 {
                out.push(self.pos[u] + V2::from_angle(TAU * (k as f64 + 0.5) / 8.0) * d);
            }
        }
        out
    }

    fn try_relocate(&mut self, v: usize, core: &[usize], dist: &Distances) -> bool {
        let start = self.pos[v];
        let mut best = (self.energy(v, dist, core), start);
        // Cheapest stress first; once the stress alone can't beat the best, nothing can.
        let mut cands: Vec<(f64, V2)> = self
            .candidates(v, dist)
            .into_iter()
            .map(|c| {
                self.pos[v] = c;
                (self.stress_of(v, dist, core), c)
            })
            .collect();
        cands.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (stress, cand) in cands {
            if stress >= best.0 - MIN_GAIN {
                break;
            }
            self.pos[v] = cand;
            let e = self.energy(v, dist, core);
            if e < best.0 - MIN_GAIN {
                best = (e, cand);
            }
        }
        self.pos[v] = best.1;
        best.1 != start
    }

    /// Swap `v` with a nearby node if that helps; returns the node it swapped with.
    fn try_swap(&mut self, v: usize, core: &[usize], dist: &Distances) -> Option<usize> {
        let mut others: Vec<usize> = core.iter().copied().filter(|&u| u != v && self.is_free(u)).collect();
        let pv = self.pos[v];
        others.sort_by(|&a, &b| (self.pos[a] - pv).len().total_cmp(&(self.pos[b] - pv).len()));
        let energy_v = self.energy(v, dist, core);
        for &u in others.iter().take(24) {
            self.pos.swap(v, u);
            let lower_bound = self.stress_of(v, dist, core) + self.stress_of(u, dist, core);
            self.pos.swap(v, u);
            let before = energy_v + self.stress_of(u, dist, core);
            if lower_bound >= before - MIN_GAIN {
                continue; // Can't win even before counting u's conflicts.
            }
            let before = energy_v + self.energy(u, dist, core);
            self.pos.swap(v, u);
            let after = self.energy(v, dist, core) + self.energy(u, dist, core);
            if after < before - MIN_GAIN {
                return Some(u);
            }
            self.pos.swap(v, u);
        }
        None
    }

    // ------------------------------------------------------------------
    // Satellites
    // ------------------------------------------------------------------

    fn place_satellites(&mut self, core: &[usize]) {
        self.set_scene(core);
        for &a in core {
            let sats = self.satellites[a].clone();
            if sats.is_empty() {
                continue;
            }
            let pa = self.pos[a];
            let mut taken: Vec<f64> = self.nbrs[a].iter().filter(|&&u| self.is_core(u)).map(|&u| (self.pos[u] - pa).angle()).collect();
            taken.sort_by(f64::total_cmp);

            for (&s, angle) in sats.iter().zip(fan_out(&taken, sats.len())) {
                // Keep a satellite on the side it was on before something got locked.
                let angle = self.initial.as_ref().map_or(angle, |init| (init[s] - init[a]).angle());
                self.place_satellite(s, a, angle);
                self.add_to_scene(s);
            }
        }
    }

    /// Put satellite `s` near `angle` around its anchor `a`, nudging it where it would
    /// collide with something.
    fn place_satellite(&mut self, s: usize, a: usize, angle: f64) {
        let at = |e: &Self, theta: f64, stretch: f64| {
            let u = V2::from_angle(theta);
            let reach = ray_exit(e.half[a], u) + ray_exit(e.half[s], u) + e.p.satellite_gap * stretch;
            e.pos[a] + u * reach
        };
        // Count the satellite's own edge too while scoring, without committing it.
        self.active[s] = true;
        self.scene_edges.push((a.min(s), a.max(s)));
        let ei = self.scene_edges.len() - 1;
        self.incident[a].push(ei);
        self.incident[s].push(ei);

        let mut best = (f64::INFINITY, V2::default());
        for step in 0..36 {
            // 0, +10°, -10°, +20°, ... all the way around.
            let offset = (((step + 1) / 2) as f64) * (PI / 18.0) * if step % 2 == 1 { 1.0 } else { -1.0 };
            for (si, stretch) in [1.0, 2.5, 4.5].into_iter().enumerate() {
                self.pos[s] = at(self, angle + offset, stretch);
                let cost = self.conflicts(s).cost() + ANGLE_COST * self.angle_penalty(s) + offset.abs() * 0.5 + si as f64 * 0.6;
                if cost < best.0 - 1e-9 {
                    best = (cost, self.pos[s]);
                }
            }
        }
        self.pos[s] = best.1;

        self.incident[a].pop();
        self.incident[s].pop();
        self.scene_edges.pop();
        self.active[s] = false;
    }

    // ------------------------------------------------------------------
    // Packing and cleanup
    // ------------------------------------------------------------------

    fn bounds(&self, nodes: &[usize]) -> (V2, V2) {
        let mut lo = V2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = V2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &v in nodes {
            let (p, h) = (self.pos[v], self.half[v]);
            lo = V2::new(lo.x.min(p.x - h.x), lo.y.min(p.y - h.y));
            hi = V2::new(hi.x.max(p.x + h.x), hi.y.max(p.y + h.y));
        }
        (lo, hi)
    }

    /// Shelf-pack the components that contain no fixed boxes, below any fixed ones.
    fn pack(&mut self, components: &[Vec<usize>]) {
        let fixed_nodes: Vec<usize> = (0..self.n).filter(|&v| !self.is_free(v)).collect();
        let origin = if fixed_nodes.is_empty() {
            V2::default()
        } else {
            let (lo, hi) = self.bounds(&fixed_nodes);
            V2::new(lo.x, hi.y + self.p.component_gap)
        };

        let mut movable: Vec<&Vec<usize>> = components.iter().filter(|c| c.iter().all(|&v| self.is_free(v))).collect();
        movable.sort_by_key(|c| (Reverse(c.len()), c[0]));
        if movable.is_empty() {
            return;
        }
        let sizes: Vec<V2> = movable.iter().map(|c| {
            let (lo, hi) = self.bounds(c);
            hi - lo
        }).collect();
        let area: f64 = sizes.iter().map(|s| (s.x + self.p.component_gap) * (s.y + self.p.component_gap)).sum();
        let widest = sizes.iter().map(|s| s.x).fold(0.0, f64::max);
        let row_width = widest.max(area.sqrt() * 1.5);

        let mut cursor = origin;
        let mut row_height: f64 = 0.0;
        for (comp, size) in movable.iter().zip(&sizes) {
            if cursor.x > origin.x && cursor.x - origin.x + size.x > row_width {
                cursor = V2::new(origin.x, cursor.y + row_height + self.p.component_gap);
                row_height = 0.0;
            }
            let (lo, _) = self.bounds(comp);
            let shift = cursor - lo;
            for &v in comp.iter() {
                self.pos[v] = self.pos[v] + shift;
            }
            cursor.x += size.x + self.p.component_gap;
            row_height = row_height.max(size.y);
        }
    }

    /// Push apart boxes that still overlap, along the axis of least overlap.
    fn remove_overlaps(&mut self) {
        let pad = V2::new(self.p.gap / 2.0, self.p.gap / 2.0);
        for _ in 0..200 {
            let mut moved = false;
            for i in 0..self.n {
                for j in i + 1..self.n {
                    let (fi, fj) = (self.is_free(i), self.is_free(j));
                    if !fi && !fj {
                        continue;
                    }
                    let o = box_overlap(self.pos[i], self.half[i] + pad, self.pos[j], self.half[j] + pad);
                    if o.x <= 0.0 || o.y <= 0.0 {
                        continue;
                    }
                    let delta = self.pos[j] - self.pos[i];
                    let push = if o.x < o.y {
                        V2::new(if delta.x >= 0.0 { o.x } else { -o.x }, 0.0)
                    } else {
                        V2::new(0.0, if delta.y >= 0.0 { o.y } else { -o.y })
                    };
                    let (si, sj) = match (fi, fj) {
                        (true, true) => (0.5, 0.5),
                        (true, false) => (1.0, 0.0),
                        _ => (0.0, 1.0),
                    };
                    self.pos[i] = self.pos[i] - push * (si + 1e-3);
                    self.pos[j] = self.pos[j] + push * (sj + 1e-3);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }
}

/// Spread `count` directions into the widest gaps between the `taken` angles (sorted).
/// Directions come out clockwise, starting from the top.
fn fan_out(taken: &[f64], count: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(count);
    if taken.is_empty() {
        for i in 0..count {
            out.push(-PI / 2.0 + TAU * i as f64 / count as f64 + if count == 2 { PI / 2.0 } else { 0.0 });
        }
        return out;
    }
    // Gaps as (start angle, width).
    let gaps: Vec<(f64, f64)> = (0..taken.len())
        .map(|i| {
            let start = taken[i];
            let end = if i + 1 < taken.len() { taken[i + 1] } else { taken[0] + TAU };
            (start, end - start)
        })
        .collect();
    let mut per_gap = vec![0usize; gaps.len()];
    for _ in 0..count {
        let best = (0..gaps.len()).max_by(|&a, &b| {
            let fa = gaps[a].1 / (per_gap[a] + 2) as f64;
            let fb = gaps[b].1 / (per_gap[b] + 2) as f64;
            fa.total_cmp(&fb).then(b.cmp(&a))
        });
        per_gap[best.unwrap()] += 1;
    }
    for (g, &(start, width)) in gaps.iter().enumerate() {
        for k in 0..per_gap[g] {
            out.push(start + width * (k + 1) as f64 / (per_gap[g] + 1) as f64);
        }
    }
    // Clockwise from the top (screen coordinates: y grows downwards).
    let from_top = |a: f64| (a + PI / 2.0).rem_euclid(TAU);
    out.sort_by(|a, b| from_top(*a).total_cmp(&from_top(*b)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sizes: &[(f64, f64)], edges: &[(usize, usize)], fixed: &[Option<V2>]) -> Vec<V2> {
        layout(&Input { sizes, fixed, edges, downward: &[], initial: None }, &Params::default())
    }

    fn assert_no_overlaps(sizes: &[(f64, f64)], pos: &[V2]) {
        for i in 0..pos.len() {
            for j in i + 1..pos.len() {
                let h = |k: usize| V2::new(sizes[k].0 / 2.0, sizes[k].1 / 2.0);
                let o = box_overlap(pos[i], h(i), pos[j], h(j));
                assert!(o.x <= 0.0 || o.y <= 0.0, "boxes {i} and {j} overlap");
            }
        }
    }

    #[test]
    fn deterministic_and_overlap_free() {
        let sizes = vec![(80.0, 36.0); 8];
        let edges = [(0, 1), (1, 2), (2, 3), (3, 0), (0, 4), (4, 5), (5, 6), (6, 7), (7, 4), (2, 6)];
        let fixed = vec![None; 8];
        let a = run(&sizes, &edges, &fixed);
        let b = run(&sizes, &edges, &fixed);
        assert_eq!(a, b);
        assert_no_overlaps(&sizes, &a);
    }

    #[test]
    fn fixed_boxes_stay_put() {
        let sizes = vec![(60.0, 30.0); 4];
        let fixed = vec![Some(V2::new(500.0, 300.0)), None, None, None];
        let pos = run(&sizes, &[(0, 1), (1, 2), (2, 3)], &fixed);
        assert_eq!(pos[0], V2::new(500.0, 300.0));
        assert_no_overlaps(&sizes, &pos);
    }

    #[test]
    fn planar_cycle_has_no_crossings() {
        // A wheel: hub 0 connected to a 6-cycle.
        let sizes = vec![(70.0, 40.0); 7];
        let mut edges = vec![];
        for i in 1..=6 {
            edges.push((0, i));
            edges.push((i, if i == 6 { 1 } else { i + 1 }));
        }
        let pos = run(&sizes, &edges, &vec![None; 7]);
        for (i, &(a, b)) in edges.iter().enumerate() {
            for &(c, d) in &edges[i + 1..] {
                if a != c && a != d && b != c && b != d {
                    assert!(!segments_cross(pos[a], pos[b], pos[c], pos[d]));
                }
            }
        }
    }

    #[test]
    fn satellites_fan_into_free_gaps() {
        let taken = [0.0, PI / 2.0];
        let dirs = fan_out(&taken, 2);
        // Both satellites belong in the big 270° gap between 90° and 360°.
        for d in dirs {
            let a = d.rem_euclid(TAU);
            assert!(a > PI / 2.0 && a < TAU, "{a}");
        }
    }
}
