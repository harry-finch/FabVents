//! Calcul d'un motif par étages, avec cache :
//!
//! ```text
//! germes   (plaque, disposition, contrôleurs d'espacement)   ─┐
//! trous    (germes, forme, contrôleurs, bords, zone,          │
//!           éléments, taille mini)                            ├ recalculé seulement
//! mesures  (trous, nervure mini)                              │ si ses entrées changent
//! découpe  (trous qui se chevauchent ou dépassent : booléens) ─┘
//! ```
//!
//! Un trou isolé et entièrement dans la zone reste exact (cercle, arcs) ;
//! seuls les trous qui se chevauchent, ou qui dépassent du bord en mode
//! « découpe au contour », passent par les booléens et deviennent des
//! polygones (`cuts`).
//!
//! Déplacer un contrôleur de taille ne relance pas la disposition ; changer
//! la nervure minimale ne relance que les mesures.

use crate::boolean::{cut, Cut};
use crate::contour::Contour;
use crate::field::Field;
use crate::fit::{fit, Circles};
use crate::geom::Vec2;
use crate::issue::{Issue, Severity};
use crate::layout::{seeds, Seed};
use crate::metrics::{measure_with_overlaps, Metrics};
use crate::project::{
    Channel, Controller, EdgeMode, Fabrication, Feature, FeatureShape, Layout, Plate, Project,
    Shape,
};
use crate::region::Region;
use crate::shape::{hole_geometry, HoleGeometry};
use crate::solid::{self, SolidPlan};
use crate::validate::validate;
use serde::Serialize;

/// Flèche des arcs des trous découpés (mm) : écart de contour négligeable,
/// aire à 0,05 % près.
pub(crate) const CUT_TOLERANCE: f64 = 0.002;

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
    /// Trous retirés par une disparition aléatoire.
    pub removed_cull: usize,
    /// Trous retirés car trop près d'un trou de fixation ou d'une zone réservée.
    pub removed_feature: usize,
    /// Trous fusionnés ou coupés au bord (devenus des polygones).
    pub merged: usize,
}

/// Résultat d'un calcul.
pub struct Frame<'a> {
    pub holes: &'a [Hole],
    /// Pour chaque trou : remplacé par les découpes (`cuts`).
    pub complex: &'a [bool],
    /// Perforations fusionnées ou coupées au bord (polygones fins, pour les
    /// aires).
    pub cuts: &'a [Cut],
    /// Les mêmes découpes aux contours exacts (arcs retrouvés), pour les
    /// exports.
    pub cut_contours: &'a [CutContour],
    pub metrics: Metrics,
    pub stats: Stats,
    pub issues: Vec<Issue>,
    /// Pièces à extruder et contrôles de l'export STEP, calculés seulement
    /// quand une dépouille ou des dalles sont demandées (sinon vide).
    pub plan: &'a SolidPlan,
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
    /// Contrôleurs du canal Espacement (seulement pour les dispositions qui le suivent).
    spacing: Vec<Controller>,
}

#[derive(PartialEq)]
struct HolesKey {
    seeds_gen: u64,
    plate: Plate,
    shape: Shape,
    controllers: Vec<Controller>,
    edge: EdgeMode,
    min_size: f64,
    features: Vec<Feature>,
}

#[derive(PartialEq)]
struct PlanKey {
    cut_gen: u64,
    plate: Plate,
    features: Vec<Feature>,
    fabrication: Fabrication,
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
    overlapping: Vec<bool>,
    metrics_gen: u64,
    cut_gen: u64,
    complex: Vec<bool>,
    cuts: Vec<Cut>,
    cut_contours: Vec<CutContour>,
    plan_key: Option<PlanKey>,
    plan: SolidPlan,
    empty_plan: SolidPlan,
    /// Étages recalculés au dernier appel (germes, trous, mesures, découpe,
    /// pièces de l'export STEP).
    pub last_runs: [bool; 5],
}

