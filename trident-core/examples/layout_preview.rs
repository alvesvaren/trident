//! Lay out `.trd` files natively and report layout quality, optionally writing an SVG preview.
//!
//! Usage: cargo run --release --example layout_preview -- [--layout NAME] [--unlock] [--svg DIR] FILE...
//!
//! `--unlock` strips every `@pos` first (same as "Remove All Locks" in the editor),
//! `--layout` overrides the file's `@layout:` directive.

use std::path::Path;
use std::time::Instant;

use serde_json::Value;

struct Box2 {
    id: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    shape: &'static str,
    label: String,
}

impl Box2 {
    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

fn shape_of(node: &Value) -> &'static str {
    let mods: Vec<&str> = node["modifiers"].as_array().map(|a| a.iter().filter_map(|m| m.as_str()).collect()).unwrap_or_default();
    for s in ["circle", "diamond", "triangle"] {
        if mods.contains(&s) {
            return s;
        }
    }
    "rectangle"
}

fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let orient = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// Does the segment p-q pass through the box shrunk by `inset`?
fn segment_hits_box(p: (f64, f64), q: (f64, f64), b: &Box2, inset: f64) -> bool {
    let (x0, y0, x1, y1) = (b.x + inset, b.y + inset, b.x + b.w - inset, b.y + b.h - inset);
    if x0 >= x1 || y0 >= y1 {
        return false;
    }
    // Liang–Barsky clipping
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    for (pp, qq) in [(-dx, p.0 - x0), (dx, x1 - p.0), (-dy, p.1 - y0), (dy, y1 - p.1)] {
        if pp == 0.0 {
            if qq < 0.0 {
                return false;
            }
        } else {
            let r = qq / pp;
            if pp < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    t0 < t1
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn main() {
    let mut layout: Option<String> = None;
    let mut unlock = false;
    let mut svg_dir: Option<String> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--layout" => layout = args.next(),
            "--unlock" => unlock = true,
            "--svg" => svg_dir = args.next(),
            _ => files.push(a),
        }
    }

    println!("{:<22} {:>5} {:>5} {:>6} {:>6} {:>6} {:>9} {:>8} {:>8}", "file", "nodes", "edges", "cross", "ovlap", "thru", "len/edge", "aspect", "ms");
    for file in files {
        let mut src = std::fs::read_to_string(&file).expect("read file");
        if unlock {
            src = trident_core::remove_all_pos(&src);
        }
        if let Some(l) = &layout {
            // Drop an existing directive, then prepend ours.
            src = src.lines().filter(|l| !l.trim_start().starts_with("@layout:")).collect::<Vec<_>>().join("\n");
            src = format!("@layout: {l}\n{src}");
        }

        let started = Instant::now();
        let json = trident_core::compile_diagram(&src);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        let out: Value = serde_json::from_str(&json).unwrap();
        if !out["error"].is_null() {
            println!("{file}: error {}", out["error"]);
            continue;
        }

        let boxes: Vec<Box2> = out["nodes"].as_array().cloned().unwrap_or_default().iter().map(|n| {
            let b = &n["bounds"];
            let label = n["label"].as_str().unwrap_or(n["id"].as_str().unwrap()).replace(['_', '~'], "");
            Box2 {
                id: n["id"].as_str().unwrap().to_string(),
                x: b["x"].as_f64().unwrap(),
                y: b["y"].as_f64().unwrap(),
                w: b["w"].as_f64().unwrap(),
                h: b["h"].as_f64().unwrap(),
                shape: shape_of(n),
                label,
            }
        }).collect();
        let idx = |id: &str| boxes.iter().position(|b| b.id == id).unwrap();
        let edges: Vec<(usize, usize)> = out["edges"].as_array().cloned().unwrap_or_default().iter()
            .map(|e| (idx(e["from"].as_str().unwrap()), idx(e["to"].as_str().unwrap())))
            .collect();

        // Quality metrics.
        let mut crossings = 0;
        for i in 0..edges.len() {
            for j in i + 1..edges.len() {
                let (a, b) = edges[i];
                let (c, d) = edges[j];
                if a == c || a == d || b == c || b == d {
                    continue;
                }
                if segments_cross(boxes[a].center(), boxes[b].center(), boxes[c].center(), boxes[d].center()) {
                    crossings += 1;
                }
            }
        }
        let mut overlaps = 0;
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let (p, q) = (&boxes[i], &boxes[j]);
                if p.x < q.x + q.w && q.x < p.x + p.w && p.y < q.y + q.h && q.y < p.y + p.h {
                    overlaps += 1;
                }
            }
        }
        let mut through = 0;
        for &(a, b) in &edges {
            for (k, bx) in boxes.iter().enumerate() {
                if k != a && k != b && segment_hits_box(boxes[a].center(), boxes[b].center(), bx, bx.w.min(bx.h) * 0.2) {
                    through += 1;
                }
            }
        }
        let total_len: f64 = edges.iter().map(|&(a, b)| {
            let (p, q) = (boxes[a].center(), boxes[b].center());
            ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()
        }).sum();
        let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for b in &boxes {
            minx = minx.min(b.x);
            miny = miny.min(b.y);
            maxx = maxx.max(b.x + b.w);
            maxy = maxy.max(b.y + b.h);
        }
        for g in out["groups"].as_array().cloned().unwrap_or_default() {
            let b = &g["bounds"];
            minx = minx.min(b["x"].as_f64().unwrap());
            miny = miny.min(b["y"].as_f64().unwrap() - 20.0);
            maxx = maxx.max(b["x"].as_f64().unwrap() + b["w"].as_f64().unwrap());
            maxy = maxy.max(b["y"].as_f64().unwrap() + b["h"].as_f64().unwrap());
        }
        let name = Path::new(&file).file_stem().unwrap().to_string_lossy().to_string();
        println!(
            "{:<22} {:>5} {:>5} {:>6} {:>6} {:>6} {:>9.0} {:>8.2} {:>8.2}",
            name, boxes.len(), edges.len(), crossings, overlaps, through,
            total_len / edges.len().max(1) as f64, (maxx - minx) / (maxy - miny), ms
        );

        if let Some(dir) = &svg_dir {
            let pad = 20.0;
            let mut s = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{} {} {} {}' width='{}' height='{}' font-family='monospace' font-size='11'>\n<rect x='{}' y='{}' width='100%' height='100%' fill='white'/>\n",
                minx - pad, miny - pad, maxx - minx + 2.0 * pad, maxy - miny + 2.0 * pad,
                maxx - minx + 2.0 * pad, maxy - miny + 2.0 * pad, minx - pad, miny - pad
            );
            for g in out["groups"].as_array().cloned().unwrap_or_default() {
                let b = &g["bounds"];
                s += &format!(
                    "<rect x='{}' y='{}' width='{}' height='{}' fill='#f4f4ff' stroke='#99a' stroke-dasharray='4 3'/><text x='{}' y='{}' fill='#557'>{}</text>\n",
                    b["x"], b["y"], b["w"], b["h"], b["x"], b["y"].as_f64().unwrap() - 4.0, escape(g["id"].as_str().unwrap())
                );
            }
            for &(a, b) in &edges {
                let (p, q) = (boxes[a].center(), boxes[b].center());
                s += &format!("<line x1='{:.1}' y1='{:.1}' x2='{:.1}' y2='{:.1}' stroke='#333'/>\n", p.0, p.1, q.0, q.1);
            }
            for b in &boxes {
                let (cx, cy) = b.center();
                let style = "fill='white' stroke='black' stroke-width='1.5'";
                s += &match b.shape {
                    "circle" => format!("<ellipse cx='{cx}' cy='{cy}' rx='{}' ry='{}' {style}/>", b.w / 2.0, b.h / 2.0),
                    "diamond" => format!("<polygon points='{cx},{} {},{cy} {cx},{} {},{cy}' {style}/>", b.y, b.x + b.w, b.y + b.h, b.x),
                    "triangle" => format!("<polygon points='{cx},{} {},{} {},{}' {style}/>", b.y, b.x + b.w, b.y + b.h, b.x, b.y + b.h),
                    _ => format!("<rect x='{}' y='{}' width='{}' height='{}' {style}/>", b.x, b.y, b.w, b.h),
                };
                let ty = if b.shape == "rectangle" && b.h > 50.0 { b.y + 28.0 } else { cy + 4.0 };
                s += &format!("<text x='{cx}' y='{ty}' text-anchor='middle'>{}</text>\n", escape(&b.label));
            }
            s += "</svg>\n";
            let tag = layout.clone().unwrap_or_else(|| "default".into());
            std::fs::write(format!("{dir}/{name}.{tag}.svg"), s).unwrap();
        }
    }
}
