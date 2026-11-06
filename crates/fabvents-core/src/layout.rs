//! Dispositions : placement des germes (centres des perforations), sans
//! connaître la forme. Les dispositions à espacement libre (aléatoire,
//! spirale, concentrique, Fibonacci, chemin) suivent le canal Espacement.

use crate::curve::{densify, flatten, offset, place_along};
use crate::field::{ChannelField, Field};
use crate::geom::{Vec2, DEG};
use crate::issue::{Issue, Severity};
use crate::project::{
    Channel, Concentric, CrossHatch, Feature, Fibonacci, Flow, Grid, GridPattern, Layout,
    PathLayout, Plate, Random, Spiral, Voronoi,
};
use crate::rng::Rng;
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

/// Nombre maximal de positions candidates explorées par une disposition.
pub const MAX_CANDIDATES: usize = 400_000;
/// Espacement local minimal accepté (mm).
pub const MIN_SPACING: f64 = 0.2;
/// Cases maximales de la grille d'accélération du tirage aléatoire.
const MAX_CELLS: f64 = 8.0e6;
/// Sommets maximaux des tracés denses (spirale, anneaux, chemins).
const MAX_DENSE: usize = 2_000_000;

/// Position d'un germe et angle propre à la disposition (degrés). `shape` :
/// rang de son contour dans [`Seeds::shapes`] quand la disposition donne la
/// forme (cellule de Voronoï, ligne fluide).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub pos: Vec2,
    pub angle: f64,
    pub shape: Option<u32>,
}

/// Germes d'une disposition et contours qu'elle impose.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seeds {
    pub seeds: Vec<Seed>,
    /// Cellules de Voronoï (polygones convexes) ou lignes fluides (polylignes).
    pub shapes: Vec<Vec<Vec2>>,
}

pub fn seeds(layout: &Layout, plate: &Plate, spacing: &ChannelField) -> Result<Vec<Seed>, Issue> {
    Ok(layout_seeds(layout, plate, &[], spacing, &Field::new(&[]))?.seeds)
}

/// Germes et contours imposés. `spacing` : canal Espacement ; `field` :
/// tous les contrôleurs (les lignes fluides suivent aussi les canaux Angle
/// et Taille) ; `features` : trous de fixation que contournent les lignes.
pub fn layout_seeds(
    layout: &Layout,
    plate: &Plate,
    features: &[Feature],
    spacing: &ChannelField,
    field: &Field,
) -> Result<Seeds, Issue> {
    let raw = match layout {
        Layout::Voronoi(v) => return voronoi(v, plate, spacing),
        Layout::Flow(f) => return flow(f, plate, features, spacing, field),
        Layout::CrossHatch(c) => cross_hatch(c, plate)?,
        Layout::Random(r) => random(r, plate, spacing)?,
        Layout::Spiral(s) => spiral(s, plate, spacing)?,
        Layout::Concentric(c) => concentric(c, plate, spacing)?,
        Layout::Fibonacci(f) => fibonacci(f, plate, spacing)?,
        Layout::Path(p) => path(p, spacing)?,
        Layout::Grid(g) => {
            return Ok(Seeds {
                seeds: grid(g, plate)?,
                shapes: Vec::new(),
            })
        }
    };
    Ok(Seeds {
        seeds: inside_extent(raw, plate),
        shapes: Vec::new(),
    })
}