impl Pipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute(&mut self, p: &Project) -> Frame<'_> {
        self.last_runs = [false; 5];
        let mut issues = validate(p);
        if issues.iter().any(|i| i.severity == Severity::Error) {
            self.invalidate();
            return Frame {
                holes: &[],
                complex: &[],
                cuts: &[],
                cut_contours: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: Stats::default(),
                issues,
                plan: &self.empty_plan,
            };
        }

        // Étage 1 : germes.
        let spacing: Vec<Controller> = if p.layout.base_spacing().is_some() {
            p.controllers
                .iter()
                .filter(|c| c.spacing.is_some())
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let key = SeedsKey {
            layout: p.layout.clone(),
            plate: p.plate.clone(),
            spacing,
        };
        if self.seeds_key.as_ref() != Some(&key) {
            let field = Field::new(&key.spacing);
            let result = seeds(&p.layout, &p.plate, &field.channel(Channel::Spacing));
            match result {
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
                complex: &[],
                cuts: &[],
                cut_contours: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: Stats::default(),
                issues,
                plan: &self.empty_plan,
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
            features: p.features.clone(),
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
                complex: &[],
                cuts: &[],
                cut_contours: &[],
                metrics: Metrics {
                    plate_area: p.plate.area(),
                    ..Metrics::default()
                },
                stats: self.stats,
                issues,
                plan: &self.empty_plan,
            };
        }

        // Étage 3 : mesures.
        let key = MetricsKey {
            holes_gen: self.holes_gen,
            min_bridge: p.fabrication.min_bridge,
        };
        if self.metrics_key.as_ref() != Some(&key) {
            let (m, overlapping) =
                measure_with_overlaps(&self.holes, p.plate.area(), p.fabrication.min_bridge);
            self.metrics = m;
            self.overlapping = overlapping;
            self.metrics_key = Some(key);
            self.metrics_gen += 1;
            self.last_runs[2] = true;
        }

        // Étage 4 : découpe (fusion des chevauchements, coupe au bord).
        if self.cut_gen != self.metrics_gen {
            let (complex, cuts, exact) = build_cuts(p, &self.holes, &self.overlapping);
            self.complex = complex;
            self.cuts = cuts;
            self.cut_contours = exact;
            self.cut_gen = self.metrics_gen;
            self.last_runs[3] = true;
        }
        let simple_area: f64 = self
            .holes
            .iter()
            .zip(&self.complex)
            .filter(|(_, c)| !**c)
            .map(|(h, _)| h.geom.area)
            .sum();
        let cut_area: f64 = self.cuts.iter().map(Cut::area).sum();
        self.metrics.open_area = simple_area + cut_area;
        self.metrics.open_ratio = if self.metrics.plate_area > 0.0 {
            100.0 * self.metrics.open_area / self.metrics.plate_area
        } else {
            0.0
        };
        self.metrics.islands = self.cuts.iter().map(|c| c.islands.len()).sum();
        self.stats.merged = self.complex.iter().filter(|c| **c).count();

        let m = &self.metrics;
        if matches!(&p.layout, Layout::Path(l) if l.paths.iter().all(|c| c.points.len() < 2)) {
            issues.push(Issue::info("layout.no_path"));
        } else if self.holes.is_empty() {
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
        if self.stats.removed_feature > 0 {
            issues.push(Issue::info("holes.removed_feature").count(self.stats.removed_feature));
        }
        let plate = p.plate.region();
        let outside = p
            .features
            .iter()
            .filter(|f| f.cut)
            .filter(|f| {
                let r = feature_region(f);
                !plate.fits(&r.core, r.round, 0.0)
            })
            .count();
        if outside > 0 {
            issues.push(Issue::warning("features.outside").count(outside));
        }
        if m.overlaps > 0 {
            issues.push(Issue::info("holes.merged").count(m.overlaps));
        }
        if m.islands > 0 {
            issues.push(Issue::warning("holes.islands").count(m.islands));
        }
        if m.thin_bridges > 0 {
            let mut i = Issue::warning("bridge.thin")
                .count(m.thin_bridges)
                .field("fabrication.min_bridge");
            i.value = m.bridge_min;
            issues.push(i);
        }

        // Étage 5 : pièces de l'export STEP, pour vérifier en direct une
        // dépouille ou un découpage en dalles.
        let wanted = p.fabrication.draft != 0.0 || p.fabrication.tiles != [1, 1];
        let key = PlanKey {
            cut_gen: self.cut_gen,
            plate: p.plate.clone(),
            features: p.features.clone(),
            fabrication: p.fabrication.clone(),
        };
        if !wanted {
            self.plan = SolidPlan::default();
            self.plan_key = None;
        } else if self.plan_key.as_ref() != Some(&key) {
            self.plan = solid::plan(p, &self.holes, &self.complex, &self.cut_contours);
            self.plan_key = Some(key);
            self.last_runs[4] = true;
        }
        issues.extend(self.plan.issues.iter().cloned());

        Frame {
            holes: &self.holes,
            complex: &self.complex,
            cuts: &self.cuts,
            cut_contours: &self.cut_contours,
            metrics: self.metrics.clone(),
            stats: self.stats,
            issues,
            plan: &self.plan,
        }
    }

    fn invalidate(&mut self) {
        self.seeds_key = None;
        self.holes_key = None;
        self.metrics_key = None;
        self.cut_gen = u64::MAX;
    }
}

