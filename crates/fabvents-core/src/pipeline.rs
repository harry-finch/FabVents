//! Calcul d'un motif par étages, avec cache :
//!
//! ```text
//! germes   (plaque, disposition)                       ─┐
//! trous    (germes, forme, contrôleurs, bords, taille mini) ─┤ recalculé seulement
//! mesures  (trous, nervure mini)                        ─┘ si ses entrées changent
//! ```
//!
//! Déplacer un contrôleur de taille ne relance pas la disposition ; changer
//! la nervure minimale ne relance que les mesures.

use crate::contour::Contour;
use crate::field::ChannelField;
use crate::geom::Vec2;
use crate::issue::{Issue, Severity};
use crate::layout::{seeds, Seed};
use crate::metrics::{measure, Metrics};
use crate::project::{Channel, Controller, EdgeMode, Layout, Plate, Project, Shape};
use crate::shape::{hole_geometry, HoleGeometry};
use crate::validate::validate;
use serde::Serialize;

/// Nombre maximal de perforations d'un motif.
pub const MAX_HOLES: usize = 150_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Hole {
    pub center: Vec2,
    /// Taille de référence (mm), après le canal Taille.
    pub size: f64,
    /// Angle (degrés), après le canal Angle et la disposition.
    pub angle: f64,
    pub geom: HoleGeometry,
}

impl Hole {
    pub fn contour(&self) -> Contour {
        Contour::rounded(&self.geom.core, self.geom.round)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Stats {
    /// Germes placés par la disposition dans l'emprise de la plaque.
    pub seeds: usize,
    /// Trous retirés car hors de la zone perforable.
    pub removed_edge: usize,
    /// Trous retirés car plus petits que la taille minimale.
    pub removed_small: usize,
}

/// Résultat d'un calcul.
pub struct Frame<'a> {
    pub holes: &'a [Hole],
    pub metrics: Metrics,
    pub stats: Stats,
    pub issues: Vec<Issue>,
}

impl Frame<'_> {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }
}

#[derive(PartialEq)]
struct SeedsKey {
    layout: Layout,
    plate: Plate,
}

#[derive(PartialEq)]
struct HolesKey {
    seeds_gen: u64,
    plate: Plate,
    shape: Shape,
    controllers: Vec<Controller>,
    edge: EdgeMode,
    min_size: f64,
}

#[derive(PartialEq)]
struct MetricsKey {
    holes_gen: u64,
    min_bridge: f64,
}

#[derive(Default)]
pub struct Pipeline {
    seeds_key: Option<SeedsKey>,
    seeds: Vec<Seed>,
    seeds_error: Option<Issue>,
    seeds_gen: u64,
    holes_key: Option<HolesKey>,
    holes: Vec<Hole>,
    stats: Stats,
    holes_gen: u64,
    metrics_key: Option<MetricsKey>,
    metrics: Metrics,
    /// Étages recalculés au dernier appel (germes, trous, mesures).
    pub last_runs: [bool; 3],
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute(&mut self, p: &Project) -> Frame<'_> {
        self.last_runs = [false; 3];
        let mut issues = validate(p);
        if issues.iter().any(|i| i.severity == Severity::Error) {
            self.invalidate();
            return Frame {
                holes: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: Stats::default(),
                issues,
            };
        }

        // Étage 1 : germes.
        let key = SeedsKey {
            layout: p.layout.clone(),
            plate: p.plate.clone(),
        };
        if self.seeds_key.as_ref() != Some(&key) {
            match seeds(&p.layout, &p.plate) {
                Ok(s) => {
                    self.seeds = s;
                    self.seeds_error = None;
                }
                Err(e) => {
                    self.seeds.clear();
                    self.seeds_error = Some(e);
                }
            }
            self.seeds_key = Some(key);
            self.seeds_gen += 1;
            self.last_runs[0] = true;
        }
        if let Some(e) = &self.seeds_error {
            issues.push(e.clone());
            self.holes_key = None;
            self.metrics_key = None;
            return Frame {
                holes: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: Stats::default(),
                issues,
            };
        }

        // Étage 2 : trous.
        let key = HolesKey {
            seeds_gen: self.seeds_gen,
            plate: p.plate.clone(),
            shape: p.shape.clone(),
            controllers: p.controllers.clone(),
            edge: p.edge,
            min_size: p.fabrication.min_size,
        };
        if self.holes_key.as_ref() != Some(&key) {
            let (holes, stats) = build_holes(p, &self.seeds);
            self.holes = holes;
            self.stats = stats;
            self.holes_key = Some(key);
            self.holes_gen += 1;
            self.last_runs[1] = true;
        }
        if self.holes.len() > MAX_HOLES {
            issues.push(
                Issue::error("layout.too_many_holes", "layout.pitch_x").count(self.holes.len()),
            );
            return Frame {
                holes: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: self.stats,
                issues,
            };
        }

        // Étage 3 : mesures.
        let key = MetricsKey {
            holes_gen: self.holes_gen,
            min_bridge: p.fabrication.min_bridge,
        };
        if self.metrics_key.as_ref() != Some(&key) {
            self.metrics = measure(&self.holes, p.plate.area(), p.fabrication.min_bridge);
            self.metrics_key = Some(key);
            self.last_runs[2] = true;
        }