/// Voronoï : germes Poisson-disc, relaxation de Lloyd pondérée par la
/// densité voulue (`1 / espacement²`), cellules bornées par la boîte de la
/// zone perforable : au bord, la matière vaut la marge plus une demi-nervure.
fn voronoi(v: &Voronoi, plate: &Plate, field: &ChannelField) -> Result<Seeds, Issue> {
    let rnd = Random {
        spacing: v.spacing,
        seed: v.seed,
    };
    let (sp, lo_s, _) = spacing_fn(field, v.spacing);
    let area = plate.perforable();
    let (mut lo, mut hi) = (
        Vec2::new(f64::INFINITY, f64::INFINITY),
        Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in area.iter().flatten() {
        lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    if !(lo.x < hi.x && lo.y < hi.y) {
        return Ok(Seeds::default());
    }
    let inside = |p: &Vec2| p.x >= lo.x && p.y >= lo.y && p.x <= hi.x && p.y <= hi.y;
    let mut pts: Vec<Vec2> = random(&rnd, plate, field)?
        .into_iter()
        .map(|s| s.pos)
        .filter(inside)
        .collect();
    let weight = |q: Vec2| {
        let s = sp(q);
        1.0 / (s * s)
    };
    for _ in 0..v.relax {
        let cells = crate::voronoi::cells(&pts, lo, hi, lo_s);
        pts = cells
            .iter()
            .zip(&pts)
            .map(|(c, &p)| {
                if c.len() >= 3 {
                    crate::voronoi::weighted_centroid(c, &weight)
                } else {
                    p
                }
            })
            .collect();
    }
    let cells = crate::voronoi::cells(&pts, lo, hi, lo_s);
    let mut out = Seeds::default();
    for c in cells.into_iter().filter(|c| c.len() >= 3) {
        let pos = crate::voronoi::weighted_centroid(&c, &|_| 1.0);
        out.seeds.push(Seed {
            pos,
            angle: 0.0,
            shape: Some(out.shapes.len() as u32),
        });
        out.shapes.push(c);
    }
    Ok(out)
}

/// Lignes fluides : lignes de courant et leur point milieu comme germe.
fn flow(
    f: &Flow,
    plate: &Plate,
    features: &[Feature],
    spacing: &ChannelField,
    field: &Field,
) -> Result<Seeds, Issue> {
    let (sp, _, _) = spacing_fn(spacing, f.spacing);
    let region = plate.region();
    let zone = plate.zone_region();
    let keepouts: Vec<(crate::region::Region, f64)> = features
        .iter()
        .map(|ft| (crate::pipeline::feature_region(ft), ft.clearance))
        .collect();
    let bounds = crate::flow::Bounds {
        plate: &region,
        margin: plate.margin,
        zone: zone.as_ref(),
        keepouts: &keepouts,
    };
    let size = field.channel(Channel::Size);
    let width = |p: Vec2| size.value(f.width, p).max(0.0);
    let lines = crate::flow::streamlines(
        f,
        plate.center(),
        0.5 * plate.extent().length(),
        &bounds,
        &sp,
        &field.channel(Channel::Angle),
        &width,
    )
    .map_err(|n| too_many("layout.spacing", n))?;
    let mut out = Seeds::default();
    for l in lines {
        out.seeds.push(Seed {
            pos: l[l.len() / 2],
            angle: 0.0,
            shape: Some(out.shapes.len() as u32),
        });
        out.shapes.push(l);
    }
    Ok(out)
}

fn too_many(field: &str, n: usize) -> Issue {
    Issue::new("layout.too_many_holes", Severity::Error)
        .field(field)
        .count(n)
}

/// Garde les germes dans l'emprise de la plaque.
fn inside_extent(seeds: Vec<Seed>, plate: &Plate) -> Vec<Seed> {
    let ext = plate.extent();
    seeds
        .into_iter()
        .filter(|s| s.pos.x >= 0.0 && s.pos.y >= 0.0 && s.pos.x <= ext.x && s.pos.y <= ext.y)
        .collect()
}

/// Origine d'une disposition centrée (centre de la plaque + décalage) et
/// rayon à couvrir pour atteindre tous les coins.
fn center_and_reach(plate: &Plate, ox: f64, oy: f64, margin: f64) -> (Vec2, f64) {
    let off = Vec2::new(ox, oy);
    (
        plate.center() + off,
        0.5 * plate.extent().length() + off.length() + margin,
    )
}

/// Espacement local du canal Espacement, borné à `[lo, hi]`.
fn spacing_fn<'a>(field: &'a ChannelField, base: f64) -> (impl Fn(Vec2) -> f64 + 'a, f64, f64) {
    let (lo, hi) = field.bounds(base);
    let lo = lo.max(MIN_SPACING);
    let hi = hi.max(lo);
    (move |p: Vec2| field.value(base, p).clamp(lo, hi), lo, hi)
}

fn along(
    pts: &[Vec2],
    closed: bool,
    s: &dyn Fn(Vec2) -> f64,
    orient: bool,
    out: &mut Vec<Seed>,
    field: &str,
) -> Result<(), Issue> {
    let left = MAX_CANDIDATES.saturating_sub(out.len());
    let placed =
        place_along(pts, closed, s, 0.0, left).map_err(|n| too_many(field, out.len() + n))?;
    out.extend(placed.into_iter().map(|q| Seed {
        shape: None,
        pos: q.pos,
        angle: if orient { q.angle } else { 0.0 },
    }));
    Ok(())
}

/// Poisson-disc à rayon variable (Bridson généralisé) : deux points restent
/// au moins à la moyenne de leurs espacements locaux.
fn random(r: &Random, plate: &Plate, field: &ChannelField) -> Result<Vec<Seed>, Issue> {
    let (spacing, lo, hi) = spacing_fn(field, r.spacing);
    let ext = plate.extent();
    let cell = lo * FRAC_1_SQRT_2;
    let (gw, gh) = ((ext.x / cell).ceil() + 1.0, (ext.y / cell).ceil() + 1.0);
    if gw * gh > MAX_CELLS {
        return Err(too_many(
            "layout.spacing",
            (ext.x * ext.y / (lo * lo)) as usize,
        ));
    }
    let (gw, gh) = (gw as usize, gh as usize);
    let mut grid = vec![u32::MAX; gw * gh];
    let mut pts: Vec<Vec2> = Vec::new();
    let mut radii: Vec<f64> = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    let mut rng = Rng::new(r.seed);
    let cell_of = |p: Vec2| ((p.x / cell) as usize, (p.y / cell) as usize);

    let insert = |p: Vec2,
                  rad: f64,
                  grid: &mut Vec<u32>,
                  pts: &mut Vec<Vec2>,
                  radii: &mut Vec<f64>,
                  active: &mut Vec<usize>| {
        let (cx, cy) = cell_of(p);
        grid[cy * gw + cx] = pts.len() as u32;
        active.push(pts.len());
        pts.push(p);
        radii.push(rad);
    };
    let start = plate.center();
    insert(
        start,
        spacing(start),
        &mut grid,
        &mut pts,
        &mut radii,
        &mut active,
    );

    const ATTEMPTS: usize = 30;
    while !active.is_empty() {
        let k = (rng.unit() * active.len() as f64) as usize % active.len();
        let parent = active[k];
        let (pp, pr) = (pts[parent], radii[parent]);
        let mut placed = false;
        for _ in 0..ATTEMPTS {
            let q = pp + Vec2::from_angle(rng.range(0.0, TAU)) * rng.range(pr, 2.0 * pr);
            if q.x < 0.0 || q.y < 0.0 || q.x >= ext.x || q.y >= ext.y {
                continue;
            }
            let qr = spacing(q);
            let reach = ((0.5 * (qr + hi)) / cell).ceil() as isize;
            let (cx, cy) = cell_of(q);
            let (cx, cy) = (cx as isize, cy as isize);
            let mut ok = true;
            'search: for gy in (cy - reach).max(0)..=(cy + reach).min(gh as isize - 1) {
                for gx in (cx - reach).max(0)..=(cx + reach).min(gw as isize - 1) {
                    let n = grid[gy as usize * gw + gx as usize];
                    if n != u32::MAX {
                        let n = n as usize;
                        let min = 0.5 * (qr + radii[n]);
                        if q.distance(pts[n]) < min {
                            ok = false;
                            break 'search;
                        }
                    }
                }
            }
            if ok {
                insert(q, qr, &mut grid, &mut pts, &mut radii, &mut active);
                placed = true;
                if pts.len() > MAX_CANDIDATES {
                    return Err(too_many("layout.spacing", pts.len()));
                }
                break;
            }
        }
        if !placed {
            active.swap_remove(k);
        }
    }
    Ok(pts
        .into_iter()
        .map(|pos| Seed {
            pos,
            angle: 0.0,
            shape: None,
        })
        .collect())
}

