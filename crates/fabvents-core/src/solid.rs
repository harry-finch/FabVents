//! Pièces à extruder pour l'export STEP : plaque (ou dalles) percée de ses
//! trous, îlots détachés, parois en dépouille. Vérifie avant l'export ce
//! que la dépouille et le découpage en dalles rendraient impossible :
//!
//! - `draft.collapse` : une perforation se referme (ou perd son arrondi)
//!   avant le dessous de la plaque ;
//! - `draft.pierce` : au dessous, une perforation touche sa voisine, un
//!   trou de fixation ou le bord de la plaque ;
//! - `tiles.draft` : une coupe de dalles traverse des perforations alors
//!   que les parois sont en dépouille (section non plane).
//!
//! Ces messages sont des avertissements : ils bloquent l'export STEP, pas
//! le reste. Les perforations en cause sont renvoyées pour le canevas.

use crate::brep::{safe_cut, split, Axis, Loop};
use crate::export::step::{Solid, Wall};
use crate::geom::{core_distance, Vec2, DEG};
use crate::issue::Issue;
use crate::pipeline::{feature_contour, feature_region, CutContour, Hole};
use crate::project::Project;
use crate::region::Region;
use std::collections::HashMap;

/// Codes qui empêchent l'export STEP.
pub const BLOCKING: [&str; 3] = ["draft.collapse", "draft.pierce", "tiles.draft"];

#[derive(Clone, Debug, Default)]
pub struct Tile {
    pub col: u32,
    pub row: u32,
    pub solids: Vec<Solid>,
}

#[derive(Clone, Debug, Default)]
pub struct SolidPlan {
    /// Dalles (une seule sans découpage), de gauche à droite puis de bas en haut.
    pub tiles: Vec<Tile>,
    /// Lignes de coupe intérieures (x des colonnes, y des rangées).
    pub cuts_x: Vec<f64>,
    pub cuts_y: Vec<f64>,
    /// Contours (facettisés) des perforations en cause, pour le canevas.
    pub alerts: Vec<Vec<Vec2>>,
    pub issues: Vec<Issue>,
    pub thickness: f64,
}

impl SolidPlan {
    pub fn blocked(&self) -> bool {
        self.issues.iter().any(|i| BLOCKING.contains(&i.code))
    }
}

/// Une boucle de trou (dessus, dessous) et sa croissance au dessous.
struct Item {
    top: Loop,
    bottom: Option<Loop>,
    grow: f64,
}

impl Item {
    fn new(top: Loop, d: f64) -> Item {
        let bottom = top.offset_left(d);
        let grow = bottom.as_ref().map_or(0.0, |b| top.max_shift(b));
        Item { top, bottom, grow }
    }

    fn wall(&self) -> Option<Wall> {
        Some(Wall {
            top: self.top.clone(),
            bottom: self.bottom.clone()?,
        })
    }

    fn bbox(&self) -> (Vec2, Vec2) {
        let (mut lo, mut hi) = self.top.bbox();
        if let Some(b) = &self.bottom {
            let (l2, h2) = b.bbox();
            lo = Vec2::new(lo.x.min(l2.x), lo.y.min(l2.y));
            hi = Vec2::new(hi.x.max(h2.x), hi.y.max(h2.y));
        }
        (lo, hi)
    }
}

