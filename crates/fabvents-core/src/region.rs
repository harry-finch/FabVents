//! Régions planes (plaque, zone perforée, trous de fixation, zones
//! réservées), décrites comme les trous par une âme et un arrondi : point +
//! rayon (disque), rectangle réduit + rayon de coin, ou polygone (ellipse,
//! contour libre, tracé). Distances exactes entre une région et un trou.

use crate::contour::Contour;
use crate::geom::{dist_point_segment, dist_segment_segment, point_in_polygon, signed_area, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// Point, segment ou polygone (sens trigonométrique).
    pub core: Vec<Vec2>,
    pub round: f64,
    /// Convexe : un trou y est contenu dès que les sommets de son âme le sont.
    pub convex: bool,
}

impl Region {
    pub fn circle(center: Vec2, radius: f64) -> Self {
        Self {
            core: vec![center],
            round: radius.max(0.0),
            convex: true,
        }
    }

    /// Rectangle centré à coins arrondis (rayon borné à la demi-largeur).
    pub fn rounded_rect(center: Vec2, width: f64, height: f64, corner: f64) -> Self {
        let r = corner.clamp(0.0, width.min(height) / 2.0);
        let (hx, hy) = (width / 2.0 - r, height / 2.0 - r);
        let mut core = vec![
            center + Vec2::new(-hx, -hy),
            center + Vec2::new(hx, -hy),
            center + Vec2::new(hx, hy),
            center + Vec2::new(-hx, hy),
        ];
        core.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if core.len() > 1 && core[0].distance(*core.last().unwrap()) < 1e-9 {
            core.pop();
        }
        Self {
            core,
            round: r,
            convex: true,
        }
    }

    /// Polygone quelconque (orienté dans le sens trigonométrique).
    pub fn polygon(mut pts: Vec<Vec2>) -> Self {
        if signed_area(&pts) < 0.0 {
            pts.reverse();
        }
        let convex = is_convex(&pts);
        Self {
            core: pts,
            round: 0.0,
            convex,
        }
    }

    fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        let n = self.core.len();
        let edges = match n {
            0 => 0,
            1 => 1,
            2 => 1,
            _ => n,
        };
        (0..edges).map(move |i| (self.core[i], self.core[(i + 1) % n]))
    }

    /// Distance signée au bord : négative à l'intérieur.
    pub fn sdf(&self, p: Vec2) -> f64 {
        if self.core.len() == 1 {
            return p.distance(self.core[0]) - self.round;
        }
        let d = self
            .segments()
            .map(|(a, b)| dist_point_segment(p, a, b))
            .fold(f64::INFINITY, f64::min);
        let inside = self.core.len() >= 3 && point_in_polygon(p, &self.core);
        (if inside { -d } else { d }) - self.round
    }

    /// Distance entre une âme de trou et le bord de la région (sans tenir
    /// compte de l'intérieur).
    fn boundary_distance(&self, core: &[Vec2]) -> f64 {
        let hole: Vec<(Vec2, Vec2)> = crate::geom::core_segments(core);
        let mut best = f64::INFINITY;
        for (a, b) in self.segments() {
            for &(c, d) in &hole {
                best = best.min(dist_segment_segment(a, b, c, d));
            }
        }
        best - self.round
    }

    /// Vrai si le trou (âme, arrondi) est entièrement à au moins `margin`
    /// mm à l'intérieur de la région.
    pub fn fits(&self, core: &[Vec2], round: f64, margin: f64) -> bool {
        if self.convex {
            return core.iter().all(|&v| self.sdf(v) <= -(margin + round));
        }
        self.sdf(core[0]) <= 0.0 && self.boundary_distance(core) >= margin + round
    }

    /// Écart entre le trou et la région ; nul ou négatif s'ils se touchent.
    pub fn gap(&self, core: &[Vec2], round: f64) -> f64 {
        crate::geom::core_distance(core, &self.core) - round - self.round
    }

    pub fn contour(&self) -> Contour {
        Contour::rounded(&self.core, self.round)
    }

    /// Contour élargi de `d` mm (garde d'une zone réservée).
    pub fn inflated(&self, d: f64) -> Region {
        if self.convex {
            return Region {
                round: self.round + d,
                ..self.clone()
            };
        }
        let shapes = crate::boolean::offset(&[self.contour().tessellated(0.02)], d);
        Region::polygon(shapes.into_iter().next().unwrap_or_default())
    }
}

fn is_convex(pts: &[Vec2]) -> bool {
    let n = pts.len();
    if n < 4 {
        return true;
    }
    (0..n).all(|i| {
        let (a, b, c) = (pts[i], pts[(i + 1) % n], pts[(i + 2) % n]);
        (b - a).cross(c - b) >= -1e-9
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_distance_is_exact() {
        let r = Region::rounded_rect(Vec2::new(50.0, 25.0), 100.0, 50.0, 10.0);
        assert!((r.sdf(Vec2::new(50.0, 25.0)) + 25.0).abs() < 1e-12);
        assert!(r.sdf(Vec2::new(50.0, 0.0)).abs() < 1e-12);
        assert!((r.sdf(Vec2::ZERO) - (2f64.sqrt() - 1.0) * 10.0).abs() < 1e-9);
    }

    #[test]
    fn concave_region_checks_edges() {
        // Un « L » : le coin rentrant (10, 10) est à l'extérieur du carré plein.
        let l = Region::polygon(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 20.0),
            Vec2::new(0.0, 20.0),
        ]);
        assert!(!l.convex);
        // Un cercle de rayon 3 centré en (12, 12) : centre dehors.
        assert!(!l.fits(&[Vec2::new(12.0, 12.0)], 3.0, 0.0));
        // Centré en (8, 8) : dedans mais à 2 mm du coin rentrant.
        assert!(!l.fits(&[Vec2::new(8.0, 8.0)], 3.0, 0.0));
        assert!(l.fits(&[Vec2::new(5.0, 5.0)], 3.0, 0.0));
    }

    #[test]
    fn gap_between_hole_and_mounting_hole() {
        let m = Region::circle(Vec2::ZERO, 2.0);
        assert!((m.gap(&[Vec2::new(10.0, 0.0)], 3.0) - 5.0).abs() < 1e-12);
        assert!(m.gap(&[Vec2::new(1.0, 0.0)], 1.0) <= 0.0);
    }
}
