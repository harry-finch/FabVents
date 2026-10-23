//! Formes des perforations : âme (point, segment ou polygone) + arrondi,
//! pour une taille et un angle donnés (voir `contour`).

use crate::geom::{Vec2, DEG};
use crate::project::{Shape, ShapeKind};
use std::f64::consts::{PI, TAU};

/// Âme et arrondi d'un trou, en coordonnées de la plaque.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleGeometry {
    pub core: Vec<Vec2>,
    pub round: f64,
    /// Rayon du cercle englobant, depuis le centre.
    pub bound: f64,
    /// Aire exacte.
    pub area: f64,
}

/// Forme `shape` à la taille de référence `size`, tournée de `angle` degrés,
/// centrée en `center`.
pub fn hole_geometry(shape: &Shape, size: f64, angle: f64, center: Vec2) -> HoleGeometry {
    let rot = angle * DEG;
    let place = |v: Vec2| center + v.rotated(rot);
    match shape.kind {
        ShapeKind::Circle => {
            let r = size / 2.0;
            HoleGeometry {
                core: vec![center],
                round: r,
                bound: r,
                area: PI * r * r,
            }
        }
        ShapeKind::Rectangle { aspect, corner } => {
            let (w, h) = (size, size * aspect);
            let k = if shape.size > 0.0 {
                size / shape.size
            } else {
                1.0
            };
            let rc = (corner * k).clamp(0.0, w.min(h) / 2.0);
            let (hx, hy) = (w / 2.0 - rc, h / 2.0 - rc);
            let core = degenerate(&[
                Vec2::new(-hx, -hy),
                Vec2::new(hx, -hy),
                Vec2::new(hx, hy),
                Vec2::new(-hx, hy),
            ]);
            HoleGeometry {
                core: core.into_iter().map(place).collect(),
                round: rc,
                bound: Vec2::new(hx, hy).length() + rc,
                area: w * h - (4.0 - PI) * rc * rc,
            }
        }
        ShapeKind::Polygon { sides } => {
            let n = sides.max(3) as usize;
            let r = size / 2.0;
            let start = -PI / 2.0 + PI / n as f64;
            let core: Vec<Vec2> = (0..n)
                .map(|i| place(Vec2::from_angle(start + TAU * i as f64 / n as f64) * r))
                .collect();
            HoleGeometry {
                core,
                round: 0.0,
                bound: r,
                area: 0.5 * n as f64 * r * r * (TAU / n as f64).sin(),
            }
        }
        ShapeKind::Star { points, inner } => {
            let n = points.max(3) as usize;
            let r = size / 2.0;
            let ri = r * inner;
            let core: Vec<Vec2> = (0..2 * n)
                .map(|i| {
                    let a = PI / 2.0 + PI * i as f64 / n as f64;
                    place(Vec2::from_angle(a) * if i % 2 == 0 { r } else { ri })
                })
                .collect();
            // 2n triangles (centre, sommet extérieur, sommet intérieur).
            let area = n as f64 * r * ri * (PI / n as f64).sin();
            HoleGeometry {
                core,
                round: 0.0,
                bound: r,
                area,
            }
        }
        ShapeKind::Slot { aspect } => {
            let width = size * aspect;
            let half = (size - width).max(0.0) / 2.0;
            let core = degenerate(&[Vec2::new(-half, 0.0), Vec2::new(half, 0.0)]);
            let rr = width / 2.0;
            HoleGeometry {
                core: core.into_iter().map(place).collect(),
                round: rr,
                bound: half + rr,
                area: 2.0 * half * width + PI * rr * rr,
            }
        }
    }
}

/// Retire les sommets confondus (rectangle réduit à un segment ou à un point).
fn degenerate(pts: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for &p in pts {
        if !out.iter().any(|q| q.distance(p) < 1e-9) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::Contour;

    fn shape(kind: ShapeKind) -> Shape {
        Shape {
            kind,
            size: 10.0,
            rotation: 0.0,
        }
    }

    #[test]
    fn areas_match_contours() {
        let kinds = [
            ShapeKind::Circle,
            ShapeKind::Rectangle {
                aspect: 0.6,
                corner: 1.5,
            },
            ShapeKind::Polygon { sides: 6 },
            ShapeKind::Star {
                points: 5,
                inner: 0.45,
            },
            ShapeKind::Slot { aspect: 0.3 },
        ];
        for k in kinds {
            let s = shape(k.clone());
            let h = hole_geometry(&s, 10.0, 17.0, Vec2::new(3.0, 4.0));
            let c = Contour::rounded(&h.core, h.round);
            assert!(
                (c.area() - h.area).abs() < 1e-9,
                "{k:?} : {} ≠ {}",
                c.area(),
                h.area
            );
        }
    }

    #[test]
    fn rectangle_with_full_corners_becomes_a_slot() {
        let s = shape(ShapeKind::Rectangle {
            aspect: 0.5,
            corner: 2.5,
        });
        let h = hole_geometry(&s, 10.0, 0.0, Vec2::ZERO);
        assert_eq!(h.core.len(), 2);
        assert_eq!(h.round, 2.5);
    }

    #[test]
    fn hexagon_has_flat_base() {
        let h = hole_geometry(
            &shape(ShapeKind::Polygon { sides: 6 }),
            10.0,
            0.0,
            Vec2::ZERO,
        );
        let min_y = h.core.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert_eq!(
            h.core.iter().filter(|p| (p.y - min_y).abs() < 1e-9).count(),
            2
        );
    }

    #[test]
    fn corner_scales_with_size() {
        let s = shape(ShapeKind::Rectangle {
            aspect: 1.0,
            corner: 2.0,
        });
        assert_eq!(hole_geometry(&s, 20.0, 0.0, Vec2::ZERO).round, 4.0);
    }
}