/// Spirale d'Archimède `r = départ + pas · θ / 2π`.
fn spiral(s: &Spiral, plate: &Plate, field: &ChannelField) -> Result<Vec<Seed>, Issue> {
    let (spacing, lo, hi) = spacing_fn(field, s.spacing);
    let (o, reach) = center_and_reach(plate, s.offset_x, s.offset_y, hi);
    let b = s.pitch / TAU;
    let ds = (lo / 4.0).max(0.25);
    let length =
        std::f64::consts::PI * (reach * reach - s.start * s.start).max(0.0) / s.pitch + reach;
    if length / ds > MAX_DENSE as f64 {
        return Err(too_many("layout.pitch", (length / lo) as usize));
    }
    let dir = if s.clockwise { -1.0 } else { 1.0 };
    let mut pts = Vec::new();
    let mut theta = 0.0f64;
    loop {
        let r = s.start + b * theta;
        pts.push(o + Vec2::from_angle(dir * theta) * r);
        if r > reach {
            break;
        }
        theta += ds / (r * r + b * b).sqrt().max(1e-6);
    }
    let mut out = Vec::new();
    along(&pts, false, &spacing, s.orient, &mut out, "layout.spacing")?;
    Ok(out)
}

/// Anneaux `r = départ + k · pas`, chacun tourné de `k · phase`.
fn concentric(c: &Concentric, plate: &Plate, field: &ChannelField) -> Result<Vec<Seed>, Issue> {
    let (spacing, lo, hi) = spacing_fn(field, c.spacing);
    let (o, reach) = center_and_reach(plate, c.offset_x, c.offset_y, hi);
    let ds = (lo / 4.0).max(0.25);
    let rings = ((reach - c.start) / c.pitch).floor().max(0.0) as usize + 1;
    let dense: f64 = (0..rings)
        .map(|k| TAU * (c.start + k as f64 * c.pitch) / ds)
        .sum();
    if dense > MAX_DENSE as f64 {
        return Err(too_many("layout.pitch", (dense * ds / lo) as usize));
    }
    let mut out = Vec::new();
    for k in 0..rings {
        let r = c.start + k as f64 * c.pitch;
        if r < 1e-9 {
            out.push(Seed {
                pos: o,
                angle: 0.0,
                shape: None,
            });
            continue;
        }
        let n = ((TAU * r / ds).ceil() as usize).max(24);
        let a0 = k as f64 * c.phase * DEG;
        let ring: Vec<Vec2> = (0..n)
            .map(|i| o + Vec2::from_angle(a0 + TAU * i as f64 / n as f64) * r)
            .collect();
        let first = out.len();
        along(&ring, true, &spacing, c.orient, &mut out, "layout.spacing")?;
        // Points ramenés sur le cercle exact (la polyligne passe sous les arcs).
        for q in &mut out[first..] {
            q.pos = o + (q.pos - o).normalized() * r;
        }
    }
    Ok(out)
}

