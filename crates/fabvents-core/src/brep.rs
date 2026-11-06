//! Boucles exactes d'un solide : suite de segments droits et d'arcs, sans
//! facettisation. Sert à l'export STEP : orientation (matière à gauche),
//! décalage du fond pour la dépouille, découpe par une droite pour les
//! dalles.
//!
//! Convention : une boucle est parcourue **matière à gauche** ; le contour
//! extérieur d'une pièce tourne donc dans le sens trigonométrique, un trou
//! dans le sens horaire.

use crate::contour::{Contour, Vertex};
use crate::geom::{bulge_arc, point_in_polygon, signed_area, Vec2};
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

/// Tolérance de position (mm).
const EPS: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Line,
    /// Arc de centre `c` et de rayon `r`, sens trigonométrique si `ccw`.
    Arc {
        c: Vec2,
        r: f64,
        ccw: bool,
    },
}

/// Boucle fermée : le segment `i` va de `pts[i]` à `pts[i + 1]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    pub pts: Vec<Vec2>,
    pub segs: Vec<Seg>,
}

/// Axe d'une droite de coupe : `X` coupe à x constant, `Y` à y constant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    fn along(self, p: Vec2) -> f64 {
        match self {
            Axis::X => p.x,
            Axis::Y => p.y,
        }
    }

    fn across(self, p: Vec2) -> f64 {
        match self {
            Axis::X => p.y,
            Axis::Y => p.x,
        }
    }

    fn point(self, at: f64, across: f64) -> Vec2 {
        match self {
            Axis::X => Vec2::new(at, across),
            Axis::Y => Vec2::new(across, at),
        }
    }
}

/// Angle parcouru (positif, dans `(0, 2π]`) de `a` à `b` autour de `c`.
pub fn arc_sweep(c: Vec2, a: Vec2, b: Vec2, ccw: bool) -> f64 {
    let d = (b - c).angle() - (a - c).angle();
    let s = if ccw { d } else { -d }.rem_euclid(TAU);
    if s < 1e-12 {
        TAU
    } else {
        s
    }
}

