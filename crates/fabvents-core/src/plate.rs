//! Contour de la plaque et zone perforable : régions (`region`), contour
//! exact, aire, et contour de la zone perforable (plaque réduite de la
//! marge, intersectée avec la zone perforée) pour la découpe au bord.

use crate::boolean::offset;
use crate::contour::Contour;
use crate::geom::{signed_area, Vec2};
use crate::project::{Outline, Plate, Zone};
use crate::region::Region;
use std::f64::consts::{PI, TAU};

/// Sommets de l'ellipse de la plaque.
const ELLIPSE_SEGMENTS: usize = 256;

impl Plate {
    /// Rayon de coin réellement appliqué (au plus la demi-largeur, signalé
    /// par la validation).
    pub fn effective_corner(&self) -> f64 {
        match self.outline {
            Outline::Rectangle => self
                .corner_radius
                .clamp(0.0, self.width.min(self.height) / 2.0),
            _ => 0.0,
        }
    }

    /// Région de la plaque.
    pub fn region(&self) -> Region {
        let c = self.center();
        match self.outline {
            Outline::Rectangle => {
                Region::rounded_rect(c, self.width, self.height, self.corner_radius)
            }
            Outline::Circle => Region::circle(c, self.width / 2.0),
            Outline::Ellipse => {
                let (a, b) = (self.width / 2.0, self.height / 2.0);
                Region::polygon(
                    (0..ELLIPSE_SEGMENTS)
                        .map(|k| {
                            let t = TAU * k as f64 / ELLIPSE_SEGMENTS as f64;
                            c + Vec2::new(a * t.cos(), b * t.sin())
                        })
                        .collect(),
                )
            }
            Outline::Custom => Region::polygon(self.custom.clone()),
        }
    }

    /// Zone perforée choisie (aucune : toute la plaque).
    pub fn zone_region(&self) -> Option<Region> {
        match &self.zone {
            Zone::Plate => None,
            Zone::Circle { center, diameter } => Some(Region::circle(*center, diameter / 2.0)),
            Zone::Rectangle {
                center,
                width,
                height,
                corner,
            } => Some(Region::rounded_rect(*center, *width, *height, *corner)),
        }
    }

    /// Distance signée au bord de la plaque : négative à l'intérieur.
    pub fn sdf(&self, p: Vec2) -> f64 {
        self.region().sdf(p)
    }

    pub fn contour(&self) -> Contour {
        self.region().contour()
    }

    pub fn area(&self) -> f64 {
        match self.outline {
            Outline::Ellipse => PI * self.width * self.height / 4.0,
            Outline::Custom => signed_area(&self.custom).abs(),
            _ => self.region().contour().area(),
        }
    }

    /// Contours de la zone perforable : plaque réduite de la marge, et
    /// zone perforée. Une seule forme (contour extérieur), ou aucune.
    pub fn perforable(&self) -> Vec<Vec<Vec2>> {
        let plate = self
            .region()
            .contour()
            .tessellated(crate::pipeline::CUT_TOLERANCE);
        let inset = offset(&[plate], -self.margin);
        let Some(zone) = self.zone_region() else {
            return inset;
        };
        let z = zone.contour().tessellated(crate::pipeline::CUT_TOLERANCE);
        crate::boolean::cut(&inset, &[z], &[])
            .into_iter()
            .map(|c| c.outer)
            .collect()
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
        assert!((p.sdf(Vec2::ZERO) - (2f64.sqrt() - 1.0) * 10.0).abs() < 1e-9);
        assert!((p.contour().area() - p.area()).abs() < 1e-9);
    }

    #[test]
    fn circle_and_ellipse_plates() {
        let c = Plate {
            outline: Outline::Circle,
            width: 120.0,
            ..Plate::default()
        };
        assert_eq!(c.center(), Vec2::new(60.0, 60.0));
        assert!(c.sdf(Vec2::new(60.0, 0.0)).abs() < 1e-12);
        let e = Plate {
            outline: Outline::Ellipse,
            width: 200.0,
            height: 100.0,
            ..Plate::default()
        };
        assert!(e.sdf(Vec2::new(100.0, 0.0)).abs() < 0.01);
        assert!(e.sdf(Vec2::new(0.0, 50.0)).abs() < 0.01);
        assert!((e.contour().area() - e.area()).abs() / e.area() < 1e-3);
    }

    #[test]
    fn perforable_area_is_inset_and_zoned() {
        let p = Plate {
            width: 100.0,
            height: 100.0,
            corner_radius: 0.0,
            margin: 5.0,
            ..Plate::default()
        };
        let a = signed_area(&p.perforable()[0]).abs();
        assert!((a - 8100.0).abs() < 1.0, "{a}");
        let z = Plate {
            zone: Zone::Circle {
                center: Vec2::new(50.0, 50.0),
                diameter: 40.0,
            },
            ..p
        };
        let a = signed_area(&z.perforable()[0]).abs();
        assert!((a - PI * 400.0).abs() < 1.0, "{a}");
    }
}