/// Phyllotaxie : le point `k` tourne de `k · divergence` et son rayon
/// suit l'aire déjà occupée, chaque point comptant `√3/2 · espacement²`
/// (la densité d'un pavage hexagonal du même espacement).
fn fibonacci(f: &Fibonacci, plate: &Plate, field: &ChannelField) -> Result<Vec<Seed>, Issue> {
    let (spacing, _, hi) = spacing_fn(field, f.spacing);
    let (o, reach) = center_and_reach(plate, f.offset_x, f.offset_y, hi);
    let cell = |s: f64| 3f64.sqrt() / 2.0 * s * s;
    let mut out = Vec::new();
    let mut area = 0.0;
    let mut last = cell(spacing(o));
    for k in 0.. {
        let r = ((area + 0.5 * last) / std::f64::consts::PI).sqrt();
        if r > reach {
            break;
        }
        let a = k as f64 * f.divergence * DEG;
        let pos = o + Vec2::from_angle(a) * r;
        last = cell(spacing(pos));
        area += last;
        out.push(Seed {
            pos,
            angle: if f.orient { a.to_degrees() } else { 0.0 },
            shape: None,
        });
        if out.len() > MAX_CANDIDATES {
            return Err(too_many("layout.spacing", out.len()));
        }
    }
    Ok(out)
}