impl Loop {
    /// Boucle d'un contour exact (cercle coupé en deux demi-arcs).
    pub fn from_contour(c: &Contour) -> Loop {
        match *c {
            Contour::Circle { center, radius } => Loop {
                pts: vec![
                    center + Vec2::new(radius, 0.0),
                    center - Vec2::new(radius, 0.0),
                ],
                segs: vec![
                    Seg::Arc {
                        c: center,
                        r: radius,
                        ccw: true,
                    };
                    2
                ],
            },
            Contour::Path(ref v) => {
                let mut pts = Vec::with_capacity(v.len());
                let mut segs = Vec::with_capacity(v.len());
                let n = v.len();
                for i in 0..n {
                    let a = v[i];
                    let b = v[(i + 1) % n];
                    if a.p.distance(b.p) < EPS {
                        continue;
                    }
                    pts.push(a.p);
                    segs.push(if a.bulge.abs() < 1e-12 {
                        Seg::Line
                    } else {
                        let (c, r) = bulge_arc(a.p, b.p, a.bulge);
                        Seg::Arc {
                            c,
                            r,
                            ccw: a.bulge > 0.0,
                        }
                    });
                }
                Loop { pts, segs }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.pts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pts.is_empty()
    }

    fn end(&self, i: usize) -> Vec2 {
        self.pts[(i + 1) % self.pts.len()]
    }

    /// Angle parcouru par l'arc `i`, signé (positif dans le sens trigonométrique).
    pub fn sweep(&self, i: usize) -> f64 {
        match self.segs[i] {
            Seg::Line => 0.0,
            Seg::Arc { c, ccw, .. } => {
                let s = arc_sweep(c, self.pts[i], self.end(i), ccw);
                if ccw {
                    s
                } else {
                    -s
                }
            }
        }
    }

    /// Aire signée exacte (positive dans le sens trigonométrique).
    pub fn signed_area(&self) -> f64 {
        let mut a = signed_area(&self.pts);
        for i in 0..self.len() {
            if let Seg::Arc { r, .. } = self.segs[i] {
                let t = self.sweep(i);
                a += 0.5 * r * r * (t - t.sin());
            }
        }
        a
    }

    /// Même boucle parcourue dans l'autre sens.
    pub fn reversed(&self) -> Loop {
        let n = self.len();
        let pts = (0..n).map(|k| self.pts[(n - k) % n]).collect();
        let segs = (0..n)
            .map(|k| match self.segs[(2 * n - k - 1) % n] {
                Seg::Line => Seg::Line,
                Seg::Arc { c, r, ccw } => Seg::Arc { c, r, ccw: !ccw },
            })
            .collect();
        Loop { pts, segs }
    }

    /// Boucle orientée dans le sens trigonométrique (`ccw`) ou horaire.
    pub fn oriented(self, ccw: bool) -> Loop {
        if (self.signed_area() > 0.0) == ccw {
            self
        } else {
            self.reversed()
        }
    }

    /// Direction au départ du segment `i`.
    fn tangent_out(&self, i: usize) -> Vec2 {
        tangent(self.segs[i], self.pts[i], self.pts[i], self.end(i))
    }

    /// Direction à l'arrivée du segment `i`.
    fn tangent_in(&self, i: usize) -> Vec2 {
        tangent(self.segs[i], self.end(i), self.pts[i], self.end(i))
    }

    /// Contour exact équivalent (sommets et bulges).
    pub fn to_contour(&self) -> Contour {
        Contour::Path(
            (0..self.len())
                .map(|i| Vertex {
                    p: self.pts[i],
                    bulge: (self.sweep(i) / 4.0).tan(),
                })
                .collect(),
        )
    }

    pub fn tessellated(&self, tol: f64) -> Vec<Vec2> {
        self.to_contour().tessellated(tol)
    }

    /// Boîte englobante exacte (extrémités des arcs comprises).
    pub fn bbox(&self) -> (Vec2, Vec2) {
        let mut lo = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let mut add = |p: Vec2| {
            lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
        };
        for i in 0..self.len() {
            add(self.pts[i]);
            if let Seg::Arc { c, r, ccw } = self.segs[i] {
                let a0 = (self.pts[i] - c).angle();
                let s = arc_sweep(c, self.pts[i], self.end(i), ccw);
                for k in 0..4 {
                    let q = k as f64 * FRAC_PI_2;
                    let t = if ccw { q - a0 } else { a0 - q }.rem_euclid(TAU);
                    if t < s {
                        add(c + Vec2::from_angle(q) * r);
                    }
                }
            }
        }
        (lo, hi)
    }

    /// Le point est-il à l'intérieur (facettisation fine) ?
    pub fn contains(&self, p: Vec2) -> bool {
        point_in_polygon(p, &self.tessellated(0.01))
    }

    /// Boucle décalée de `d` mm vers la gauche (côté matière) : le fond d'une
    /// paroi en dépouille. `None` si la boucle se referme : arc dont le rayon
    /// s'annule, segment qui s'inverse.
    pub fn offset_left(&self, d: f64) -> Option<Loop> {
        if d == 0.0 {
            return Some(self.clone());
        }
        let n = self.len();
        let shifted: Vec<Shape> = (0..n)
            .map(|i| match self.segs[i] {
                Seg::Line => {
                    let dir = (self.end(i) - self.pts[i]).normalized();
                    Some(Shape::Line {
                        p: self.pts[i] + dir.left() * d,
                        dir,
                    })
                }
                // Centre à gauche (arc trigonométrique) : le rayon diminue.
                Seg::Arc { c, r, ccw } => {
                    let r2 = if ccw { r - d } else { r + d };
                    (r2 > EPS).then_some(Shape::Circle { c, r: r2 })
                }
            })
            .collect::<Option<_>>()?;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let prev = (i + n - 1) % n;
            let t_in = self.tangent_in(prev);
            let t_out = self.tangent_out(i);
            let p = self.pts[i];
            let q = if t_in.cross(t_out).abs() < 1e-9 && t_in.dot(t_out) > 0.0 {
                p + t_out.left() * d
            } else {
                intersect(shifted[prev], shifted[i])
                    .into_iter()
                    .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))?
            };
            pts.push(q);
        }
        let mut segs = Vec::with_capacity(n);
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            match (self.segs[i], shifted[i]) {
                (Seg::Line, _) => {
                    let orig = self.end(i) - self.pts[i];
                    if (b - a).dot(orig) <= EPS * orig.length() {
                        return None;
                    }
                    segs.push(Seg::Line);
                }
                (Seg::Arc { c, ccw, .. }, Shape::Circle { r, .. }) => {
                    let before = arc_sweep(c, self.pts[i], self.end(i), ccw);
                    let after = arc_sweep(c, a, b, ccw);
                    if a.distance(b) < EPS || (after - before).abs() > PI {
                        return None;
                    }
                    segs.push(Seg::Arc { c, r, ccw });
                }
                _ => unreachable!(),
            }
        }
        Some(Loop { pts, segs })
    }

    /// Plus grand déplacement d'un sommet entre la boucle et `other` (même
    /// nombre de sommets) : croissance effective d'une dépouille.
    pub fn max_shift(&self, other: &Loop) -> f64 {
        self.pts
            .iter()
            .zip(&other.pts)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0, f64::max)
    }

    /// Points de passage de la boucle sur la droite `axis = at`, la boucle
    /// coupée en ces points (chaque segment d'un seul côté).
    fn refine(&self, axis: Axis, at: f64) -> (Loop, Vec<bool>) {
        let mut out = Loop {
            pts: Vec::new(),
            segs: Vec::new(),
        };
        let mut crossing = Vec::new();
        for i in 0..self.len() {
            let (a, b) = (self.pts[i], self.end(i));
            out.pts.push(a);
            out.segs.push(self.segs[i]);
            crossing.push(false);
            let mut cuts: Vec<(f64, Vec2)> = Vec::new();
            match self.segs[i] {
                Seg::Line => {
                    let (fa, fb) = (axis.along(a) - at, axis.along(b) - at);
                    if fa * fb < 0.0 {
                        let p = a.lerp(b, fa / (fa - fb));
                        cuts.push((0.0, axis.point(at, axis.across(p))));
                    }
                }
                Seg::Arc { c, r, ccw } => {
                    let dx = at - axis.along(c);
                    if dx.abs() < r {
                        let h = (r * r - dx * dx).sqrt();
                        let s = arc_sweep(c, a, b, ccw);
                        for sign in [-1.0, 1.0] {
                            let p = axis.point(at, axis.across(c) + sign * h);
                            let t = arc_sweep(c, a, p, ccw);
                            if t > 1e-9 && t < s - 1e-9 {
                                cuts.push((t, p));
                            }
                        }
                        cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
                    }
                }
            }
            for (_, p) in cuts {
                out.pts.push(p);
                out.segs.push(self.segs[i]);
                crossing.push(true);
            }
        }
        (out, crossing)
    }

    /// Le segment `k` est-il du côté inférieur de la droite ?
    fn below(&self, k: usize, axis: Axis, at: f64) -> bool {
        let (a, b) = (self.pts[k], self.end(k));
        let mid = match self.segs[k] {
            Seg::Line => a.lerp(b, 0.5),
            Seg::Arc { c, r, ccw } => {
                let s = arc_sweep(c, a, b, ccw);
                let a0 = (a - c).angle();
                c + Vec2::from_angle(if ccw { a0 + s / 2.0 } else { a0 - s / 2.0 }) * r
            }
        };
        axis.along(mid) < at
    }
}

