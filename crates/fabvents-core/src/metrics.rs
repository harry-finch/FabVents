//! Mesures d'un motif : taux d'ouverture, tailles, nervures, chevauchements.
//!
//! La nervure entre deux trous est la distance entre leurs âmes moins leurs
//! arrondis : exacte pour toutes les formes, sans facettisation. Les paires
//! sont cherchées par une grille de répartition, jusqu'à `search` mm.

use crate::geom::core_distance;
use crate::pipeline::Hole;
use serde::Serialize;

/// Distance minimale explorée pour mesurer les nervures (mm).
pub const BRIDGE_SEARCH_MIN: f64 = 5.0;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Metrics {
    pub holes: usize,
    pub plate_area: f64,
    /// Surface ouverte : aire exacte des perforations, chevauchements
    /// fusionnés et découpe au bord comprise.
    pub open_area: f64,
    /// Taux d'ouverture (%).
    pub open_ratio: f64,
    pub size_min: Option<f64>,
    pub size_max: Option<f64>,
    /// Plus petite nervure entre deux trous qui ne se touchent pas ;
    /// `None` si aucune paire n'est plus proche que `bridge_search`.
    pub bridge_min: Option<f64>,
    pub bridge_search: f64,
    /// Paires de trous qui se touchent ou se chevauchent.
    pub overlaps: usize,
    /// Paires séparées par une nervure plus fine que le minimum demandé.
    pub thin_bridges: usize,
    /// Îlots de matière entièrement entourés par des perforations fusionnées
    /// (ils tomberont à la découpe).
    pub islands: usize,
}

pub fn measure(holes: &[Hole], plate_area: f64, min_bridge: f64) -> Metrics {
    measure_with_overlaps(holes, plate_area, min_bridge).0
}

/// Mesures, et pour chaque trou : touche-t-il ou chevauche-t-il un voisin ?
pub fn measure_with_overlaps(
    holes: &[Hole],
    plate_area: f64,
    min_bridge: f64,
) -> (Metrics, Vec<bool>) {
    let mut overlapping = vec![false; holes.len()];
    let mut m = Metrics {
        holes: holes.len(),
        plate_area,
        ..Metrics::default()
    };
    if holes.is_empty() {
        return (m, overlapping);
    }
    for h in holes {
        m.open_area += h.geom.area;
        m.size_min = Some(m.size_min.map_or(h.size, |v| v.min(h.size)));
        m.size_max = Some(m.size_max.map_or(h.size, |v| v.max(h.size)));
    }
    m.open_ratio = if plate_area > 0.0 {
        100.0 * m.open_area / plate_area
    } else {
        0.0
    };

    let search = BRIDGE_SEARCH_MIN.max(3.0 * min_bridge);
    m.bridge_search = search;
    let rmax = holes.iter().map(|h| h.geom.bound).fold(0.0f64, f64::max);
    let cell = (2.0 * rmax + search).max(1e-3);

    // Grille de répartition compacte (tri par case).
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for h in holes {
        x0 = x0.min(h.center.x);
        y0 = y0.min(h.center.y);
        x1 = x1.max(h.center.x);
        y1 = y1.max(h.center.y);
    }
    let nx = (((x1 - x0) / cell).floor() as usize + 1).min(4096);
    let ny = (((y1 - y0) / cell).floor() as usize + 1).min(4096);
    let cell_of = |i: usize| {
        let c = holes[i].center;
        let cx = (((c.x - x0) / cell) as usize).min(nx - 1);
        let cy = (((c.y - y0) / cell) as usize).min(ny - 1);
        (cx, cy)
    };
    let mut start = vec![0usize; nx * ny + 1];
    for i in 0..holes.len() {
        let (cx, cy) = cell_of(i);
        start[cy * nx + cx + 1] += 1;
    }
    for k in 0..nx * ny {
        start[k + 1] += start[k];
    }
    let mut fill = start.clone();
    let mut order = vec![0usize; holes.len()];
    for i in 0..holes.len() {
        let (cx, cy) = cell_of(i);
        let k = cy * nx + cx;
        order[fill[k]] = i;
        fill[k] += 1;
    }

    let mut best = f64::INFINITY;
    for i in 0..holes.len() {
        let (cx, cy) = cell_of(i);
        let a = &holes[i];
        for gy in cy.saturating_sub(1)..=(cy + 1).min(ny - 1) {
            for gx in cx.saturating_sub(1)..=(cx + 1).min(nx - 1) {
                let k = gy * nx + gx;
                for &j in &order[start[k]..start[k + 1]] {
                    if j <= i {
                        continue;
                    }
                    let b = &holes[j];
                    let d = a.center.distance(b.center);
                    if d - a.geom.bound - b.geom.bound >= search {
                        continue;
                    }
                    let gap = if a.geom.core.len() == 1 && b.geom.core.len() == 1 {
                        d - a.geom.round - b.geom.round
                    } else {
                        core_distance(&a.geom.core, &b.geom.core) - a.geom.round - b.geom.round
                    };
                    if gap <= 1e-9 {
                        m.overlaps += 1;
                        overlapping[i] = true;
                        overlapping[j] = true;
                    } else {
                        best = best.min(gap);
                        if gap < min_bridge {
                            m.thin_bridges += 1;
                        }
                    }
                }
            }
        }
    }
    if best.is_finite() && best < search {
        m.bridge_min = Some(best);
    }
    (m, overlapping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Vec2;
    use crate::project::{Shape, ShapeKind};
    use crate::shape::hole_geometry;

    fn circle(x: f64, d: f64) -> Hole {
        let s = Shape::default();
        Hole {
            center: Vec2::new(x, 0.0),
            size: d,
            angle: 0.0,
            geom: hole_geometry(&s, d, 0.0, Vec2::new(x, 0.0)),
        }
    }

    #[test]
    fn bridges_between_circles() {
        let holes = [circle(0.0, 4.0), circle(6.0, 4.0), circle(11.0, 4.0)];
        let m = measure(&holes, 1000.0, 1.5);
        assert_eq!(m.bridge_min, Some(1.0));
        assert_eq!(m.thin_bridges, 1);
        assert_eq!(m.overlaps, 0);
    }

    #[test]
    fn overlaps_are_counted() {
        let holes = [circle(0.0, 4.0), circle(3.0, 4.0)];
        assert_eq!(measure(&holes, 1000.0, 1.0).overlaps, 1);
    }

    #[test]
    fn slot_bridge_is_exact() {
        let s = Shape {
            kind: ShapeKind::Slot { aspect: 0.25 },
            size: 8.0,
            rotation: 0.0,
        };
        let mk = |y: f64| Hole {
            center: Vec2::new(0.0, y),
            size: 8.0,
            angle: 0.0,
            geom: hole_geometry(&s, 8.0, 0.0, Vec2::new(0.0, y)),
        };
        // Fentes horizontales de 2 mm de large, entraxe 3,5 mm → nervure 1,5 mm.
        let m = measure(&[mk(0.0), mk(3.5)], 1000.0, 1.0);
        assert!((m.bridge_min.unwrap() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn open_ratio() {
        let m = measure(&[circle(0.0, 2.0)], std::f64::consts::PI * 4.0, 1.0);
        assert!((m.open_ratio - 25.0).abs() < 1e-9);
        assert_eq!(m.bridge_min, None);
    }
}