        let m = &self.metrics;
        if self.holes.is_empty() {
            issues.push(Issue::info("holes.none"));
        }
        if self.stats.removed_small > 0 {
            issues.push(
                Issue::info("holes.removed_small")
                    .count(self.stats.removed_small)
                    .value(p.fabrication.min_size)
                    .field("fabrication.min_size"),
            );
        }
        if m.overlaps > 0 {
            issues.push(Issue::warning("holes.overlap").count(m.overlaps));
        }
        if m.thin_bridges > 0 {
            let mut i = Issue::warning("bridge.thin")
                .count(m.thin_bridges)
                .field("fabrication.min_bridge");
            i.value = m.bridge_min;
            issues.push(i);
        }

        Frame {
            holes: &self.holes,
            metrics: self.metrics.clone(),
            stats: self.stats,
            issues,
        }
    }

    fn invalidate(&mut self) {
        self.seeds_key = None;
        self.holes_key = None;
        self.metrics_key = None;
    }
}

fn build_holes(p: &Project, seeds: &[Seed]) -> (Vec<Hole>, Stats) {
    let size_field = ChannelField::new(&p.controllers, Channel::Size);
    let angle_field = ChannelField::new(&p.controllers, Channel::Angle);
    let margin = p.plate.margin;
    let mut stats = Stats {
        seeds: seeds.len(),
        ..Stats::default()
    };
    let mut holes = Vec::with_capacity(seeds.len());
    for s in seeds {
        if p.plate.sdf(s.pos) > -margin {
            stats.removed_edge += 1;
            continue;
        }
        let size = size_field.value(p.shape.size, s.pos);
        if size < p.fabrication.min_size || size <= 0.0 {
            stats.removed_small += 1;
            continue;
        }
        let angle = angle_field.value(p.shape.rotation, s.pos) + s.angle;
        let geom = hole_geometry(&p.shape, size, angle, s.pos);
        if p.edge == EdgeMode::Whole
            && geom
                .core
                .iter()
                .any(|&v| p.plate.sdf(v) > -(margin + geom.round))
        {
            stats.removed_edge += 1;
            continue;
        }
        holes.push(Hole {
            center: s.pos,
            size,
            angle,
            geom,
        });
    }
    (holes, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Geometry, Grid, GridPattern, Influence};

    fn square(pitch: f64) -> Project {
        Project {
            layout: Layout::Grid(Grid {
                pattern: GridPattern::Square,
                pitch_x: pitch,
                pitch_y: pitch,
                ..Grid::default()
            }),
            plate: Plate {
                width: 100.0,
                height: 100.0,
                corner_radius: 0.0,
                margin: 0.0,
                ..Plate::default()
            },
            ..Project::default()
        }
    }

    #[test]
    fn whole_mode_drops_holes_crossing_the_edge() {
        let mut p = square(10.0);
        p.edge = EdgeMode::Center;
        let n_center = Pipeline::new().compute(&p).holes.len();
        p.edge = EdgeMode::Whole;
        let n_whole = Pipeline::new().compute(&p).holes.len();
        // 11 × 11 centres dans la plaque ; les 40 du bord dépassent.
        assert_eq!(n_center, 121);
        assert_eq!(n_whole, 81);
    }

    #[test]
    fn stages_are_cached() {
        let mut p = Project::example();
        let mut pl = Pipeline::new();
        pl.compute(&p);
        assert_eq!(pl.last_runs, [true, true, true]);
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, false, false]);
        if let Geometry::Point { at } = &mut p.controllers[0].geometry {
            at.x += 5.0;
        }
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, true, true]);
        p.fabrication.min_bridge = 2.0;
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, false, true]);
    }

    #[test]
    fn small_holes_are_removed_and_reported() {
        let mut p = square(10.0);
        p.controllers.push(Controller {
            id: 1,
            geometry: Geometry::Point {
                at: Vec2::new(50.0, 50.0),
            },
            size: Some(Influence {
                reach: 12.0,
                target: 0.0,
                hard_edge: true,
            }),
            angle: None,
        });
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        assert_eq!(f.stats.removed_small, 5);
        assert!(f
            .issues
            .iter()
            .any(|i| i.code == "holes.removed_small" && i.count == Some(5)));
    }

    #[test]
    fn invalid_input_gives_errors_and_no_holes() {
        let mut p = Project::example();
        p.shape.size = -1.0;
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        assert!(f.has_errors());
        assert!(f.holes.is_empty());
    }

    #[test]
    fn example_has_sound_metrics() {
        let mut pl = Pipeline::new();
        let f = pl.compute(&Project::example());
        assert!(f.holes.len() > 300);
        assert!(f.metrics.open_ratio > 20.0 && f.metrics.open_ratio < 60.0);
        assert!(f.metrics.bridge_min.unwrap() > 0.9);
        assert_eq!(f.metrics.overlaps, 0);
    }
}