/// Région d'un élément (trou de fixation, zone réservée).
/// Contour exact d'un élément de plaque (congés d'un tracé en arcs).
pub fn feature_contour(f: &Feature) -> Contour {
    match &f.shape {
        FeatureShape::Curve(c) => crate::curve::contour(c),
        _ => feature_region(f).contour(),
    }
}

pub fn feature_region(f: &Feature) -> Region {
    match &f.shape {
        FeatureShape::Circle { center, diameter } => Region::circle(*center, diameter / 2.0),
        FeatureShape::Rectangle {
            center,
            width,
            height,
            corner,
        } => Region::rounded_rect(*center, *width, *height, *corner),
        FeatureShape::Curve(c) => Region::polygon(crate::curve::flatten(c, 0.02)),
    }
}

fn build_holes(p: &Project, seeds: &[Seed]) -> (Vec<Hole>, Stats) {
    let field = Field::new(&p.controllers);
    let size_field = field.channel(Channel::Size);
    let angle_field = field.channel(Channel::Angle);
    let margin = p.plate.margin;
    let plate = p.plate.region();
    let zone = p.plate.zone_region();
    let features: Vec<(Region, f64)> = p
        .features
        .iter()
        .map(|f| (feature_region(f), f.clearance))
        .collect();
    let clip = p.edge == EdgeMode::Clip;
    let mut stats = Stats {
        seeds: seeds.len(),
        ..Stats::default()
    };
    let mut holes = Vec::with_capacity(seeds.len());
    for s in seeds {
        // Tri rapide sur le centre : en découpe au bord, on garde tout centre
        // assez proche pour que le trou entre dans la zone.
        let reach = if clip { p.shape.size } else { 0.0 };
        if plate.sdf(s.pos) > reach - margin || zone.as_ref().is_some_and(|z| z.sdf(s.pos) > reach)
        {
            stats.removed_edge += 1;
            continue;
        }
        let size = size_field.value(p.shape.size, s.pos);
        if size < p.fabrication.min_size || size <= 0.0 {
            stats.removed_small += 1;
            continue;
        }
        if size_field.culled(s.pos) {
            stats.removed_cull += 1;
            continue;
        }
        let angle = angle_field.value(p.shape.rotation, s.pos) + s.angle;
        let geom = hole_geometry(&p.shape, size, angle, s.pos);
        if clip {
            // Hors de la zone, ou entièrement dans une zone réservée : rien à découper.
            if plate.sdf(s.pos) > geom.bound - margin
                || zone.as_ref().is_some_and(|z| z.sdf(s.pos) > geom.bound)
                || features
                    .iter()
                    .any(|(r, c)| r.sdf(s.pos) < -(geom.bound + c))
            {
                stats.removed_edge += 1;
                continue;
            }
        } else {
            if p.edge == EdgeMode::Whole
                && (!plate.fits(&geom.core, geom.round, margin)
                    || zone
                        .as_ref()
                        .is_some_and(|z| !z.fits(&geom.core, geom.round, 0.0)))
            {
                stats.removed_edge += 1;
                continue;
            }
            if features
                .iter()
                .any(|(r, c)| r.gap(&geom.core, geom.round) < *c)
            {
                stats.removed_feature += 1;
                continue;
            }
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

/// Trous à fusionner ou à couper, et leurs découpes.
/// Découpe aux contours exacts : arcs retrouvés sur les cercles connus.
#[derive(Clone, Debug, PartialEq)]
pub struct CutContour {
    pub outer: Contour,
    pub islands: Vec<Contour>,
}

/// Cercles sur lesquels une découpe peut s'appuyer : perforations
/// fusionnées, gardes des trous de fixation, zone perforée, plaque et
/// plaque réduite de la marge.
fn known_circles(p: &Project, holes: &[Hole], complex: &[bool]) -> Circles {
    let mut contours: Vec<Contour> = holes
        .iter()
        .zip(complex)
        .filter(|(_, c)| **c)
        .map(|(h, _)| h.contour())
        .collect();
    for f in &p.features {
        contours.push(feature_region(f).inflated(f.clearance).contour());
    }
    if let Some(z) = p.plate.zone_region() {
        contours.push(z.contour());
    }
    let plate = p.plate.region();
    contours.push(plate.contour());
    if plate.round > p.plate.margin {
        contours.push(
            Region {
                round: plate.round - p.plate.margin,
                ..plate
            }
            .contour(),
        );
    }
    Circles::from_contours(&contours)
}

fn build_cuts(
    p: &Project,
    holes: &[Hole],
    overlapping: &[bool],
) -> (Vec<bool>, Vec<Cut>, Vec<CutContour>) {
    let clip = p.edge == EdgeMode::Clip;
    let plate = p.plate.region();
    let zone = p.plate.zone_region();
    let features: Vec<(Region, f64)> = p
        .features
        .iter()
        .map(|f| (feature_region(f), f.clearance))
        .collect();
    let complex: Vec<bool> = holes
        .iter()
        .zip(overlapping)
        .map(|(h, &o)| {
            o || (clip
                && (!plate.fits(&h.geom.core, h.geom.round, p.plate.margin)
                    || zone
                        .as_ref()
                        .is_some_and(|z| !z.fits(&h.geom.core, h.geom.round, 0.0))
                    || features
                        .iter()
                        .any(|(r, c)| r.gap(&h.geom.core, h.geom.round) < *c)))
        })
        .collect();
    if !complex.iter().any(|c| *c) {
        return (complex, Vec::new(), Vec::new());
    }
    let polys: Vec<Vec<crate::geom::Vec2>> = holes
        .iter()
        .zip(&complex)
        .filter(|(_, c)| **c)
        .map(|(h, _)| h.contour().tessellated(CUT_TOLERANCE))
        .collect();
    let (area, keepouts) = if clip {
        let k = features
            .iter()
            .map(|(r, c)| r.inflated(*c).contour().tessellated(CUT_TOLERANCE))
            .collect();
        (p.plate.perforable(), k)
    } else {
        // Seulement des chevauchements : la plaque elle-même borne la fusion.
        (
            vec![p.plate.contour().tessellated(CUT_TOLERANCE)],
            Vec::new(),
        )
    };
    let cuts = cut(&polys, &area, &keepouts);
    let circles = known_circles(p, holes, &complex);
    let exact = cuts
        .iter()
        .map(|c| CutContour {
            outer: fit(&c.outer, &circles),
            islands: c.islands.iter().map(|i| fit(i, &circles)).collect(),
        })
        .collect();
    (complex, cuts, exact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{
        Concentric, CrossHatch, Cull, CullMode, Curve, Fibonacci, Geometry, Grid, GridPattern,
        Influence, PathLayout, Random, Spiral, Zone,
    };
    use std::f64::consts::PI;

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
        assert_eq!(pl.last_runs, [true, true, true, true, false]);
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, false, false, false, false]);
        if let Geometry::Point { at } = &mut p.controllers[0].geometry {
            at.x += 5.0;
        }
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, true, true, true, false]);
        p.fabrication.min_bridge = 2.0;
        pl.compute(&p);
        assert_eq!(pl.last_runs, [false, false, true, true, false]);
    }

    #[test]
    fn small_holes_are_removed_and_reported() {
        let mut p = square(10.0);
        p.controllers.push(Controller {
            size: Some(Influence {
                hard_edge: true,
                ..Influence::new(12.0, 0.0)
            }),
            ..Controller::new(
                1,
                Geometry::Point {
                    at: Vec2::new(50.0, 50.0),
                },
            )
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

    fn spacing_point() -> Controller {
        Controller {
            spacing: Some(Influence::new(30.0, 4.0)),
            ..Controller::new(
                9,
                Geometry::Point {
                    at: Vec2::new(50.0, 50.0),
                },
            )
        }
    }

    #[test]
    fn spacing_controllers_reseed_only_layouts_that_follow_them() {
        let mut grid = square(10.0);
        grid.controllers.push(spacing_point());
        let mut pl = Pipeline::new();
        pl.compute(&grid);
        if let Geometry::Point { at } = &mut grid.controllers[0].geometry {
            at.x += 5.0;
        }
        pl.compute(&grid);
        assert!(!pl.last_runs[0], "la grille ne suit pas l'espacement");

        let mut rnd = grid.clone();
        rnd.layout = Layout::Random(Random::default());
        pl.compute(&rnd);
        if let Geometry::Point { at } = &mut rnd.controllers[0].geometry {
            at.x += 5.0;
        }
        pl.compute(&rnd);
        assert!(pl.last_runs[0]);
    }

    #[test]
    fn every_layout_computes_a_sound_pattern() {
        let layouts = [
            Layout::Random(Random::default()),
            Layout::Spiral(Spiral::default()),
            Layout::Concentric(Concentric::default()),
            Layout::Fibonacci(Fibonacci::default()),
            Layout::CrossHatch(CrossHatch::default()),
            Layout::Path(PathLayout {
                paths: vec![Curve {
                    points: vec![Vec2::new(20.0, 20.0), Vec2::new(80.0, 80.0)],
                    closed: false,
                    fillets: vec![],
                }],
                ..PathLayout::default()
            }),
        ];
        for l in layouts {
            let mut p = square(10.0);
            p.shape.size = 3.0;
            p.layout = l.clone();
            p.controllers.push(spacing_point());
            let mut pl = Pipeline::new();
            let f = pl.compute(&p);
            assert!(!f.has_errors(), "{l:?} : {:?}", f.issues);
            assert!(f.holes.len() > 5, "{l:?}");
            assert!(f.holes.iter().all(|h| p.plate.sdf(h.center) <= 0.0));
            // Déterminisme : même projet, même motif.
            let again: Vec<Vec2> = Pipeline::new()
                .compute(&p)
                .holes
                .iter()
                .map(|h| h.center)
                .collect();
            assert_eq!(again, f.holes.iter().map(|h| h.center).collect::<Vec<_>>());
        }
    }

    #[test]
    fn empty_path_layout_says_so() {
        let mut p = square(10.0);
        p.layout = Layout::Path(PathLayout::default());
        let f = Pipeline::new().compute(&p).issues;
        assert!(f.iter().any(|i| i.code == "layout.no_path"));
    }

    #[test]
    fn cull_removes_a_share_of_holes_reproducibly() {
        let mut p = square(2.0);
        p.shape.size = 1.0;
        p.controllers.push(Controller {
            size: Some(Influence {
                cull: Some(Cull {
                    amount: 0.4,
                    mode: CullMode::Uniform,
                    seed: 11,
                }),
                ..Influence::new(1000.0, 1.0)
            }),
            ..Controller::new(
                1,
                Geometry::Point {
                    at: Vec2::new(50.0, 50.0),
                },
            )
        });
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let (kept, culled) = (f.holes.len(), f.stats.removed_cull);
        let share = culled as f64 / (kept + culled) as f64;
        assert!((share - 0.4).abs() < 0.03, "{share}");
        assert_eq!(Pipeline::new().compute(&p).stats.removed_cull, culled);
    }

    #[test]
    fn clip_mode_cuts_holes_at_the_edge_with_exact_area() {
        // Plaque 100 × 100 sans marge, trous de 6 mm au pas de 10 alignés sur le bord :
        // les 40 trous du bord sont coupés (demi- ou quart de disque).
        let mut p = square(10.0);
        p.shape.size = 6.0;
        p.edge = EdgeMode::Clip;
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        assert_eq!(f.holes.len(), 121);
        assert_eq!(f.complex.iter().filter(|c| **c).count(), 40);
        let disc = PI * 9.0;
        let expected = 81.0 * disc + 36.0 * disc / 2.0 + 4.0 * disc / 4.0;
        assert!(
            (f.metrics.open_area / expected - 1.0).abs() < 5e-4,
            "{} / {expected}",
            f.metrics.open_area
        );
        assert_eq!(f.cuts.len(), 40);
    }

    #[test]
    fn overlaps_are_merged_and_counted_once() {
        let mut p = square(4.0);
        p.shape.size = 5.0;
        p.edge = EdgeMode::Whole;
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        // Tous les trous se chevauchent : une seule grande découpe en forme de grille.
        assert!(f.complex.iter().all(|c| *c));
        let sum: f64 = f.holes.iter().map(|h| h.geom.area).sum();
        assert!(f.metrics.open_area < sum);
        assert!(
            f.metrics.islands > 0,
            "chaque maille entoure un îlot de matière"
        );
        assert!(f.issues.iter().any(|i| i.code == "holes.islands"));
    }

    #[test]
    fn zone_and_mounting_holes_restrict_holes() {
        let mut p = square(5.0);
        p.shape.size = 2.0;
        p.plate.zone = Zone::Circle {
            center: Vec2::new(50.0, 50.0),
            diameter: 60.0,
        };
        let mut pl = Pipeline::new();
        let n_zone = pl.compute(&p).holes.len();
        assert!(pl
            .compute(&p)
            .holes
            .iter()
            .all(|h| h.center.distance(Vec2::new(50.0, 50.0)) <= 29.0));
        p.features.push(Feature {
            id: 1,
            shape: FeatureShape::Circle {
                center: Vec2::new(50.0, 50.0),
                diameter: 10.0,
            },
            cut: true,
            clearance: 3.0,
        });
        let f = pl.compute(&p);
        assert!(f.holes.len() < n_zone);
        assert!(f.stats.removed_feature > 0);
        // Aucune perforation à moins de 3 mm du trou de fixation.
        assert!(f
            .holes
            .iter()
            .all(|h| h.center.distance(Vec2::new(50.0, 50.0)) - 1.0 - 5.0 >= 3.0 - 1e-9));
        assert!(!f.issues.iter().any(|i| i.code == "features.outside"));
        // Un trou de fixation à cheval sur le bord est signalé.
        p.features[0].shape = FeatureShape::Circle {
            center: Vec2::new(99.0, 50.0),
            diameter: 10.0,
        };
        let f = pl.compute(&p);
        assert!(f.issues.iter().any(|i| i.code == "features.outside"));
    }

    #[test]
    fn clip_mode_subtracts_reserved_zones() {
        let mut p = square(10.0);
        p.shape.size = 8.0;
        p.edge = EdgeMode::Clip;
        let mut pl = Pipeline::new();
        let base = pl.compute(&p).metrics.open_area;
        p.features.push(Feature {
            id: 1,
            shape: FeatureShape::Rectangle {
                center: Vec2::new(50.0, 50.0),
                width: 30.0,
                height: 30.0,
                corner: 0.0,
            },
            cut: false,
            clearance: 0.0,
        });
        let f = pl.compute(&p);
        assert!(f.metrics.open_area < base);
        assert!(!f.cuts.is_empty());
    }
}
