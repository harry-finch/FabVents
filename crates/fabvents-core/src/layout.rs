//! Dispositions : placement des germes (centres des perforations), sans
//! connaître la forme ni les contrôleurs.

use crate::geom::{Vec2, DEG};
use crate::issue::{Issue, Severity};
use crate::project::{Grid, GridPattern, Layout, Plate};

/// Nombre maximal de positions candidates explorées par une disposition.
pub const MAX_CANDIDATES: usize = 400_000;

/// Position d'un germe et angle propre à la disposition (degrés).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub pos: Vec2,
    pub angle: f64,
}

pub fn seeds(layout: &Layout, plate: &Plate) -> Result<Vec<Seed>, Issue> {
    match layout {
        Layout::Grid(g) => grid(g, plate),
    }
}

/// Grille centrée sur la plaque (puis décalée) : le motif reste symétrique
/// quand on change les dimensions.
fn grid(g: &Grid, plate: &Plate) -> Result<Vec<Seed>, Issue> {
    let (px, py, stagger) = match g.pattern {
        GridPattern::Square => (g.pitch_x, g.pitch_y, false),
        GridPattern::Staggered => (g.pitch_x, g.pitch_y, true),
        GridPattern::Hexagonal => (g.pitch_x, g.pitch_x * 3f64.sqrt() / 2.0, true),
    };
    let ext = plate.extent();
    let origin = plate.center() + Vec2::new(g.offset_x, g.offset_y);
    // Rayon à couvrir autour de l'origine de la grille, quelle que soit sa rotation.
    let reach = 0.5 * ext.length() + Vec2::new(g.offset_x, g.offset_y).length() + px.max(py);
    let nx = (reach / px).ceil() as i64 + 1;
    let ny = (reach / py).ceil() as i64 + 1;
    let candidates = (2 * nx + 1) as f64 * (2 * ny + 1) as f64;
    if candidates > MAX_CANDIDATES as f64 {
        return Err(Issue::new("layout.too_many_holes", Severity::Error)
            .field("layout.pitch_x")
            .count(candidates as usize));
    }

    let rot = g.angle * DEG;
    let mut out = Vec::new();
    for j in -ny..=ny {
        let shift = if stagger && j.rem_euclid(2) == 1 {
            px / 2.0
        } else {
            0.0
        };
        for i in -nx..=nx {
            let local = Vec2::new(i as f64 * px + shift, j as f64 * py);
            let pos = origin + local.rotated(rot);
            if pos.x < 0.0 || pos.y < 0.0 || pos.x > ext.x || pos.y > ext.y {
                continue;
            }
            out.push(Seed {
                pos,
                angle: g.angle + j as f64 * g.row_twist + i as f64 * g.col_twist,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plate(w: f64, h: f64) -> Plate {
        Plate {
            width: w,
            height: h,
            ..Plate::default()
        }
    }

    #[test]
    fn square_grid_is_centered_and_counted() {
        let g = Grid {
            pattern: GridPattern::Square,
            pitch_x: 10.0,
            pitch_y: 10.0,
            ..Grid::default()
        };
        let s = grid(&g, &plate(100.0, 50.0)).unwrap();
        // x = 0, 10, … 100 (11 colonnes) ; y = 5, 15, … 45 (5 rangées).
        assert_eq!(s.len(), 55);
        assert!(s.iter().any(|q| q.pos == Vec2::new(50.0, 25.0)));
    }

    #[test]
    fn hexagonal_rows_are_offset_by_half_pitch() {
        let g = Grid {
            pattern: GridPattern::Hexagonal,
            pitch_x: 10.0,
            ..Grid::default()
        };
        let s = grid(&g, &plate(100.0, 100.0)).unwrap();
        let dy = 10.0 * 3f64.sqrt() / 2.0;
        let row1: Vec<_> = s
            .iter()
            .filter(|q| (q.pos.y - (50.0 + dy)).abs() < 1e-9)
            .collect();
        assert!(row1.iter().any(|q| (q.pos.x - 55.0).abs() < 1e-9));
    }

    #[test]
    fn twist_accumulates_per_row_and_column() {
        let g = Grid {
            pattern: GridPattern::Square,
            row_twist: 5.0,
            col_twist: 1.0,
            ..Grid::default()
        };
        let s = grid(&g, &plate(100.0, 100.0)).unwrap();
        let at = |x: f64, y: f64| {
            s.iter()
                .find(|q| q.pos.distance(Vec2::new(x, y)) < 1e-9)
                .unwrap()
                .angle
        };
        assert_eq!(at(50.0, 50.0), 0.0);
        assert_eq!(at(58.0, 50.0), 1.0);
        assert_eq!(at(50.0, 58.0), 5.0);
    }

    #[test]
    fn tiny_pitch_is_refused() {
        let g = Grid {
            pitch_x: 0.01,
            pitch_y: 0.01,
            ..Grid::default()
        };
        assert_eq!(
            grid(&g, &plate(200.0, 150.0)).unwrap_err().code,
            "layout.too_many_holes"
        );
    }
}
