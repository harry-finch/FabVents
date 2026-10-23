//! Gabarits : grilles de ventilateur aux dimensions usuelles (cadre carré,
//! ouverture ronde, quatre trous de fixation). Dimensions courantes des
//! ventilateurs de PC : à vérifier sur la fiche du ventilateur utilisé.

use crate::geom::Vec2;
use crate::project::{
    EdgeMode, Feature, FeatureShape, Grid, GridPattern, Layout, Plate, Project, Shape, ShapeKind,
    Zone,
};

/// Taille du cadre (mm), entraxe des fixations (mm), diamètre des
/// fixations (mm), diamètre de l'ouverture (mm).
pub const FANS: [(u32, f64, f64, f64); 6] = [
    (40, 32.0, 3.4, 38.0),
    (60, 50.0, 4.3, 57.0),
    (80, 71.5, 4.3, 76.0),
    (92, 82.5, 4.3, 88.0),
    (120, 105.0, 4.3, 116.0),
    (140, 124.5, 4.3, 136.0),
];

/// Grille de ventilateur de `size` mm, ou `None` si la taille n'est pas au catalogue.
pub fn fan_grill(size: u32) -> Option<Project> {
    let &(s, spacing, hole, opening) = FANS.iter().find(|f| f.0 == size)?;
    let side = s as f64;
    let c = Vec2::new(side / 2.0, side / 2.0);
    let small = s < 80;
    let h = spacing / 2.0;
    let features = [(-h, -h), (h, -h), (h, h), (-h, h)]
        .iter()
        .enumerate()
        .map(|(i, &(dx, dy))| Feature {
            id: i as u32 + 1,
            shape: FeatureShape::Circle {
                center: c + Vec2::new(dx, dy),
                diameter: hole,
            },
            cut: true,
            clearance: 1.5,
        })
        .collect();
    Some(Project {
        name: format!("Grille ventilateur {s} mm"),
        plate: Plate {
            width: side,
            height: side,
            corner_radius: if small { 3.0 } else { 4.0 },
            thickness: 2.0,
            margin: 1.0,
            zone: Zone::Circle {
                center: c,
                diameter: opening,
            },
            ..Plate::default()
        },
        layout: Layout::Grid(Grid {
            pattern: GridPattern::Hexagonal,
            pitch_x: if small { 3.8 } else { 4.4 },
            ..Grid::default()
        }),
        shape: Shape {
            kind: ShapeKind::Circle,
            size: if small { 2.6 } else { 3.2 },
            rotation: 0.0,
        },
        edge: EdgeMode::Whole,
        features,
        ..Project::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;

    #[test]
    fn every_fan_template_is_sound() {
        for &(s, ..) in &FANS {
            let p = fan_grill(s).unwrap();
            let mut pl = Pipeline::new();
            let f = pl.compute(&p);
            assert!(!f.has_errors(), "{s} : {:?}", f.issues);
            assert!(f.holes.len() > 50, "{s}");
            assert_eq!(f.metrics.overlaps, 0, "{s}");
            assert_eq!(f.metrics.thin_bridges, 0, "{s}");
            assert!(
                f.metrics.open_ratio > 25.0,
                "{s} : {}",
                f.metrics.open_ratio
            );
        }
        assert!(fan_grill(77).is_none());
    }
}
