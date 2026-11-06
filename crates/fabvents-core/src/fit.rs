//! Contours exacts des découpes : les booléens rendent des polygones fins
//! (facettisation à 0,002 mm) ; on y retrouve les arcs des cercles connus
//! (trous, trous de fixation, zone perforée) et on fusionne les segments
//! alignés. Les exports (SVG, DXF, STEP) y gagnent des arcs vrais et des
//! fichiers bien plus légers.

use crate::contour::{Contour, Vertex};
use crate::geom::{dist_point_segment, Vec2};
use std::collections::HashMap;
use std::f64::consts::TAU;

/// Écart toléré entre un sommet et un cercle connu (mm) : flèche de la
/// facettisation des découpes (0,002 mm) et précision des booléens.
const ON_CIRCLE: f64 = 3e-3;
/// Flèche maximale d'une facette sur un cercle (mm), au-delà de la
/// facettisation des découpes.
const SAGITTA: f64 = 0.01;
/// Écart toléré pour fusionner des segments alignés (mm).
const COLLINEAR: f64 = 1e-4;

/// Cercles connus, rangés dans une grille de répartition.
#[derive(Default)]
pub struct Circles {
    list: Vec<(Vec2, f64)>,
    cell: f64,
    grid: HashMap<(i64, i64), Vec<usize>>,
}

impl Circles {
    pub fn new(list: Vec<(Vec2, f64)>) -> Circles {
        let rmax = list.iter().map(|c| c.1).fold(0.0f64, f64::max);
        let cell = (2.0 * rmax).max(1.0);
        let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (k, &(c, r)) in list.iter().enumerate() {
            let (x0, y0) = key(c - Vec2::new(r, r), cell);
            let (x1, y1) = key(c + Vec2::new(r, r), cell);
            for gx in x0..=x1 {
                for gy in y0..=y1 {
                    grid.entry((gx, gy)).or_default().push(k);
                }
            }
        }
        Circles { list, cell, grid }
    }

    /// Cercles collectés sur des contours (cercles et arcs).
    pub fn from_contours<'a>(contours: impl IntoIterator<Item = &'a Contour>) -> Circles {
        let mut list: Vec<(Vec2, f64)> = Vec::new();
        for c in contours {
            match c {
                Contour::Circle { center, radius } => list.push((*center, *radius)),
                Contour::Path(v) => {
                    let n = v.len();
                    for i in 0..n {
                        if v[i].bulge != 0.0 {
                            list.push(crate::geom::bulge_arc(v[i].p, v[(i + 1) % n].p, v[i].bulge));
                        }
                    }
                }
            }
        }
        // Un même cercle n'est gardé qu'une fois (coins d'un rectangle arrondi).
        list.sort_by(|a, b| {
            a.0.x
                .total_cmp(&b.0.x)
                .then(a.0.y.total_cmp(&b.0.y))
                .then(a.1.total_cmp(&b.1))
        });
        list.dedup_by(|a, b| a.0.distance(b.0) < 1e-9 && (a.1 - b.1).abs() < 1e-9);
        Circles::new(list)
    }

    fn near(&self, p: Vec2) -> impl Iterator<Item = usize> + '_ {
        self.grid
            .get(&key(p, self.cell))
            .into_iter()
            .flatten()
            .copied()
            .filter(move |&k| {
                let (c, r) = self.list[k];
                (p.distance(c) - r).abs() < ON_CIRCLE
            })
    }

    /// Le segment `a → b` est-il une facette du cercle `k` ?
    fn facet(&self, k: usize, a: Vec2, b: Vec2) -> bool {
        let (c, r) = self.list[k];
        let on = |p: Vec2| (p.distance(c) - r).abs() < ON_CIRCLE;
        on(a) && on(b) && a.distance(b) < 2.0 * r && r - a.lerp(b, 0.5).distance(c) < SAGITTA
    }
}

fn key(p: Vec2, cell: f64) -> (i64, i64) {
    ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64)
}

/// Longueur au-dessous de laquelle un passage droit entre deux arcs est un
/// raccord de facettisation, remplacé par l'intersection exacte (mm).
const JOIN: f64 = 0.05;