pub fn plan(p: &Project, holes: &[Hole], complex: &[bool], cuts: &[CutContour]) -> SolidPlan {
    let t = p.plate.thickness;
    let d = t * (p.fabrication.draft * DEG).tan();
    let mut plan = SolidPlan {
        thickness: t,
        ..SolidPlan::default()
    };
    let plate_region = p.plate.region();
    let outer = Loop::from_contour(&p.plate.contour()).oriented(true);

    // Boucles des trous (sens horaire) : perforations simples, découpes,
    // trous de fixation ; îlots (sens trigonométrique), pièces à part.
    let simple: Vec<usize> = (0..holes.len())
        .filter(|&i| !complex.get(i).copied().unwrap_or(false))
        .collect();
    let mut items: Vec<Item> = simple
        .iter()
        .map(|&i| Item::new(Loop::from_contour(&holes[i].contour()).oriented(false), d))
        .collect();
    let n_simple = items.len();
    items.extend(
        cuts.iter()
            .map(|c| Item::new(Loop::from_contour(&c.outer).oriented(false), d)),
    );
    let n_cuts = items.len();
    let features: Vec<_> = p.features.iter().filter(|f| f.cut).collect();
    items.extend(
        features
            .iter()
            .map(|f| Item::new(Loop::from_contour(&feature_contour(f)).oriented(false), d)),
    );
    let islands: Vec<Item> = cuts
        .iter()
        .flat_map(|c| &c.islands)
        .map(|i| Item::new(Loop::from_contour(i).oriented(true), d))
        .collect();

    // Dépouille : boucles qui se referment.
    let mut flagged = vec![false; items.len()];
    let mut collapse = 0;
    for (k, it) in items.iter().chain(&islands).enumerate() {
        if it.bottom.is_none() {
            collapse += 1;
            if k < items.len() {
                flagged[k] = true;
            } else {
                plan.alerts.push(it.top.tessellated(0.05));
            }
        }
    }

    // Dépouille qui élargit : parois percées au dessous.
    let mut pierce = vec![false; items.len()];
    if d > 0.0 {
        let cut_grow = items[n_simple..n_cuts]
            .iter()
            .map(|i| i.grow)
            .fold(0.0, f64::max);
        // Âme, arrondi et croissance de chaque perforation (les perforations
        // fusionnées prennent la croissance la plus forte des découpes).
        let shapes: Vec<(usize, &Hole, f64, bool)> = holes
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let is_complex = complex.get(i).copied().unwrap_or(false);
                let grow = if is_complex {
                    cut_grow
                } else {
                    items[simple.binary_search(&i).unwrap()].grow
                };
                (i, h, grow, is_complex)
            })
            .collect();
        let item_of =
            |i: usize, is_complex: bool| (!is_complex).then(|| simple.binary_search(&i).unwrap());
        let reach = shapes
            .iter()
            .map(|s| s.1.geom.bound + s.2)
            .fold(0.0, f64::max);
        let cell = (2.0 * reach).max(1e-3);
        let key = |c: Vec2| ((c.x / cell).floor() as i64, (c.y / cell).floor() as i64);
        let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (k, s) in shapes.iter().enumerate() {
            grid.entry(key(s.1.center)).or_default().push(k);
        }
        let hit = |i: usize, is_complex: bool, pierce: &mut Vec<bool>| match item_of(i, is_complex)
        {
            Some(k) => pierce[k] = true,
            None => {
                for v in &mut pierce[n_simple..n_cuts] {
                    *v = true;
                }
            }
        };
        for (k, &(i, a, ga, ca)) in shapes.iter().enumerate() {
            // Bord de la plaque.
            if !plate_region.fits(&a.geom.core, a.geom.round + ga, 0.0) {
                hit(i, ca, &mut pierce);
            }
            let (gx, gy) = key(a.center);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    for &m in grid.get(&(gx + dx, gy + dy)).map_or(&[][..], |v| v) {
                        if m <= k {
                            continue;
                        }
                        let (j, b, gb, cb) = shapes[m];
                        if ca && cb {
                            continue;
                        }
                        let gap =
                            core_distance(&a.geom.core, &b.geom.core) - a.geom.round - b.geom.round;
                        if gap > 1e-9 && gap - ga - gb <= 0.0 {
                            hit(i, ca, &mut pierce);
                            hit(j, cb, &mut pierce);
                        }
                    }
                }
            }
            // Trous de fixation.
            for (n, f) in features.iter().enumerate() {
                let fr: Region = feature_region(f);
                let gf = items[n_cuts + n].grow;
                let gap = fr.gap(&a.geom.core, a.geom.round);
                if gap > 1e-9 && gap - ga - gf <= 0.0 {
                    hit(i, ca, &mut pierce);
                    pierce[n_cuts + n] = true;
                }
            }
        }
        for (n, f) in features.iter().enumerate() {
            let fr = feature_region(f);
            if !plate_region.fits(&fr.core, fr.round + items[n_cuts + n].grow, 0.0) {
                pierce[n_cuts + n] = true;
            }
        }
    }
    let n_pierce = pierce.iter().filter(|v| **v).count();
    for (k, it) in items.iter().enumerate() {
        if flagged[k] || pierce[k] {
            plan.alerts.push(it.top.tessellated(0.05));
        }
    }
    if collapse > 0 {
        plan.issues.push(
            Issue::warning("draft.collapse")
                .count(collapse)
                .field("fabrication.draft"),
        );
    }
    if n_pierce > 0 {
        plan.issues.push(
            Issue::warning("draft.pierce")
                .count(n_pierce)
                .field("fabrication.draft"),
        );
    }

    // Dalles : lignes de coupe, de préférence entre les perforations.
    let [nx, ny] = p.fabrication.tiles;
    let ext = p.plate.extent();
    let boxes: Vec<(Vec2, Vec2)> = items.iter().chain(&islands).map(Item::bbox).collect();
    let keep = p.fabrication.min_bridge.max(1.0);
    let mut through = 0;
    for (axis, n, len) in [(Axis::X, nx, ext.x), (Axis::Y, ny, ext.y)] {
        let spans: Vec<(f64, f64)> = boxes
            .iter()
            .map(|(lo, hi)| match axis {
                Axis::X => (lo.x, hi.x),
                Axis::Y => (lo.y, hi.y),
            })
            .collect();
        for c in cut_positions(len, n, &spans, keep) {
            through += usize::from(c.through);
            match axis {
                Axis::X => plan.cuts_x.push(c.at),
                Axis::Y => plan.cuts_y.push(c.at),
            }
        }
    }
    // Une coupe à travers une paroi en dépouille n'aurait pas de section plane.
    if through > 0 && d != 0.0 {
        plan.issues.push(
            Issue::warning("tiles.draft")
                .count(through)
                .field("fabrication.draft"),
        );
    }

    if plan.blocked() {
        return plan;
    }

    // Boucles de la plaque : contour, îlots (étiquette = rang de l'îlot),
    // trous (étiquette = rang du trou), coupées le long des dalles.
    let all_top: Vec<Loop> = std::iter::once(outer.clone())
        .chain(islands.iter().map(|i| i.top.clone()))
        .chain(items.iter().map(|i| i.top.clone()))
        .collect();
    for (axis, cuts) in [(Axis::X, &mut plan.cuts_x), (Axis::Y, &mut plan.cuts_y)] {
        for c in cuts.iter_mut() {
            *c = safe_cut(&all_top, axis, *c);
        }
    }
    #[derive(Clone, Copy, PartialEq)]
    enum Tag {
        Outer,
        Island(usize),
        Hole(usize),
    }
    let tagged: Vec<(Loop, Option<Tag>)> = std::iter::once((outer, Some(Tag::Outer)))
        .chain(
            islands
                .iter()
                .enumerate()
                .map(|(k, i)| (i.top.clone(), Some(Tag::Island(k)))),
        )
        .chain(
            items
                .iter()
                .enumerate()
                .map(|(k, i)| (i.top.clone(), Some(Tag::Hole(k)))),
        )
        .collect();
    type Cell<T> = (u32, u32, Vec<(Loop, Option<T>)>);
    let mut cells: Vec<Cell<Tag>> = Vec::new();
    let mut rest = tagged;
    let mut columns = Vec::new();
    for &x in &plan.cuts_x {
        let (lower, upper) = split(&rest, Axis::X, x);
        columns.push(lower);
        rest = upper;
    }
    columns.push(rest);
    for (col, column) in columns.into_iter().enumerate() {
        let mut rest = column;
        for (row, &y) in plan.cuts_y.iter().enumerate() {
            let (lower, upper) = split(&rest, Axis::Y, y);
            cells.push((col as u32, row as u32, lower));
            rest = upper;
        }
        cells.push((col as u32, plan.cuts_y.len() as u32, rest));
    }

    // Chaque dalle : ses pièces (contours, îlots) et les trous qu'elles contiennent.
    for (col, row, loops) in cells {
        let wall_of = |l: &Loop, tag: Option<Tag>| match tag {
            Some(Tag::Island(k)) => islands[k].wall().expect("îlot vérifié"),
            Some(Tag::Hole(k)) => items[k].wall().expect("trou vérifié"),
            _ => Wall::vertical(l.clone()),
        };
        let mut solids: Vec<(f64, Solid)> = Vec::new();
        let mut holes = Vec::new();
        for (l, tag) in &loops {
            if l.signed_area() > 0.0 {
                solids.push((
                    l.signed_area(),
                    Solid {
                        outer: wall_of(l, *tag),
                        holes: Vec::new(),
                    },
                ));
            } else {
                holes.push(wall_of(l, *tag));
            }
        }
        // Le plus petit contour qui entoure le trou (un îlot avant la plaque).
        solids.sort_by(|a, b| a.0.total_cmp(&b.0));
        let boxes: Vec<(Vec2, Vec2)> = solids.iter().map(|s| s.1.outer.top.bbox()).collect();
        for h in holes {
            let at = h.top.pts[0];
            let target = (0..solids.len()).find(|&k| {
                let (lo, hi) = boxes[k];
                at.x >= lo.x
                    && at.x <= hi.x
                    && at.y >= lo.y
                    && at.y <= hi.y
                    && solids[k].1.outer.top.contains(at)
            });
            if let Some(k) = target {
                solids[k].1.holes.push(h);
            }
        }
        if !solids.is_empty() {
            plan.tiles.push(Tile {
                col,
                row,
                solids: solids.into_iter().map(|s| s.1).collect(),
            });
        }
    }
    plan.tiles.sort_by_key(|t| (t.row, t.col));
    plan
}

