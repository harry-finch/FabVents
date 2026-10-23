//! Géométrie plane de base : vecteur 2D en millimètres, distances, aires.
//!
//! Repère du moteur : Y vers le haut (convention CAO), angles en degrés dans
//! le projet, en radians dans les calculs.

use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Vecteur unitaire d'angle `a` (radians).
    pub fn from_angle(a: f64) -> Self {
        Self::new(a.cos(), a.sin())
    }

    pub fn dot(self, o: Vec2) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// Produit vectoriel (composante Z) : positif si `o` est à gauche de `self`.
    pub fn cross(self, o: Vec2) -> f64 {
        self.x * o.y - self.y * o.x
    }

    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }

    pub fn distance(self, o: Vec2) -> f64 {
        (self - o).length()
    }

    pub fn normalized(self) -> Vec2 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            Vec2::ZERO
        }
    }

    /// Rotation de `a` radians autour de l'origine (sens trigonométrique).
    pub fn rotated(self, a: f64) -> Vec2 {
        let (s, c) = a.sin_cos();
        Vec2::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    /// Normale à droite (rotation de -90°) : normale extérieure d'une arête
    /// d'un contour parcouru dans le sens trigonométrique.
    pub fn right(self) -> Vec2 {
        Vec2::new(self.y, -self.x)
    }

    /// Normale à gauche (rotation de +90°).
    pub fn left(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn lerp(self, o: Vec2, t: f64) -> Vec2 {
        self + (o - self) * t
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        self.x += o.x;
        self.y += o.y;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f64) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// Aire algébrique (formule du lacet) : positive si le contour tourne dans le
/// sens trigonométrique.
pub fn signed_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        s += pts[i].cross(pts[(i + 1) % n]);
    }
    0.5 * s
}

/// Distance d'un point au segment `[a, b]` (un segment nul est un point).
pub fn dist_point_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ab = b - a;
    let l2 = ab.length_sq();
    if l2 <= 1e-24 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn orient(a: Vec2, b: Vec2, c: Vec2) -> f64 {
    (b - a).cross(c - a)
}

/// Vrai si les segments `[a, b]` et `[c, d]` se coupent ou se touchent.
pub fn segments_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    let on = |p: Vec2, q: Vec2, r: Vec2, o: f64| {
        o == 0.0
            && r.x >= p.x.min(q.x)
            && r.x <= p.x.max(q.x)
            && r.y >= p.y.min(q.y)
            && r.y <= p.y.max(q.y)
    };
    on(c, d, a, d1) || on(c, d, b, d2) || on(a, b, c, d3) || on(a, b, d, d4)
}

/// Distance entre deux segments (nulle s'ils se coupent).
pub fn dist_segment_segment(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> f64 {
    if segments_intersect(a, b, c, d) {
        return 0.0;
    }
    dist_point_segment(a, c, d)
        .min(dist_point_segment(b, c, d))
        .min(dist_point_segment(c, a, b))
        .min(dist_point_segment(d, a, b))
}

/// Point dans un polygone (règle pair-impair).
pub fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Segments d'une « âme » de trou : un point, un segment ou un contour fermé.
pub fn core_segments(core: &[Vec2]) -> Vec<(Vec2, Vec2)> {
    match core.len() {
        0 => Vec::new(),
        1 => vec![(core[0], core[0])],
        2 => vec![(core[0], core[1])],
        n => (0..n).map(|i| (core[i], core[(i + 1) % n])).collect(),
    }
}

/// Distance entre deux âmes (point, segment ou polygone fermé). Nulle si
/// elles se coupent ou si l'une contient l'autre.
pub fn core_distance(a: &[Vec2], b: &[Vec2]) -> f64 {
    if (a.len() >= 3 && point_in_polygon(b[0], a)) || (b.len() >= 3 && point_in_polygon(a[0], b)) {
        return 0.0;
    }
    let sa = core_segments(a);
    let sb = core_segments(b);
    let mut best = f64::INFINITY;
    for &(p, q) in &sa {
        for &(r, s) in &sb {
            best = best.min(dist_segment_segment(p, q, r, s));
            if best == 0.0 {
                return 0.0;
            }
        }
    }
    best
}

/// Angle d'un arc de « bulge » DXF : `bulge = tan(angle / 4)`, positif dans le
/// sens trigonométrique.
pub fn bulge_angle(bulge: f64) -> f64 {
    4.0 * bulge.atan()
}

/// Centre et rayon de l'arc qui va de `p` à `q` avec le bulge donné.
pub fn bulge_arc(p: Vec2, q: Vec2, bulge: f64) -> (Vec2, f64) {
    let chord = q - p;
    let c = chord.length();
    let theta = bulge_angle(bulge);
    let r = c / (2.0 * (theta / 2.0).sin().abs());
    let m = p.lerp(q, 0.5);
    // Flèche de l'arc ; le centre est à gauche de la corde pour un arc
    // trigonométrique, à droite pour un arc horaire.
    let s = bulge.abs() * c / 2.0;
    let side = if bulge > 0.0 {
        chord.left()
    } else {
        chord.right()
    };
    (m + side.normalized() * (r - s), r)
}

/// Nombre de segments pour approcher un arc de rayon `r` et d'angle `theta`
/// avec une flèche maximale `tol`.
pub fn arc_segments(r: f64, theta: f64, tol: f64, max: usize) -> usize {
    if r <= tol {
        return 1;
    }
    let step = 2.0 * (1.0 - tol / r).clamp(-1.0, 1.0).acos();
    if step <= 0.0 {
        return max;
    }
    ((theta.abs() / step).ceil() as usize).clamp(1, max)
}

/// Ramène un angle (radians) dans `[0, 2π)`.
pub fn wrap_tau(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    if r.is_finite() {
        r
    } else {
        0.0
    }
}

pub const DEG: f64 = PI / 180.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_orientation() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        assert!((signed_area(&sq) - 4.0).abs() < 1e-12);
        let mut cw = sq;
        cw.reverse();
        assert!((signed_area(&cw) + 4.0).abs() < 1e-12);
    }

    #[test]
    fn segment_distances() {
        let d = dist_segment_segment(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(1.0, 2.0),
        );
        assert!((d - 2.0).abs() < 1e-12);
        let x = dist_segment_segment(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 0.0),
        );
        assert_eq!(x, 0.0);
    }

    #[test]
    fn containment_counts_as_contact() {
        let big = [
            Vec2::new(-5.0, -5.0),
            Vec2::new(5.0, -5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(-5.0, 5.0),
        ];
        assert_eq!(core_distance(&big, &[Vec2::ZERO]), 0.0);
    }

    #[test]
    fn semicircle_bulge() {
        let (c, r) = bulge_arc(Vec2::new(-1.0, 1.0), Vec2::new(-1.0, -1.0), 1.0);
        assert!((r - 1.0).abs() < 1e-12);
        assert!(c.distance(Vec2::new(-1.0, 0.0)) < 1e-12);
        let (c2, r2) = bulge_arc(Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0), (PI / 8.0).tan());
        assert!((r2 - 1.0).abs() < 1e-9);
        assert!(c2.length() < 1e-9);
    }
}
