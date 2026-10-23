//! Contour de la plaque : distance signée (zone perforable), contour exact,
//! aire. Les contours v0.1 (rectangle à coins arrondis, disque) sont
//! convexes : un trou est entièrement dans la zone dès que les sommets de son
//! âme y sont, à l'arrondi près.

use crate::contour::Contour;
use crate::geom::Vec2;
use crate::project::{Outline, Plate};
use std::f64::consts::PI;

impl Plate {
    /// Rayon de coin réellement appliqué (au plus la demi-largeur, signalé
    /// par la validation).
    pub fn effective_corner(&self) -> f64 {
        match self.outline {
            Outline::Rectangle => self
                .corner_radius
                .clamp(0.0, self.width.min(self.height) / 2.0),
            Outline::Circle => 0.0,
        }
    }

    /// Distance signée au bord : négative à l'intérieur.
    pub fn sdf(&self, p: Vec2) -> f64 {
        let c = self.center();
        match self.outline {
            Outline::Circle => p.distance(c) - self.width / 2.0,
            Outline::Rectangle => {
                let r = self.effective_corner();
                let q = Vec2::new(
                    (p.x - c.x).abs() - (self.width / 2.0 - r),
                    (p.y - c.y).abs() - (self.height / 2.0 - r),
                );
                let outside = Vec2::new(q.x.max(0.0), q.y.max(0.0)).length();
                outside + q.x.max(q.y).min(0.0) - r
            }
        }
    }

    pub fn contour(&self) -> Contour {
        let c = self.center();
        match self.outline {
            Outline::Circle => Contour::Circle {
                center: c,
                radius: self.width / 2.0,
            },
            Outline::Rectangle => {
                let r = self.effective_corner();
                let hx = self.width / 2.0 - r;
                let hy = self.height / 2.0 - r;
                let core = [
                    c + Vec2::new(-hx, -hy),
                    c + Vec2::new(hx, -hy),
                    c + Vec2::new(hx, hy),
                    c + Vec2::new(-hx, hy),
                ];
                Contour::rounded(&core, r)
            }
        }
    }

    pub fn area(&self) -> f64 {
        match self.outline {
            Outline::Circle => PI * self.width * self.width / 4.0,
            Outline::Rectangle => {
                let r = self.effective_corner();
                self.width * self.height - (4.0 - PI) * r * r
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rectangle_distance() {
        let p = Plate {
            width: 100.0,
            height: 50.0,
            corner_radius: 10.0,
            ..Plate::default()
        };
        assert!((p.sdf(Vec2::new(50.0, 25.0)) + 25.0).abs() < 1e-12);
        assert!(p.sdf(Vec2::new(50.0, 0.0)).abs() < 1e-12);
        // Le coin arrondi : le point (0, 0) est à (√2 - 1) × 10 hors du contour.
        assert!((p.sdf(Vec2::ZERO) - (2f64.sqrt() - 1.0) * 10.0).abs() < 1e-9);
        assert!((p.contour().area() - p.area()).abs() < 1e-9);
    }

    #[test]
    fn circle_plate() {
        let p = Plate {
            outline: Outline::Circle,
            width: 120.0,
            ..Plate::default()
        };
        assert_eq!(p.center(), Vec2::new(60.0, 60.0));
        assert!((p.sdf(Vec2::new(60.0, 0.0))).abs() < 1e-12);
    }
}
