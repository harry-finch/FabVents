//! Contours fermés exacts : cercle, ou suite de sommets dont chaque segment
//! vers le suivant est droit ou en arc (« bulge » DXF, `tan(angle / 4)`).
//!
//! Un trou est décrit par une **âme** (point, segment ou polygone convexe) et
//! un **arrondi** : son contour est la somme de Minkowski de l'âme et d'un
//! disque. Cercle = point + rayon, fente = segment + demi-largeur, rectangle
//! à coins arrondis = rectangle réduit + rayon de coin. Les arcs restent
//! exacts jusqu'aux exports (cercle et bulge en DXF, arc `A` en SVG).

use crate::geom::{arc_segments, bulge_angle, bulge_arc, signed_area, Vec2};
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub p: Vec2,
    /// Arc vers le sommet suivant : `tan(angle / 4)`, 0 pour un segment droit.
    pub bulge: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Contour {
    Circle {
        center: Vec2,
        radius: f64,
    },
    /// Contour fermé, sens trigonométrique.
    Path(Vec<Vertex>),
}

impl Contour {
    /// Contour d'une âme arrondie. L'âme doit être convexe dès qu'elle a
    /// trois sommets ou plus et que l'arrondi n'est pas nul (une étoile n'a
    /// jamais d'arrondi).
    pub fn rounded(core: &[Vec2], round: f64) -> Contour {
        match core.len() {
            0 => Contour::Path(Vec::new()),
            1 => Contour::Circle {
                center: core[0],
                radius: round,
            },
            _ if round <= 0.0 => {
                let mut pts = core.to_vec();
                if signed_area(&pts) < 0.0 {
                    pts.reverse();
                }
                Contour::Path(pts.into_iter().map(|p| Vertex { p, bulge: 0.0 }).collect())
            }
            n => {
                let mut pts = core.to_vec();
                if n >= 3 && signed_area(&pts) < 0.0 {
                    pts.reverse();
                }
                let mut out = Vec::with_capacity(2 * n);
                for i in 0..n {
                    let prev = pts[(i + n - 1) % n];
                    let cur = pts[i];
                    let next = pts[(i + 1) % n];
                    // Normales extérieures des arêtes entrante et sortante.
                    let n_in = (cur - prev).normalized().right();
                    let n_out = (next - cur).normalized().right();
                    let mut sweep = n_in.cross(n_out).atan2(n_in.dot(n_out));
                    if sweep < 0.0 {
                        sweep += TAU;
                    }
                    if sweep > 1e-9 {
                        out.push(Vertex {
                            p: cur + n_in * round,
                            bulge: (sweep / 4.0).tan(),
                        });
                    }
                    out.push(Vertex {
                        p: cur + n_out * round,
                        bulge: 0.0,
                    });
                }
                Contour::Path(out)
            }
        }
    }

    /// Aire exacte (arcs compris).
    pub fn area(&self) -> f64 {
        match self {
            Contour::Circle { radius, .. } => std::f64::consts::PI * radius * radius,
            Contour::Path(v) => {
                let n = v.len();
                if n < 2 {
                    return 0.0;
                }
                let pts: Vec<Vec2> = v.iter().map(|x| x.p).collect();
                let mut a = signed_area(&pts);
                for i in 0..n {
                    let b = v[i].bulge;
                    if b != 0.0 {
                        let (_, r) = bulge_arc(v[i].p, v[(i + 1) % n].p, b);
                        let t = bulge_angle(b);
                        a += 0.5 * r * r * (t - t.sin());
                    }
                }
                a.abs()
            }
        }
    }

    /// Polygone approché : flèche maximale `tol` (mm) sur les arcs.
    pub fn tessellate(&self, tol: f64, out: &mut Vec<Vec2>) {
        match *self {
            Contour::Circle { center, radius } => {
                let n = arc_segments(radius, TAU, tol, 256).max(12);
                for k in 0..n {
                    out.push(center + Vec2::from_angle(TAU * k as f64 / n as f64) * radius);
                }
            }
            Contour::Path(ref v) => {
                let n = v.len();
                for i in 0..n {
                    let a = v[i];
                    out.push(a.p);
                    if a.bulge != 0.0 {
                        let b = v[(i + 1) % n].p;
                        let (c, r) = bulge_arc(a.p, b, a.bulge);
                        let theta = bulge_angle(a.bulge);
                        let segs = arc_segments(r, theta, tol, 48);
                        let start = (a.p - c).angle();
                        for k in 1..segs {
                            out.push(
                                c + Vec2::from_angle(start + theta * k as f64 / segs as f64) * r,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn tessellated(&self, tol: f64) -> Vec<Vec2> {
        let mut v = Vec::new();
        self.tessellate(tol, &mut v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn stadium_area_is_exact() {
        // Fente : longueur 10, largeur 4 → (10 - 4) × 4 + π × 2².
        let c = Contour::rounded(&[Vec2::new(-3.0, 0.0), Vec2::new(3.0, 0.0)], 2.0);
        assert!((c.area() - (24.0 + PI * 4.0)).abs() < 1e-9);
    }

    #[test]
    fn rounded_rectangle_area_is_exact() {
        // 20 × 10, coins de 2 : 200 - (4 - π) × 4.
        let core = [
            Vec2::new(-8.0, -3.0),
            Vec2::new(8.0, -3.0),
            Vec2::new(8.0, 3.0),
            Vec2::new(-8.0, 3.0),
        ];
        let c = Contour::rounded(&core, 2.0);
        assert!((c.area() - (200.0 - (4.0 - PI) * 4.0)).abs() < 1e-9);
        let poly = c.tessellated(0.001);
        assert!((signed_area(&poly) - c.area()).abs() < 0.05);
    }

    #[test]
    fn clockwise_core_is_reoriented() {
        let core = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ];
        let poly = Contour::rounded(&core, 0.0).tessellated(0.01);
        assert!(signed_area(&poly) > 0.0);
    }
}