/// Treillis : nœuds `o + i·step·u + j·line_spacing·w` (`u` à `angle`, `w` à
/// `angle + cross`). Les formes de la première famille sont au milieu des
/// arêtes selon `u`, celles de la seconde au milieu des arêtes selon `w` ;
/// la matière reste aux nœuds. Chaque forme suit son arête.
fn cross_hatch(c: &CrossHatch, plate: &Plate) -> Result<Vec<Seed>, Issue> {
    let (o, reach) = center_and_reach(plate, c.offset_x, c.offset_y, c.step.max(c.line_spacing));
    let sin = (c.cross * DEG).sin().abs().max(0.05);
    let ni = (reach / (c.step * sin)).ceil() as i64 + 1;
    let nj = (reach / (c.line_spacing * sin)).ceil() as i64 + 1;
    let candidates = 2.0 * (2 * ni + 1) as f64 * (2 * nj + 1) as f64;
    if candidates > MAX_CANDIDATES as f64 {
        return Err(too_many("layout.step", candidates as usize));
    }
    let u = Vec2::from_angle(c.angle * DEG) * c.step;
    let w = Vec2::from_angle((c.angle + c.cross) * DEG) * c.line_spacing;
    let mut out = Vec::new();
    for j in -nj..=nj {
        for i in -ni..=ni {
            let node = o + u * i as f64 + w * j as f64;
            out.push(Seed {
                shape: None,
                pos: node + u * 0.5,
                angle: c.angle,
            });
            out.push(Seed {
                shape: None,
                pos: node + w * 0.5,
                angle: c.angle + c.cross,
            });
        }
    }
    Ok(out)
}

/// Rangées parallèles le long des tracés.
fn path(p: &PathLayout, field: &ChannelField) -> Result<Vec<Seed>, Issue> {
    let (spacing, lo, _) = spacing_fn(field, p.spacing);
    let ds = (lo / 4.0).max(0.25);
    let rows = p.rows.max(1);
    let mut out = Vec::new();
    for curve in &p.paths {
        if curve.points.len() < 2 {
            continue;
        }
        let flat = flatten(curve, 0.05);
        let dense = densify(&flat, curve.closed, ds);
        if dense.len() * rows as usize > MAX_DENSE {
            return Err(too_many("layout.spacing", dense.len()));
        }
        for r in 0..rows {
            let d = (r as f64 - (rows - 1) as f64 / 2.0) * p.row_gap;
            let row = offset(&dense, curve.closed, d);
            along(
                &row,
                curve.closed,
                &spacing,
                p.orient,
                &mut out,
                "layout.spacing",
            )?;
        }
    }
    Ok(out)
}