/// Valeur de coupe sûre près de `at` : aucun sommet ni tangence d'arc des
/// boucles à moins de 1e-6 mm de la droite.
pub fn safe_cut(loops: &[Loop], axis: Axis, at: f64) -> f64 {
    let mut c = at;
    for _ in 0..16 {
        let bad = loops.iter().any(|l| {
            (0..l.len()).any(|i| {
                (axis.along(l.pts[i]) - c).abs() < 1e-6
                    || matches!(l.segs[i], Seg::Arc { c: o, r, .. } if ((axis.along(o) - c).abs() - r).abs() < 1e-6)
            })
        });
        if !bad {
            break;
        }
        c += 1e-5;
    }
    c
}

/// Coupe d'une pièce (boucles orientées matière à gauche : contours dans
/// le sens trigonométrique, trous dans le sens horaire) par la droite
/// `axis = at`. Renvoie les boucles du côté inférieur, puis du côté
/// supérieur, chacune avec son étiquette : une boucle que la droite ne
/// traverse pas garde la sienne, une boucle refermée le long de la droite
/// n'en a pas (`None`) et tourne toujours dans le sens trigonométrique.
/// La droite ne doit passer par aucun sommet (voir [`safe_cut`]).
pub fn split<T: Copy>(loops: &[(Loop, Option<T>)], axis: Axis, at: f64) -> SplitResult<T> {
    let mut lower = Vec::new();
    let mut upper = Vec::new();
    // Boucles traversées, découpées aux points de passage.
    let mut cut: Vec<(Loop, Vec<bool>)> = Vec::new();
    for (l, tag) in loops {
        let (r, crossing) = l.refine(axis, at);
        if crossing.iter().any(|c| *c) {
            cut.push((r, crossing));
        } else if l.below(0, axis, at) {
            lower.push((l.clone(), *tag));
        } else {
            upper.push((l.clone(), *tag));
        }
    }
    // Points de passage triés le long de la droite : deux points consécutifs
    // bornent un morceau de droite dans la matière.
    let mut nodes: Vec<(usize, usize)> = Vec::new();
    for (li, (_, crossing)) in cut.iter().enumerate() {
        nodes.extend(
            crossing
                .iter()
                .enumerate()
                .filter(|(_, c)| **c)
                .map(|(k, _)| (li, k)),
        );
    }
    nodes.sort_by(|x, y| {
        axis.across(cut[x.0].0.pts[x.1])
            .total_cmp(&axis.across(cut[y.0].0.pts[y.1]))
    });
    let mut partner: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
    for pair in nodes.chunks(2) {
        if let [x, y] = *pair {
            partner.insert(x, y);
            partner.insert(y, x);
        }
    }
    for side_lower in [true, false] {
        let mut used: HashMap<(usize, usize), bool> = HashMap::new();
        for &start in &nodes {
            let (l0, _) = &cut[start.0];
            if used.contains_key(&start) || l0.below(start.1, axis, at) != side_lower {
                continue;
            }
            let mut out = Loop {
                pts: Vec::new(),
                segs: Vec::new(),
            };
            let mut node = start;
            loop {
                used.insert(node, true);
                // Chaîne le long de la boucle jusqu'au prochain point de passage.
                let (l, crossing) = &cut[node.0];
                let mut j = node.1;
                loop {
                    out.pts.push(l.pts[j]);
                    out.segs.push(l.segs[j]);
                    j = (j + 1) % l.len();
                    if crossing[j] {
                        break;
                    }
                }
                // Retour le long de la droite vers le point associé.
                out.pts.push(l.pts[j]);
                out.segs.push(Seg::Line);
                match partner.get(&(node.0, j)) {
                    Some(&next) if next != start && !used.contains_key(&next) => node = next,
                    _ => break,
                }
            }
            if out.len() >= 2 && out.signed_area().abs() > EPS {
                if side_lower {
                    lower.push((out, None));
                } else {
                    upper.push((out, None));
                }
            }
        }
    }
    (lower, upper)
}

