//! Voronoï : cellules d'un semis de germes dans une boîte, par découpe de
//! demi-plans (une cellule est convexe), relaxation de Lloyd pondérée par
//! la densité voulue, et réduction exacte d'une cellule (nervure).

use crate::geom::{signed_area, Vec2};
use std::collections::HashMap;

/// Polygone convexe (sens trigonométrique) coupé par le demi-plan
/// `(x - m) · n <= 0`.
fn clip(poly: &[Vec2], m: Vec2, n: Vec2) -> Vec<Vec2> {
    let side = |p: Vec2| (p - m).dot(n);
    let k = poly.len();
    let mut out = Vec::with_capacity(k + 1);
    for i in 0..k {
        let (a, b) = (poly[i], poly[(i + 1) % k]);
        let (fa, fb) = (side(a), side(b));
        if fa <= 0.0 {
            out.push(a);
        }
        if (fa < 0.0 && fb > 0.0) || (fa > 0.0 && fb < 0.0) {
            out.push(a.lerp(b, fa / (fa - fb)));
        }
    }
    out
}

/// Cellules de Voronoï des `points` dans la boîte `[lo, hi]` (même ordre ;
/// cellule vide pour un point hors de la boîte). `cell` : taille de la
/// grille de recherche, de l'ordre de l'espacement.
pub fn cells(points: &[Vec2], lo: Vec2, hi: Vec2, cell: f64) -> Vec<Vec<Vec2>> {
    let cell = cell.max(1e-3);
    let key = |p: Vec2| {
        (
            ((p.x - lo.x) / cell).floor() as i64,
            ((p.y - lo.y) / cell).floor() as i64,
        )
    };
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, &p) in points.iter().enumerate() {
        grid.entry(key(p)).or_default().push(i);
    }
    let boxed = vec![lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)];
    let span = ((hi.x - lo.x).max(hi.y - lo.y) / cell).ceil() as i64 + 1;
    points
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            if p.x < lo.x || p.y < lo.y || p.x > hi.x || p.y > hi.y {
                return Vec::new();
            }
            let mut poly = boxed.clone();
            let (gx, gy) = key(p);
            // Anneaux de cases de plus en plus loin, jusqu'à ce qu'aucun
            // germe plus lointain ne puisse encore couper la cellule.
            for ring in 0..=span {
                let reach = poly.iter().map(|v| v.distance(p)).fold(0.0, f64::max);
                if (ring as f64 - 1.0) * cell > 2.0 * reach {
                    break;
                }
                for dy in -ring..=ring {
                    for dx in -ring..=ring {
                        if dx.abs() != ring && dy.abs() != ring {
                            continue;
                        }
                        for &j in grid.get(&(gx + dx, gy + dy)).map_or(&[][..], |v| v) {
                            if j == i {
                                continue;
                            }
                            let q = points[j];
                            let n = q - p;
                            if n.length_sq() < 1e-18 {
                                continue;
                            }
                            poly = clip(&poly, p.lerp(q, 0.5), n);
                            if poly.len() < 3 {
                                return Vec::new();
                            }
                        }
                    }
                }
            }
            poly
        })
        .collect()
}

/// Centre de gravité d'un polygone convexe, pondéré par `weight` (évalué
/// au centre de chaque triangle de l'éventail).
pub fn weighted_centroid(poly: &[Vec2], weight: &dyn Fn(Vec2) -> f64) -> Vec2 {
    let n = poly.len();
    let c0 = poly.iter().fold(Vec2::ZERO, |s, &v| s + v) * (1.0 / n as f64);
    let (mut acc, mut total) = (Vec2::ZERO, 0.0);
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let area = 0.5 * (a - c0).cross(b - c0);
        let g = (c0 + a + b) * (1.0 / 3.0);
        let w = area * weight(g);
        acc += g * w;
        total += w;
    }
    if total.abs() < 1e-18 {
        c0
    } else {
        acc * (1.0 / total)
    }
}

/// Polygone convexe réduit de `d` mm vers l'intérieur (`None` s'il disparaît).
pub fn inset(poly: &[Vec2], d: f64) -> Option<Vec<Vec2>> {
    let n = poly.len();
    if n < 3 {
        return None;
    }
    let mut out = poly.to_vec();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let e = b - a;
        if e.length() < 1e-12 {
            continue;
        }
        // Normale extérieure (sens trigonométrique) : à droite de l'arête.
        let nrm = e.normalized().right();
        out = clip(&out, a - nrm * d, nrm);
        if out.len() < 3 {
            return None;
        }
    }
    // Sommets confondus retirés.
    out.dedup_by(|a, b| a.distance(*b) < 1e-9);
    while out.len() > 1 && out[0].distance(*out.last().unwrap()) < 1e-9 {
        out.pop();
    }
    (out.len() >= 3 && signed_area(&out) > 1e-12).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_grid_gives_square_cells() {
        let pts: Vec<Vec2> = (0..3)
            .flat_map(|j| {
                (0..3).map(move |i| Vec2::new(5.0 + 10.0 * i as f64, 5.0 + 10.0 * j as f64))
            })
            .collect();
        let c = cells(&pts, Vec2::ZERO, Vec2::new(30.0, 30.0), 10.0);
        for poly in &c {
            assert!((signed_area(poly) - 100.0).abs() < 1e-9, "{poly:?}");
        }
        // Les cellules pavent la boîte.
        let total: f64 = c.iter().map(|p| signed_area(p)).sum();
        assert!((total - 900.0).abs() < 1e-9);
    }

    #[test]
    fn inset_of_a_square() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let i = inset(&sq, 1.0).unwrap();
        assert!((signed_area(&i) - 64.0).abs() < 1e-9);
        assert!(inset(&sq, 5.0).is_none());
    }

    #[test]
    fn weighted_centroid_moves_towards_weight() {
        let sq = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let flat = weighted_centroid(&sq, &|_| 1.0);
        assert!(flat.distance(Vec2::new(5.0, 5.0)) < 1e-9);
        let right = weighted_centroid(&sq, &|p| 1.0 + p.x);
        assert!(right.x > 5.0);
    }
}
