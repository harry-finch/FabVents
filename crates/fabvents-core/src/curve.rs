//! Tracés : aplatissement d'une polyligne à congés, distance et côté,
//! subdivision, décalage parallèle, placement de points à espacement variable.

use crate::geom::{arc_segments, Vec2};
use crate::project::Curve;
use std::f64::consts::PI;

/// Polyligne d'un tracé, congés facettisés à la flèche `tol` (mm). Le rayon
/// d'un congé est réduit si les arêtes voisines sont trop courtes (au plus
/// la moitié de chacune).
pub fn flatten(curve: &Curve, tol: f64) -> Vec<Vec2> {
    let p = &curve.points;
    let n = p.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let cur = p[i];
        let r = curve.fillet(i);
        let interior = curve.closed || (i > 0 && i + 1 < n);
        if r <= 0.0 || !interior || n < 3 {
            out.push(cur);
            continue;
        }
        let prev = p[(i + n - 1) % n];
        let next = p[(i + 1) % n];
        let (lp, ln) = (prev.distance(cur), next.distance(cur));
        if lp < 1e-9 || ln < 1e-9 {
            out.push(cur);
            continue;
        }
        let u_in = (prev - cur) * (1.0 / lp);
        let u_out = (next - cur) * (1.0 / ln);
        let alpha = u_in.dot(u_out).clamp(-1.0, 1.0).acos();
        if !(1e-6..=PI - 1e-6).contains(&alpha) {
            out.push(cur);
            continue;
        }
        let tan_half = (alpha / 2.0).tan();
        let setback = (r / tan_half).min(0.5 * lp.min(ln));
        let radius = setback * tan_half;
        let t_in = cur + u_in * setback;
        let t_out = cur + u_out * setback;
        let center = cur + (u_in + u_out).normalized() * (radius / (alpha / 2.0).sin());
        let a0 = (t_in - center).angle();
        let mut sweep = (t_out - center).angle() - a0;
        while sweep > PI {
            sweep -= 2.0 * PI;
        }
        while sweep < -PI {
            sweep += 2.0 * PI;
        }
        let segs = arc_segments(radius, sweep, tol, 32);
        out.push(t_in);
        for k in 1..segs {
            out.push(center + Vec2::from_angle(a0 + sweep * k as f64 / segs as f64) * radius);
        }
        out.push(t_out);
    }
    out
}

/// Ajoute des sommets pour qu'aucune arête ne dépasse `step` (mm).
pub fn densify(pts: &[Vec2], closed: bool, step: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n < 2 || step <= 0.0 {
        return pts.to_vec();
    }
    let edges = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(n);
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let k = ((a.distance(b) / step).ceil() as usize).max(1);
        for j in 0..k {
            out.push(a.lerp(b, j as f64 / k as f64));
        }
    }
    if !closed {
        out.push(pts[n - 1]);
    }
    out
}

/// Décalage parallèle approché (sommet par sommet, le long de la normale à
/// gauche) : rangées parallèles d'une disposition Chemin.
pub fn offset(pts: &[Vec2], closed: bool, d: f64) -> Vec<Vec2> {
    let n = pts.len();
    if n < 2 || d == 0.0 {
        return pts.to_vec();
    }
    (0..n)
        .map(|i| {
            let prev = if i > 0 {
                pts[i - 1]
            } else if closed {
                pts[n - 1]
            } else {
                pts[0]
            };
            let next = if i + 1 < n {
                pts[i + 1]
            } else if closed {
                pts[0]
            } else {
                pts[n - 1]
            };
            pts[i] + (next - prev).normalized().left() * d
        })
        .collect()
}

/// Segments d'un tracé aplati, avec prolongement optionnel des extrémités
/// (premier segment vers l'arrière, dernier vers l'avant).
pub struct Polyline {
    segs: Vec<(Vec2, Vec2)>,
    extend: bool,
    pub closed: bool,
    pub pts: Vec<Vec2>,
}

impl Polyline {
    pub fn new(pts: Vec<Vec2>, closed: bool, extend: bool) -> Self {
        let n = pts.len();
        let mut segs = Vec::new();
        if n == 1 {
            segs.push((pts[0], pts[0]));
        }
        for i in 0..n.saturating_sub(1) {
            segs.push((pts[i], pts[i + 1]));
        }
        if closed && n > 2 {
            segs.push((pts[n - 1], pts[0]));
        }
        Self {
            segs,
            extend: extend && !closed,
            closed,
            pts,
        }
    }

    /// Distance au tracé et côté (+1 à gauche dans le sens du tracé, -1 à droite).
    pub fn probe(&self, p: Vec2) -> (f64, f64) {
        let last = self.segs.len().saturating_sub(1);
        let mut best = (f64::INFINITY, 1.0);
        for (k, &(a, b)) in self.segs.iter().enumerate() {
            let ab = b - a;
            let l2 = ab.length_sq();
            let mut t = if l2 > 1e-24 {
                (p - a).dot(ab) / l2
            } else {
                0.0
            };
            let lo = if self.extend && k == 0 {
                f64::NEG_INFINITY
            } else {
                0.0
            };
            let hi = if self.extend && k == last {
                f64::INFINITY
            } else {
                1.0
            };
            t = t.clamp(lo, hi);
            let d = p.distance(a + ab * t);
            if d < best.0 {
                let side = if ab.cross(p - a) >= 0.0 { 1.0 } else { -1.0 };
                best = (d, side);
            }
        }
        best
    }

    pub fn contains(&self, p: Vec2) -> bool {
        self.closed && crate::geom::point_in_polygon(p, &self.pts)
    }
}