/// Grille centrée sur la plaque (puis décalée) : le motif reste symétrique
/// quand on change les dimensions.
pub(crate) fn grid(g: &Grid, plate: &Plate) -> Result<Vec<Seed>, Issue> {
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
                shape: None,
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

    use crate::field::Field;
    use crate::project::{Controller, Curve, Geometry, Influence};

    fn run(layout: &Layout, plate: &Plate, ctrls: &[Controller]) -> Vec<Seed> {
        let f = Field::new(ctrls);
        seeds(layout, plate, &f.channel(crate::project::Channel::Spacing)).unwrap()
    }

    fn min_distance(s: &[Seed]) -> f64 {
        let mut best = f64::INFINITY;
        for i in 0..s.len() {
            for j in i + 1..s.len() {
                best = best.min(s[i].pos.distance(s[j].pos));
            }
        }
        best
    }

    #[test]
    fn random_is_reproducible_and_respects_spacing() {
        let p = plate(100.0, 80.0);
        let l = Layout::Random(Random {
            spacing: 6.0,
            seed: 4,
        });
        let a = run(&l, &p, &[]);
        let b = run(&l, &p, &[]);
        assert_eq!(a, b);
        assert!(min_distance(&a) >= 6.0 - 1e-9);
        // Poisson-disc : densité proche de 1 point pour 1,1 à 1,6 × s².
        let per = 100.0 * 80.0 / a.len() as f64;
        assert!(per > 36.0 && per < 70.0, "{per}");
        let c = run(
            &Layout::Random(Random {
                spacing: 6.0,
                seed: 5,
            }),
            &p,
            &[],
        );
        assert_ne!(a, c);
    }

    #[test]
    fn spacing_controller_densifies_random_layout() {
        let p = plate(100.0, 100.0);
        let dense = Controller {
            spacing: Some(Influence {
                hard_edge: true,
                ..Influence::new(25.0, 3.0)
            }),
            ..Controller::new(
                1,
                Geometry::Point {
                    at: Vec2::new(25.0, 50.0),
                },
            )
        };
        let s = run(
            &Layout::Random(Random {
                spacing: 9.0,
                seed: 1,
            }),
            &p,
            &[dense],
        );
        let near = s
            .iter()
            .filter(|q| q.pos.distance(Vec2::new(25.0, 50.0)) < 20.0)
            .count();
        let far = s
            .iter()
            .filter(|q| q.pos.distance(Vec2::new(75.0, 50.0)) < 20.0)
            .count();
        assert!(near > 4 * far, "{near} / {far}");
    }

    #[test]
    fn spiral_points_follow_the_spiral() {
        let p = plate(100.0, 100.0);
        let sp = Spiral {
            pitch: 10.0,
            spacing: 5.0,
            ..Spiral::default()
        };
        let s = run(&Layout::Spiral(sp), &p, &[]);
        assert!(s.len() > 100);
        // Espacement le long de la spirale : voisins successifs à ~5 mm.
        let d = s[40].pos.distance(s[41].pos);
        assert!((d - 5.0).abs() < 0.2, "{d}");
        // Le rayon croît de 10 mm par tour.
        let r = |q: &Seed| q.pos.distance(Vec2::new(50.0, 50.0));
        assert!(s.windows(2).all(|w| r(&w[1]) >= r(&w[0]) - 1e-9));
    }

    #[test]
    fn concentric_rings_have_a_centre_and_even_counts() {
        let p = plate(100.0, 100.0);
        let s = run(
            &Layout::Concentric(Concentric {
                pitch: 10.0,
                spacing: 2.0 * std::f64::consts::PI,
                ..Concentric::default()
            }),
            &p,
            &[],
        );
        let ring = |r: f64| {
            s.iter()
                .filter(|q| (q.pos.distance(Vec2::new(50.0, 50.0)) - r).abs() < 1e-6)
                .count()
        };
        assert_eq!(ring(0.0), 1);
        // Anneau de rayon 10 : circonférence 20π, espacement 2π → 10 points.
        assert_eq!(ring(10.0), 10);
        assert_eq!(ring(20.0), 20);
    }

    #[test]
    fn fibonacci_density_matches_its_spacing() {
        let p = plate(200.0, 200.0);
        let s = run(
            &Layout::Fibonacci(Fibonacci {
                spacing: 6.0,
                ..Fibonacci::default()
            }),
            &p,
            &[],
        );
        let inner = s
            .iter()
            .filter(|q| q.pos.distance(Vec2::new(100.0, 100.0)) < 80.0)
            .count() as f64;
        let expected = std::f64::consts::PI * 80.0 * 80.0 / (3f64.sqrt() / 2.0 * 36.0);
        assert!(
            (inner / expected - 1.0).abs() < 0.03,
            "{inner} / {expected}"
        );
        assert!(min_distance(&s[..300]) > 4.0);
    }

    #[test]
    fn cross_hatch_has_two_orientations() {
        let p = plate(100.0, 100.0);
        let s = run(&Layout::CrossHatch(CrossHatch::default()), &p, &[]);
        let a = s.iter().filter(|q| q.angle == 45.0).count();
        let b = s.iter().filter(|q| q.angle == 135.0).count();
        assert!(a > 50 && b > 50 && a + b == s.len());
    }

    #[test]
    fn path_rows_follow_the_curve() {
        let p = plate(100.0, 100.0);
        let l = Layout::Path(PathLayout {
            paths: vec![Curve {
                points: vec![Vec2::new(10.0, 50.0), Vec2::new(90.0, 50.0)],
                closed: false,
                fillets: vec![],
            }],
            spacing: 10.0,
            rows: 3,
            row_gap: 6.0,
            orient: true,
        });
        let s = run(&l, &p, &[]);
        // 80 mm à 10 mm : 9 points par rangée, 3 rangées à y = 44, 50, 56.
        assert_eq!(s.len(), 27);
        for y in [44.0, 50.0, 56.0] {
            assert_eq!(s.iter().filter(|q| (q.pos.y - y).abs() < 1e-9).count(), 9);
        }
        assert!(s.iter().all(|q| q.angle.abs() < 1e-9));
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