/// Contour exact d'un polygone fin : arcs des cercles connus (raccords
/// recalés sur les intersections exactes), segments alignés fusionnés.
pub fn fit(pts: &[Vec2], circles: &Circles) -> Contour {
    let n = pts.len();
    let line =
        |pts: &[Vec2]| Contour::Path(pts.iter().map(|&p| Vertex { p, bulge: 0.0 }).collect());
    if n < 3 {
        return line(pts);
    }
    // Étiquette de chaque segment : cercle porteur, ou aucun (droit).
    let mut label: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let prev = if i > 0 { label[i - 1] } else { None };
        let mut found = None;
        for k in circles.near(a) {
            if circles.facet(k, a, b) {
                found = Some(k);
                if Some(k) == prev {
                    break;
                }
            }
        }
        label[i] = found;
    }
    if let Some(k) = label[0].filter(|_| label.iter().all(|l| *l == label[0])) {
        return Contour::Circle {
            center: circles.list[k].0,
            radius: circles.list[k].1,
        };
    }
    // Suites de segments de même étiquette : (étiquette, premier sommet, nombre).
    let start = (0..n)
        .find(|&i| label[i] != label[(i + n - 1) % n])
        .unwrap_or(0);
    let mut runs: Vec<(Option<usize>, usize, usize)> = Vec::new();
    let mut i = start;
    let mut done = 0;
    while done < n {
        let mut len = 0;
        while done + len < n && label[(i + len) % n] == label[i] {
            len += 1;
        }
        runs.push((label[i], i, len));
        done += len;
        i = (i + len) % n;
    }
    let chain_len = |r: (Option<usize>, usize, usize)| -> f64 {
        (0..r.2)
            .map(|s| pts[(r.1 + s) % n].distance(pts[(r.1 + s + 1) % n]))
            .sum()
    };
    // Raccords courts entre deux arcs : retirés.
    let m = runs.len();
    let keep: Vec<bool> = (0..m)
        .map(|r| {
            let (lab, _, _) = runs[r];
            let prev = runs[(r + m - 1) % m].0;
            let next = runs[(r + 1) % m].0;
            !(m > 2
                && lab.is_none()
                && prev.is_some()
                && next.is_some()
                && chain_len(runs[r]) < JOIN)
        })
        .collect();
    let runs: Vec<(Option<usize>, usize, usize)> =
        (0..m).filter(|&r| keep[r]).map(|r| runs[r]).collect();
    let m = runs.len();
    // Point de départ exact de chaque suite (fin de la précédente).
    let support = |r: usize| -> Support {
        let (lab, from, len) = runs[r];
        match lab {
            Some(k) => Support::Circle(circles.list[k].0, circles.list[k].1),
            None => Support::Line(pts[from], pts[(from + len) % n]),
        }
    };
    let joints: Vec<Vec2> = (0..m)
        .map(|r| {
            let near = pts[runs[r].1];
            let prev = (r + m - 1) % m;
            // Sommet d'origine : fin de la suite précédente gardée.
            let (_, pf, pl) = runs[prev];
            let end_prev = pts[(pf + pl) % n];
            let guess = end_prev.lerp(near, 0.5);
            intersect(support(prev), support(r))
                .into_iter()
                .filter(|p| p.distance(guess) < JOIN + ON_CIRCLE + near.distance(end_prev))
                .min_by(|a, b| a.distance(guess).total_cmp(&b.distance(guess)))
                .unwrap_or(near)
        })
        .collect();
    let mut out: Vec<Vertex> = Vec::new();
    for r in 0..m {
        let (lab, from, len) = runs[r];
        let (a, b) = (joints[r], joints[(r + 1) % m]);
        match lab {
            Some(k) => {
                let c = circles.list[k].0;
                // Sens donné par les facettes, angle exact entre les raccords.
                let mut along = 0.0;
                for s in 0..len {
                    let u = pts[(from + s) % n] - c;
                    let v = pts[(from + s + 1) % n] - c;
                    along += u.cross(v).atan2(u.dot(v));
                }
                let (u, v) = (a - c, b - c);
                let raw = u.cross(v).atan2(u.dot(v));
                let mut sweep = if along >= 0.0 {
                    raw.rem_euclid(TAU)
                } else {
                    -(-raw).rem_euclid(TAU)
                };
                if (sweep - along).abs() > 1.0 {
                    sweep = along;
                }
                out.push(Vertex {
                    p: a,
                    bulge: (sweep / 4.0).tan(),
                });
            }
            None => {
                let mut chain: Vec<Vec2> = vec![a];
                chain.extend((1..len).map(|s| pts[(from + s) % n]));
                chain.push(b);
                for p in simplify(&chain, COLLINEAR) {
                    out.push(Vertex { p, bulge: 0.0 });
                }
            }
        }
    }
    Contour::Path(out)
}

