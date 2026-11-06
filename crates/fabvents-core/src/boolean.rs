//! Booléens 2D (`i_overlay`) : décalage d'un contour, et découpe des
//! perforations qui se chevauchent ou dépassent de la zone perforée.

use crate::geom::{signed_area, Vec2};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};

type P = [f64; 2];

fn to_p(pts: &[Vec2]) -> Vec<P> {
    pts.iter().map(|v| [v.x, v.y]).collect()
}

fn from_p(pts: &[P]) -> Vec<Vec2> {
    pts.iter().map(|p| Vec2::new(p[0], p[1])).collect()
}

/// Angle maximal d'un segment d'arc (rad) pour les jonctions arrondies.
const ROUND_JOIN: f64 = 0.08;

/// Contours décalés de `d` mm (négatif : vers l'intérieur), jonctions
/// arrondies. Renvoie les contours extérieurs obtenus.
pub fn offset(contours: &[Vec<Vec2>], d: f64) -> Vec<Vec<Vec2>> {
    if d == 0.0 {
        return contours.to_vec();
    }
    let shape: Vec<Vec<P>> = contours.iter().map(|c| to_p(c)).collect();
    let style = OutlineStyle::new(d).line_join(LineJoin::Round(ROUND_JOIN));
    shape
        .outline(&style)
        .into_iter()
        .filter_map(|s| s.into_iter().next())
        .map(|c| from_p(&c))
        .collect()
}

/// Une découpe : contour extérieur et îlots de matière qu'il entoure.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub outer: Vec<Vec2>,
    pub islands: Vec<Vec<Vec2>>,
}

impl Cut {
    pub fn area(&self) -> f64 {
        signed_area(&self.outer).abs()
            - self
                .islands
                .iter()
                .map(|c| signed_area(c).abs())
                .sum::<f64>()
    }
}

/// Union des perforations, coupée par la zone (`zone` : contours d'une
/// forme, extérieur puis trous) et privée des régions `keepouts`.
pub fn cut(holes: &[Vec<Vec2>], zone: &[Vec<Vec2>], keepouts: &[Vec<Vec2>]) -> Vec<Cut> {
    if holes.is_empty() {
        return Vec::new();
    }
    let subj: Vec<Vec<P>> = holes.iter().map(|c| to_p(c)).collect();
    let clip: Vec<Vec<P>> = zone.iter().map(|c| to_p(c)).collect();
    let mut shapes = FloatOverlay::with_subj_and_clip(&subj, &clip)
        .overlay(OverlayRule::Intersect, FillRule::NonZero);
    if !keepouts.is_empty() && !shapes.is_empty() {
        let k: Vec<Vec<P>> = keepouts.iter().map(|c| to_p(c)).collect();
        shapes = FloatOverlay::with_subj_and_clip(&shapes, &k)
            .overlay(OverlayRule::Difference, FillRule::NonZero);
    }
    shapes
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut it = s.into_iter();
            let outer = from_p(&it.next().unwrap());
            Cut {
                outer,
                islands: it.map(|c| from_p(&c)).collect(),
            }
        })
        .collect()
}

/// Formes (contour extérieur puis trous), comme les rend `i_overlay`.
pub type Shapes = Vec<Vec<Vec<Vec2>>>;

/// Ajoute (`add`) ou retire les contours `layer` (règle non nulle) des
/// formes `acc`.
pub fn combine(acc: &Shapes, layer: &[Vec<Vec2>], add: bool) -> Shapes {
    let subj: Vec<Vec<Vec<P>>> = acc
        .iter()
        .map(|s| s.iter().map(|c| to_p(c)).collect())
        .collect();
    let clip: Vec<Vec<P>> = layer.iter().map(|c| to_p(c)).collect();
    let rule = if add {
        OverlayRule::Union
    } else {
        OverlayRule::Difference
    };
    FloatOverlay::with_subj_and_clip(&subj, &clip)
        .overlay(rule, FillRule::NonZero)
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.into_iter().map(|c| from_p(&c)).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(x, y),
            Vec2::new(x + s, y),
            Vec2::new(x + s, y + s),
            Vec2::new(x, y + s),
        ]
    }

    #[test]
    fn overlapping_holes_merge() {
        let r = cut(
            &[square(0.0, 0.0, 2.0), square(1.0, 0.0, 2.0)],
            &[square(-10.0, -10.0, 20.0)],
            &[],
        );
        assert_eq!(r.len(), 1);
        assert!((r[0].area() - 6.0).abs() < 1e-6);
    }

    #[test]
    fn holes_are_clipped_by_the_zone() {
        let r = cut(&[square(-1.0, -1.0, 2.0)], &[square(0.0, 0.0, 10.0)], &[]);
        assert!((r[0].area() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_ring_of_holes_leaves_an_island() {
        let ring = [square(0.0, 0.0, 6.0)];
        let inner = square(2.0, 2.0, 2.0);
        // Anneau = carré 6 × 6 moins carré 2 × 2 (même forme, trou à l'envers).
        let mut hole = inner.clone();
        hole.reverse();
        let r = cut(&[ring[0].clone(), hole], &[square(-10.0, -10.0, 30.0)], &[]);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].islands.len(), 1);
        assert!((r[0].area() - 32.0).abs() < 1e-6);
    }

    #[test]
    fn keepouts_are_removed() {
        let r = cut(
            &[square(0.0, 0.0, 10.0)],
            &[square(-10.0, -10.0, 30.0)],
            &[square(0.0, 0.0, 5.0)],
        );
        let total: f64 = r.iter().map(Cut::area).sum();
        assert!((total - 75.0).abs() < 1e-6);
    }

    #[test]
    fn layers_add_and_subtract() {
        let a = combine(&Vec::new(), &[square(0.0, 0.0, 4.0)], true);
        let b = combine(&a, &[square(2.0, 0.0, 4.0)], true);
        assert_eq!(b.len(), 1);
        assert!((signed_area(&b[0][0]).abs() - 24.0).abs() < 1e-6);
        let c = combine(&b, &[square(1.0, 1.0, 2.0)], false);
        let area: f64 = c[0].iter().map(|r| signed_area(r)).sum::<f64>();
        assert!((area.abs() - 20.0).abs() < 1e-6, "{area}");
    }

    #[test]
    fn negative_offset_shrinks() {
        let o = offset(&[square(0.0, 0.0, 10.0)], -1.0);
        assert!((signed_area(&o[0]).abs() - 64.0).abs() < 1e-3);
        let g = offset(&[square(0.0, 0.0, 10.0)], 1.0);
        assert!(signed_area(&g[0]).abs() > 100.0 + 40.0 - 1e-6);
    }
}