/// Un point posé le long d'un tracé et la direction du tracé (degrés).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub pos: Vec2,
    pub angle: f64,
}

/// Pose des points le long d'une polyligne dense, à l'espacement local
/// `spacing(p)` (mm). Tracé fermé : points répartis régulièrement, le premier
/// décalé de `phase` (fraction d'intervalle). Tracé ouvert : points centrés
/// sur la longueur. `Err(n)` si plus de `max` points.
pub fn place_along(
    pts: &[Vec2],
    closed: bool,
    spacing: &dyn Fn(Vec2) -> f64,
    phase: f64,
    max: usize,
) -> Result<Vec<Placed>, usize> {
    let n = pts.len();
    if n < 2 {
        return Ok(pts.iter().map(|&pos| Placed { pos, angle: 0.0 }).collect());
    }
    let edges = if closed { n } else { n - 1 };
    let mut du = Vec::with_capacity(edges);
    let mut total = 0.0;
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let s = spacing(a.lerp(b, 0.5)).max(1e-6);
        let u = a.distance(b) / s;
        du.push(u);
        total += u;
    }
    let targets: Vec<f64> = if closed {
        let count = (total.round() as usize).max(1);
        if count > max {
            return Err(count);
        }
        let step = total / count as f64;
        (0..count)
            .map(|k| ((k as f64 + phase.rem_euclid(1.0)) * step).min(total))
            .collect()
    } else {
        // Marge d'arrondi : une longueur de 8 espacements donne bien 9 points.
        let whole = (total + 1e-9).floor();
        let count = whole as usize + 1;
        if count > max {
            return Err(count);
        }
        let start = ((total - whole) / 2.0).max(0.0);
        (0..count).map(|k| start + k as f64).collect()
    };

    let mut out = Vec::with_capacity(targets.len());
    let (mut seg, mut acc) = (0usize, 0.0);
    for u in targets {
        while seg + 1 < edges && acc + du[seg] < u {
            acc += du[seg];
            seg += 1;
        }
        let (a, b) = (pts[seg], pts[(seg + 1) % n]);
        let t = if du[seg] > 0.0 {
            ((u - acc) / du[seg]).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(Placed {
            pos: a.lerp(b, t),
            angle: (b - a).angle().to_degrees(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_curve(r: f64) -> Curve {
        Curve {
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            closed: true,
            fillets: vec![r; 4],
        }
    }

    #[test]
    fn fillets_round_the_corners() {
        let pts = flatten(&square_curve(2.0), 0.01);
        assert!(pts.len() > 16);
        // Le coin (10, 0) est remplacé par un arc de rayon 2 : le point le plus
        // proche du coin est à (√2 - 1) × 2 de l'ancien sommet... vu de l'intérieur.
        let corner = Vec2::new(10.0, 0.0);
        let dmin = pts
            .iter()
            .map(|p| p.distance(corner))
            .fold(f64::INFINITY, f64::min);
        assert!((dmin - (2f64.sqrt() - 1.0) * 2.0).abs() < 0.02, "{dmin}");
        assert_eq!(flatten(&square_curve(0.0), 0.01).len(), 4);
    }

    #[test]
    fn oversized_fillet_is_limited_by_edges() {
        let pts = flatten(&square_curve(50.0), 0.01);
        // Setback limité à la moitié des arêtes : tous les points restent dans le carré.
        assert!(pts
            .iter()
            .all(|p| p.x >= -1e-9 && p.x <= 10.0 + 1e-9 && p.y >= -1e-9 && p.y <= 10.0 + 1e-9));
    }

    #[test]
    fn distance_side_and_extension() {
        let pl = Polyline::new(
            vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)],
            false,
            false,
        );
        assert_eq!(pl.probe(Vec2::new(5.0, 3.0)), (3.0, 1.0));
        assert_eq!(pl.probe(Vec2::new(5.0, -3.0)), (3.0, -1.0));
        assert!((pl.probe(Vec2::new(20.0, 3.0)).0 - 109f64.sqrt()).abs() < 1e-9);
        let ext = Polyline::new(vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)], false, true);
        assert_eq!(ext.probe(Vec2::new(20.0, 3.0)).0, 3.0);
        assert_eq!(ext.probe(Vec2::new(-20.0, -3.0)).0, 3.0);
    }

    #[test]
    fn even_spacing_along_a_ring() {
        let ring: Vec<Vec2> = (0..360)
            .map(|k| Vec2::from_angle(k as f64 * PI / 180.0) * 10.0)
            .collect();
        let pts = place_along(&ring, true, &|_| 2.0 * PI, 0.0, 100).unwrap();
        // Circonférence 20π, espacement 2π → 10 points.
        assert_eq!(pts.len(), 10);
        let d = pts[0].pos.distance(pts[1].pos);
        assert!(pts
            .windows(2)
            .all(|w| (w[0].pos.distance(w[1].pos) - d).abs() < 1e-3));
    }

    #[test]
    fn variable_spacing_is_followed() {
        let line = densify(&[Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)], false, 0.5);
        let pts = place_along(
            &line,
            false,
            &|p| if p.x < 50.0 { 2.0 } else { 10.0 },
            0.0,
            1000,
        )
        .unwrap();
        let left = pts.iter().filter(|q| q.pos.x < 50.0).count();
        let right = pts.len() - left;
        assert!(left >= 24 && right <= 6, "{left} / {right}");
        assert!(place_along(&line, false, &|_| 0.01, 0.0, 1000).is_err());
    }

    #[test]
    fn offset_moves_left() {
        let o = offset(&[Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)], false, 2.0);
        assert_eq!(o, vec![Vec2::new(0.0, 2.0), Vec2::new(10.0, 2.0)]);
    }
}
