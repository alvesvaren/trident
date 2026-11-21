//! Small 2D geometry toolkit for the organic layout.

use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

impl V2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn from_angle(angle: f64) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        V2::new(self.x * k, self.y * k)
    }
}

fn orient(p: V2, q: V2, r: V2) -> f64 {
    (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
}

/// Do segments a-b and c-d properly cross (touching endpoints don't count)?
pub fn segments_cross(a: V2, b: V2, c: V2, d: V2) -> bool {
    // Cheap bounding-box rejection first; this runs in the hot loops.
    if a.x.max(b.x) < c.x.min(d.x)
        || c.x.max(d.x) < a.x.min(b.x)
        || a.y.max(b.y) < c.y.min(d.y)
        || c.y.max(d.y) < a.y.min(b.y)
    {
        return false;
    }
    orient(c, d, a) * orient(c, d, b) < 0.0 && orient(a, b, c) * orient(a, b, d) < 0.0
}

/// Does segment p-q pass through the box centered at `c` with half extents `half`?
pub fn segment_hits_box(p: V2, q: V2, c: V2, half: V2) -> bool {
    let (x0, y0, x1, y1) = (c.x - half.x, c.y - half.y, c.x + half.x, c.y + half.y);
    if p.x.max(q.x) <= x0 || p.x.min(q.x) >= x1 || p.y.max(q.y) <= y0 || p.y.min(q.y) >= y1 {
        return false;
    }
    // Liang–Barsky clipping.
    let d = q - p;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (pp, qq) in [(-d.x, p.x - x0), (d.x, x1 - p.x), (-d.y, p.y - y0), (d.y, y1 - p.y)] {
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

/// Overlap of two centered boxes along each axis (positive on both axes means they overlap).
pub fn box_overlap(c1: V2, h1: V2, c2: V2, h2: V2) -> V2 {
    V2::new(h1.x + h2.x - (c1.x - c2.x).abs(), h1.y + h2.y - (c1.y - c2.y).abs())
}

/// Distance from a box's center to its border along the unit direction `u`.
pub fn ray_exit(half: V2, u: V2) -> f64 {
    let tx = if u.x.abs() > 1e-9 { half.x / u.x.abs() } else { f64::INFINITY };
    let ty = if u.y.abs() > 1e-9 { half.y / u.y.abs() } else { f64::INFINITY };
    tx.min(ty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_segments() {
        let (a, b) = (V2::new(0.0, 0.0), V2::new(10.0, 10.0));
        assert!(segments_cross(a, b, V2::new(0.0, 10.0), V2::new(10.0, 0.0)));
        assert!(!segments_cross(a, b, V2::new(20.0, 0.0), V2::new(30.0, 10.0)));
        // Sharing an endpoint is not a crossing.
        assert!(!segments_cross(a, b, b, V2::new(20.0, 0.0)));
    }

    #[test]
    fn segment_through_box() {
        let half = V2::new(5.0, 5.0);
        assert!(segment_hits_box(V2::new(-20.0, 0.0), V2::new(20.0, 0.0), V2::new(0.0, 0.0), half));
        assert!(!segment_hits_box(V2::new(-20.0, 10.0), V2::new(20.0, 10.0), V2::new(0.0, 0.0), half));
    }
}