/// Boucles de part et d'autre d'une coupe.
pub type SplitResult<T> = (Vec<(Loop, Option<T>)>, Vec<(Loop, Option<T>)>);
fn tangent(seg: Seg, at: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    match seg {
        Seg::Line => (b - a).normalized(),
        Seg::Arc { c, ccw, .. } => {
            let u = (at - c).normalized();
            if ccw {
                u.left()
            } else {
                u.right()
            }
        }
    }
}

/// Support d'un segment décalé : droite (point, direction) ou cercle.
#[derive(Clone, Copy)]
enum Shape {
    Line { p: Vec2, dir: Vec2 },
    Circle { c: Vec2, r: f64 },
}

fn intersect(a: Shape, b: Shape) -> Vec<Vec2> {
    match (a, b) {
        (Shape::Line { p, dir }, Shape::Line { p: q, dir: e }) => {
            let den = dir.cross(e);
            if den.abs() < 1e-12 {
                return Vec::new();
            }
            let t = (q - p).cross(e) / den;
            vec![p + dir * t]
        }
        (Shape::Line { p, dir }, Shape::Circle { c, r })
        | (Shape::Circle { c, r }, Shape::Line { p, dir }) => {
            let f = p - c;
            let bq = f.dot(dir);
            let disc = bq * bq - (f.length_sq() - r * r);
            if disc < 0.0 {
                return Vec::new();
            }
            let s = disc.sqrt();
            vec![p + dir * (-bq - s), p + dir * (-bq + s)]
        }
        (Shape::Circle { c, r }, Shape::Circle { c: c2, r: r2 }) => {
            let d = c.distance(c2);
            if d < 1e-12 || d > r + r2 || d < (r - r2).abs() {
                return Vec::new();
            }
            let a = (r * r - r2 * r2 + d * d) / (2.0 * d);
            let h = (r * r - a * a).max(0.0).sqrt();
            let u = (c2 - c) * (1.0 / d);
            let m = c + u * a;
            vec![m + u.left() * h, m - u.left() * h]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rounded_square() -> Loop {
        let core = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        Loop::from_contour(&Contour::rounded(&core, 2.0))
    }

    #[test]
    fn exact_area_and_orientation() {
        let c = Loop::from_contour(&Contour::Circle {
            center: Vec2::ZERO,
            radius: 3.0,
        });
        assert!((c.signed_area() - PI * 9.0).abs() < 1e-9);
        let r = c.reversed();
        assert!((r.signed_area() + PI * 9.0).abs() < 1e-9);
        let s = rounded_square();
        let expected = 14.0 * 14.0 - (4.0 - PI) * 4.0;
        assert!((s.signed_area() - expected).abs() < 1e-9);
        assert!((s.reversed().signed_area() + expected).abs() < 1e-9);
    }

    #[test]
    fn bbox_includes_arc_extremes() {
        let c = Loop::from_contour(&Contour::Circle {
            center: Vec2::new(5.0, 5.0),
            radius: 2.0,
        });
        let (lo, hi) = c.bbox();
        assert!((lo.x - 3.0).abs() < 1e-9 && (lo.y - 3.0).abs() < 1e-9);
        assert!((hi.x - 7.0).abs() < 1e-9 && (hi.y - 7.0).abs() < 1e-9);
    }

    #[test]
    fn hole_offset_grows_and_shrinks() {
        // Trou : matière à gauche, donc sens horaire.
        let h = Loop::from_contour(&Contour::Circle {
            center: Vec2::ZERO,
            radius: 3.0,
        })
        .oriented(false);
        let wide = h.offset_left(0.5).unwrap();
        assert!((wide.signed_area() + PI * 3.5 * 3.5).abs() < 1e-9);
        let narrow = h.offset_left(-1.0).unwrap();
        assert!((narrow.signed_area() + PI * 4.0).abs() < 1e-9);
        assert!(h.offset_left(-3.0).is_none());
        // Carré arrondi : arcs de rayon 2 → 2,5, côtés décalés de 0,5.
        let s = rounded_square().oriented(false).offset_left(0.5).unwrap();
        let expected = 15.0 * 15.0 - (4.0 - PI) * 2.5 * 2.5;
        assert!((s.signed_area() + expected).abs() < 1e-9);
        assert!(
            (rounded_square().max_shift(&rounded_square().offset_left(0.5).unwrap()) - 0.5).abs()
                < 1e-9
        );
    }

    #[test]
    fn sharp_polygon_offset_uses_miters() {
        let sq = Loop::from_contour(&Contour::rounded(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(4.0, 0.0),
                Vec2::new(4.0, 4.0),
                Vec2::new(0.0, 4.0),
            ],
            0.0,
        ))
        .oriented(false);
        let o = sq.offset_left(1.0).unwrap();
        assert!((o.signed_area() + 36.0).abs() < 1e-9);
        assert!((sq.max_shift(&o) - 2f64.sqrt()).abs() < 1e-9);
        assert!(sq.offset_left(-2.5).is_none());
    }

    #[test]
    fn split_a_disc_in_two_halves() {
        let c = Loop::from_contour(&Contour::Circle {
            center: Vec2::new(5.0, 5.0),
            radius: 5.0,
        });
        let at = safe_cut(std::slice::from_ref(&c), Axis::X, 5.0);
        let (lo, hi) = split(&[(c, Some(0))], Axis::X, at);
        assert_eq!((lo.len(), hi.len()), (1, 1));
        assert!(lo[0].1.is_none());
        let total = lo[0].0.signed_area() + hi[0].0.signed_area();
        assert!((total - PI * 25.0).abs() < 1e-6);
        assert!((lo[0].0.signed_area() - PI * 12.5).abs() < 1e-3);
    }

    #[test]
    fn split_a_u_shape_in_three_pieces() {
        // « U » : la droite y = 7 coupe les deux branches.
        let u = Loop {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(9.0, 0.0),
                Vec2::new(9.0, 10.0),
                Vec2::new(6.0, 10.0),
                Vec2::new(6.0, 3.0),
                Vec2::new(3.0, 3.0),
                Vec2::new(3.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            segs: vec![Seg::Line; 8],
        };
        let (lo, hi) = split(&[(u.clone(), Some(0))], Axis::Y, 7.0);
        assert_eq!(lo.len(), 1);
        assert_eq!(hi.len(), 2);
        let a: f64 = lo.iter().chain(&hi).map(|l| l.0.signed_area()).sum();
        assert!((a - u.signed_area()).abs() < 1e-9);
        assert!(hi.iter().all(|l| l.0.signed_area() > 0.0));
    }

    #[test]
    fn a_cut_through_a_hole_leaves_two_notches() {
        // Plaque 20 × 10 percée d'un trou de rayon 2 au centre, coupée en x = 10.
        let plate = Loop {
            pts: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(20.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            segs: vec![Seg::Line; 4],
        };
        let hole = Loop::from_contour(&Contour::Circle {
            center: Vec2::new(10.0, 5.0),
            radius: 2.0,
        })
        .oriented(false);
        let loops = [(plate, Some(0)), (hole, Some(1))];
        let at = safe_cut(&[loops[0].0.clone(), loops[1].0.clone()], Axis::X, 10.0);
        let (lo, hi) = split(&loops, Axis::X, at);
        assert_eq!((lo.len(), hi.len()), (1, 1));
        let half = 100.0 - PI * 2.0;
        assert!(
            (lo[0].0.signed_area() - half).abs() < 1e-3,
            "{}",
            lo[0].0.signed_area()
        );
        assert!((hi[0].0.signed_area() - half).abs() < 1e-3);
        // Un trou loin de la coupe garde son étiquette.
        let far = Loop::from_contour(&Contour::Circle {
            center: Vec2::new(3.0, 5.0),
            radius: 1.0,
        })
        .oriented(false);
        let (lo, _) = split(
            &[(loops[0].0.clone(), Some(0)), (far, Some(7))],
            Axis::X,
            10.0,
        );
        assert!(lo.iter().any(|l| l.1 == Some(7)));
    }
}