/// Une coupe intérieure : position, et passe-t-elle à travers des perforations ?
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutLine {
    pub at: f64,
    pub through: bool,
}

/// Positions des `n - 1` coupes intérieures sur `[0, len]`, à moins d'un
/// quart de dalle des coupes régulières. De préférence dans un intervalle
/// libre entre les `spans` occupés (en gardant `keep` mm de chaque côté
/// quand il le permet) ; sinon à travers les perforations, là où la droite
/// est le plus loin des bords des perforations (pas de copeau de matière).
pub fn cut_positions(len: f64, n: u32, spans: &[(f64, f64)], keep: f64) -> Vec<CutLine> {
    let mut out = Vec::new();
    if n <= 1 {
        return out;
    }
    let mut sorted: Vec<(f64, f64)> = spans.to_vec();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (a, b) in sorted {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let mut gaps = Vec::new();
    let mut at = 0.0;
    for &(a, b) in &merged {
        if a > at {
            gaps.push((at, a));
        }
        at = at.max(b);
    }
    if at < len {
        gaps.push((at, len));
    }
    let mut edges: Vec<f64> = spans.iter().flat_map(|&(a, b)| [a, b]).collect();
    edges.push(0.0);
    edges.push(len);
    edges.sort_by(f64::total_cmp);
    let tile = len / n as f64;
    for k in 1..n {
        let ideal = tile * k as f64;
        let (lo, hi) = (ideal - tile / 4.0, ideal + tile / 4.0);
        let free = gaps
            .iter()
            .filter(|(a, b)| b - a > 1e-6 && *b > lo && *a < hi)
            .map(|&(a, b)| {
                let h = ((b - a) / 2.0).min(keep);
                ideal.clamp(a + h, b - h)
            })
            .filter(|c| (lo..=hi).contains(c))
            .min_by(|x, y| (x - ideal).abs().total_cmp(&(y - ideal).abs()));
        let line = match free {
            Some(at) => CutLine { at, through: false },
            None => {
                // Milieu de l'intervalle le plus large entre deux bords de
                // perforations, dans la fenêtre.
                let mut best = (f64::NEG_INFINITY, ideal);
                for w in edges.windows(2) {
                    let (a, b) = (w[0].max(lo), w[1].min(hi));
                    if b > a {
                        let score = (b - a) / 2.0 - 1e-3 * ((a + b) / 2.0 - ideal).abs();
                        if score > best.0 {
                            best = (score, (a + b) / 2.0);
                        }
                    }
                }
                CutLine {
                    at: best.1,
                    through: true,
                }
            }
        };
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::project::{Feature, FeatureShape, Grid, Layout};
    use std::f64::consts::PI;

    fn grid_plate() -> Project {
        let mut p = Project::default();
        p.plate.width = 100.0;
        p.plate.height = 60.0;
        p.plate.corner_radius = 4.0;
        p.plate.margin = 5.0;
        p.plate.thickness = 3.0;
        p.layout = Layout::Grid(Grid {
            pattern: crate::project::GridPattern::Square,
            pitch_x: 10.0,
            pitch_y: 10.0,
            ..Grid::default()
        });
        p.shape.size = 6.0;
        p
    }

    fn run(p: &Project) -> SolidPlan {
        let mut pl = Pipeline::new();
        let f = pl.compute(p);
        plan(p, f.holes, f.complex, f.cut_contours)
    }

    #[test]
    fn cuts_prefer_gaps_then_avoid_slivers() {
        let spans = [(1.0, 4.5), (5.5, 9.0)];
        let c = cut_positions(10.0, 2, &spans, 1.0);
        assert_eq!(
            c,
            vec![CutLine {
                at: 5.0,
                through: false
            }]
        );
        // Perforations en quinconce : aucune coupe libre ; dans la fenêtre
        // [7,5 ; 12,5], l'intervalle le plus large entre bords est [7,5 ; 10].
        let spans = [(0.0, 6.0), (5.0, 11.0), (10.0, 16.0), (15.0, 20.0)];
        let c = cut_positions(20.0, 2, &spans, 1.0);
        assert_eq!(
            c,
            vec![CutLine {
                at: 8.75,
                through: true
            }]
        );
    }

    #[test]
    fn plain_plate_is_one_solid_with_all_holes() {
        let p = grid_plate();
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let n = f.holes.len();
        let s = plan(&p, f.holes, f.complex, f.cut_contours);
        assert!(s.issues.is_empty());
        assert_eq!(s.tiles.len(), 1);
        assert_eq!(s.tiles[0].solids.len(), 1);
        assert_eq!(s.tiles[0].solids[0].holes.len(), n);
    }

    #[test]
    fn tiles_keep_every_hole_whole() {
        let mut p = grid_plate();
        p.fabrication.tiles = [3, 2];
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let n = f.holes.len();
        let s = plan(&p, f.holes, f.complex, f.cut_contours);
        assert!(s.issues.is_empty(), "{:?}", s.issues);
        assert_eq!((s.cuts_x.len(), s.cuts_y.len()), (2, 1));
        assert_eq!(s.tiles.len(), 6);
        let holes: usize = s
            .tiles
            .iter()
            .flat_map(|t| &t.solids)
            .map(|s| s.holes.len())
            .sum();
        assert_eq!(holes, n);
        // Les dalles recouvrent exactement la plaque.
        let area: f64 = s
            .tiles
            .iter()
            .flat_map(|t| &t.solids)
            .map(|s| s.outer.top.signed_area())
            .sum();
        assert!((area - p.plate.area()).abs() < 1e-6, "{area}");
    }

    #[test]
    fn staggered_holes_are_cut_into_notches() {
        let mut p = grid_plate();
        p.layout = Layout::Grid(Grid {
            pitch_x: 10.0,
            pitch_y: 10.0,
            pattern: crate::project::GridPattern::Hexagonal,
            ..Grid::default()
        });
        p.fabrication.tiles = [2, 2];
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let open = f.metrics.open_area;
        let s = plan(&p, f.holes, f.complex, f.cut_contours);
        assert!(s.issues.is_empty(), "{:?}", s.issues);
        assert_eq!(s.tiles.len(), 4);
        // Matière totale : plaque moins perforations, au micron près.
        let material: f64 = s
            .tiles
            .iter()
            .flat_map(|t| &t.solids)
            .map(|s| {
                s.outer.top.signed_area() + s.holes.iter().map(|h| h.top.signed_area()).sum::<f64>()
            })
            .sum();
        assert!(
            (material - (p.plate.area() - open)).abs() < 1e-3,
            "{material}"
        );
        // Avec une dépouille, une coupe à travers une perforation est refusée.
        p.fabrication.draft = 5.0;
        let s = run(&p);
        assert!(s.issues.iter().any(|i| i.code == "tiles.draft"));
    }

    #[test]
    fn draft_widens_and_checks_bridges() {
        let mut p = grid_plate();
        p.fabrication.draft = 10.0;
        let s = run(&p);
        assert!(s.issues.is_empty(), "{:?}", s.issues);
        let hole = &s.tiles[0].solids[0].holes[0];
        let d = 3.0 * (10.0 * DEG).tan();
        let top = -hole.top.signed_area();
        let bottom = -hole.bottom.signed_area();
        assert!((bottom - PI * (3.0 + d).powi(2)).abs() < 1e-9);
        assert!((top - PI * 9.0).abs() < 1e-9);
        // Nervure de 4 mm : percée au-delà de 2 mm de croissance par trou.
        p.fabrication.draft = 40.0;
        let s = run(&p);
        assert!(s.issues.iter().any(|i| i.code == "draft.pierce"));
        assert!(!s.alerts.is_empty());
    }

    #[test]
    fn negative_draft_closes_small_holes() {
        let mut p = grid_plate();
        p.shape.size = 1.0;
        p.fabrication.draft = -30.0;
        let s = run(&p);
        assert!(s.issues.iter().any(|i| i.code == "draft.collapse"));
        assert!(s.blocked());
    }

    #[test]
    fn mounting_hole_is_a_hole_of_the_plate() {
        let mut p = grid_plate();
        p.features.push(Feature {
            id: 1,
            shape: FeatureShape::Circle {
                center: Vec2::new(50.0, 30.0),
                diameter: 8.0,
            },
            cut: true,
            clearance: 2.0,
        });
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let n = f.holes.len();
        let s = plan(&p, f.holes, f.complex, f.cut_contours);
        assert_eq!(s.tiles[0].solids[0].holes.len(), n + 1);
    }
}