#[derive(Clone, Copy)]
enum Support {
    Circle(Vec2, f64),
    Line(Vec2, Vec2),
}

fn intersect(a: Support, b: Support) -> Vec<Vec2> {
    match (a, b) {
        (Support::Circle(c, r), Support::Circle(c2, r2)) => {
            let d = c.distance(c2);
            if d < 1e-12 || d > r + r2 + ON_CIRCLE || d < (r - r2).abs() - ON_CIRCLE {
                return Vec::new();
            }
            let x = (r * r - r2 * r2 + d * d) / (2.0 * d);
            let h = (r * r - x * x).max(0.0).sqrt();
            let u = (c2 - c) * (1.0 / d);
            let m = c + u * x;
            vec![m + u.left() * h, m - u.left() * h]
        }
        (Support::Circle(c, r), Support::Line(p, q))
        | (Support::Line(p, q), Support::Circle(c, r)) => {
            let dir = (q - p).normalized();
            let f = p - c;
            let bq = f.dot(dir);
            let disc = bq * bq - (f.length_sq() - r * r);
            if disc < 0.0 {
                return Vec::new();
            }
            let s = disc.sqrt();
            vec![p + dir * (-bq - s), p + dir * (-bq + s)]
        }
        (Support::Line(..), Support::Line(..)) => Vec::new(),
    }
}

/// Ramer-Douglas-Peucker sur une chaîne ouverte, sans le dernier point
/// (une chaîne qui revient à son départ donne un polygone fermé).
pub fn simplify(chain: &[Vec2], tol: f64) -> Vec<Vec2> {
    fn rec(chain: &[Vec2], tol: f64, out: &mut Vec<Vec2>) {
        let (a, b) = (chain[0], chain[chain.len() - 1]);
        let (mut far, mut idx) = (0.0, 0);
        for (k, &p) in chain.iter().enumerate().take(chain.len() - 1).skip(1) {
            let d = dist_point_segment(p, a, b);
            if d > far {
                far = d;
                idx = k;
            }
        }
        if far > tol {
            rec(&chain[..=idx], tol, out);
            rec(&chain[idx..], tol, out);
        } else {
            out.push(a);
        }
    }
    let mut out = Vec::new();
    if chain.len() >= 2 {
        rec(chain, tol, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boolean::cut;
    use std::f64::consts::PI;

    #[test]
    fn two_merged_discs_become_two_arcs() {
        let a = Contour::Circle {
            center: Vec2::ZERO,
            radius: 2.0,
        };
        let b = Contour::Circle {
            center: Vec2::new(3.0, 0.0),
            radius: 2.0,
        };
        let polys = [a.tessellated(0.002), b.tessellated(0.002)];
        let big = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];
        let union = cut(&polys, &[big], &[]);
        assert_eq!(union.len(), 1);
        let circles = Circles::from_contours([&a, &b]);
        let c = fit(&union[0].outer, &circles);
        let Contour::Path(v) = &c else { panic!() };
        assert_eq!(v.len(), 2, "{v:?}");
        assert!(v.iter().all(|x| x.bulge != 0.0));
        // Aire exacte de l'union de deux disques de rayon 2 à 3 mm d'écart.
        let lens = 8.0 * (0.75f64).acos() - 1.5 * 7f64.sqrt();
        let exact = 2.0 * PI * 4.0 - lens;
        assert!(
            (c.area() - exact).abs() < 1e-9,
            "{} {} {v:?}",
            c.area(),
            exact
        );
    }

    #[test]
    fn straight_runs_are_merged() {
        let pts: Vec<Vec2> = (0..=10)
            .map(|k| Vec2::new(k as f64, 0.0))
            .chain([Vec2::new(10.0, 5.0), Vec2::new(0.0, 5.0)])
            .collect();
        let Contour::Path(v) = fit(&pts, &Circles::default()) else {
            panic!()
        };
        assert_eq!(v.len(), 4);
    }
}
