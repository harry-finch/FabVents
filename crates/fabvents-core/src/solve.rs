//! Solveur de taux d'ouverture : cherche par dichotomie la taille de base
//! (taille de la forme, nervure de Voronoï, largeur des rubans) ou
//! l'espacement de base qui donne le taux d'ouverture visé, sans
//! chevauchement ni nervure plus fine que le minimum demandé. Si le taux
//! n'est pas atteignable dans ces limites, le meilleur réglage possible est
//! rendu et signalé (`reached: false`).

use crate::pipeline::Pipeline;
use crate::project::{Layout, Project};
use serde::{Deserialize, Serialize};

/// Réglage que fait varier le solveur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolveParam {
    /// Valeur de base du canal Taille.
    Size,
    /// Pas de la grille, espacement des autres dispositions.
    Spacing,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Solution {
    pub param: SolveParam,
    /// Valeur retenue (mm).
    pub value: f64,
    /// Taux d'ouverture obtenu (%).
    pub open_ratio: f64,
    /// Le taux visé est atteint (à `tolerance` près).
    pub reached: bool,
    pub bridge_min: Option<f64>,
    /// Calculs du motif effectués.
    pub evaluations: usize,
}

/// Écart toléré sur le taux d'ouverture (points de %) : le nombre de
/// perforations varie par paliers avec l'espacement, le taux aussi.
pub const TOLERANCE: f64 = 0.25;
const ITERATIONS: usize = 40;

/// Valeur du réglage dans le projet.
pub fn get(p: &Project, param: SolveParam) -> Option<f64> {
    match param {
        SolveParam::Size => Some(p.layout.base_size(&p.shape)),
        SolveParam::Spacing => match &p.layout {
            Layout::Grid(g) => Some(g.pitch_x),
            Layout::CrossHatch(c) => Some(c.step),
            l => l.base_spacing(),
        },
    }
}

/// Fixe le réglage. Grille : le pas vertical suit dans la même proportion.
pub fn set(p: &mut Project, param: SolveParam, v: f64) {
    match param {
        SolveParam::Size => match &mut p.layout {
            Layout::Voronoi(l) => l.rib = v,
            Layout::Flow(l) => l.width = v,
            _ => p.shape.size = v,
        },
        SolveParam::Spacing => match &mut p.layout {
            Layout::Grid(g) => {
                let k = if g.pitch_x > 0.0 {
                    g.pitch_y / g.pitch_x
                } else {
                    1.0
                };
                g.pitch_x = v;
                g.pitch_y = v * k;
            }
            Layout::CrossHatch(c) => {
                let k = if c.step > 0.0 {
                    c.line_spacing / c.step
                } else {
                    1.0
                };
                c.step = v;
                c.line_spacing = v * k;
            }
            Layout::Random(l) => l.spacing = v,
            Layout::Spiral(l) => {
                let k = if l.spacing > 0.0 {
                    l.pitch / l.spacing
                } else {
                    1.0
                };
                l.spacing = v;
                l.pitch = v * k;
            }
            Layout::Concentric(l) => {
                let k = if l.spacing > 0.0 {
                    l.pitch / l.spacing
                } else {
                    1.0
                };
                l.spacing = v;
                l.pitch = v * k;
            }
            Layout::Fibonacci(l) => l.spacing = v,
            Layout::Path(l) => l.spacing = v,
            Layout::Voronoi(l) => l.spacing = v,
            Layout::Flow(l) => l.spacing = v,
            Layout::Script(_) => {}
        },
    }
}

/// Mesure d'un réglage : taux d'ouverture et respect des nervures.
struct Probe {
    ratio: f64,
    ok: bool,
    bridge: Option<f64>,
}

/// Projet réglé pour viser `target` % d'ouverture, et le bilan.
pub fn solve_open_ratio(
    p: &Project,
    target: f64,
    param: SolveParam,
) -> Result<(Project, Solution), &'static str> {
    let start = get(p, param).ok_or("solve.unsupported")?;
    if !(target.is_finite() && target > 0.0 && target < 100.0) {
        return Err("solve.target_range");
    }
    let mut pipeline = Pipeline::new();
    let mut evaluations = 0usize;
    let mut work = p.clone();
    let min_bridge = p.fabrication.min_bridge;
    let mut probe = |v: f64| -> Option<Probe> {
        set(&mut work, param, v);
        evaluations += 1;
        let f = pipeline.compute(&work);
        if f.has_errors() || f.holes.is_empty() {
            return None;
        }
        let m = &f.metrics;
        Some(Probe {
            ratio: m.open_ratio,
            ok: m.overlaps == 0
                && m.thin_bridges == 0
                && m.bridge_min.is_none_or(|b| b >= min_bridge - 1e-9),
            bridge: m.bridge_min,
        })
    };
    // Sens : la taille ouvre (sauf la nervure de Voronoï), l'espacement ferme.
    let opens = param == SolveParam::Size && !matches!(p.layout, Layout::Voronoi(_));
    let (lo_limit, hi_limit) = match (param, &p.layout) {
        (SolveParam::Size, Layout::Voronoi(_)) => (min_bridge.max(0.05), start.max(1.0) * 8.0),
        (SolveParam::Size, _) => (p.fabrication.min_size.max(0.05), start.max(1.0) * 8.0),
        (SolveParam::Spacing, _) => (0.2, start.max(1.0) * 8.0),
    };
    // Valeur la plus ouvrante qui respecte encore les nervures : on part de
    // la valeur actuelle et on avance vers l'ouverture tant que c'est permis.
    let more_open = |a: f64, b: f64| if opens { a.max(b) } else { a.min(b) };
    let (mut safe, mut unsafe_) = match probe(start) {
        Some(pr) if pr.ok => (start, if opens { hi_limit } else { lo_limit }),
        _ => (if opens { lo_limit } else { hi_limit }, start),
    };
    if !probe(safe).is_some_and(|pr| pr.ok) {
        return Err("solve.no_feasible");
    }
    for _ in 0..ITERATIONS {
        let mid = 0.5 * (safe + unsafe_);
        match probe(mid) {
            Some(pr) if pr.ok => safe = mid,
            _ => unsafe_ = mid,
        }
        if (safe - unsafe_).abs() < 1e-4 {
            break;
        }
    }
    let limit = safe;
    let at_limit = probe(limit).ok_or("solve.no_feasible")?;
    // Valeur la moins ouvrante du domaine (pour encadrer la cible).
    let closed_end = if opens { lo_limit } else { hi_limit };
    let finish = |v: f64, probe: &mut dyn FnMut(f64) -> Option<Probe>| -> (Project, Solution) {
        let pr = probe(v);
        let mut out = p.clone();
        set(&mut out, param, v);
        let ratio = pr.as_ref().map_or(0.0, |x| x.ratio);
        (
            out,
            Solution {
                param,
                value: v,
                open_ratio: ratio,
                reached: (ratio - target).abs() <= TOLERANCE,
                bridge_min: pr.and_then(|x| x.bridge),
                evaluations: 0,
            },
        )
    };
    if at_limit.ratio < target {
        // Taux visé trop élevé : le plus ouvert possible, signalé.
        let (proj, mut sol) = finish(limit, &mut probe);
        sol.reached = false;
        sol.evaluations = evaluations;
        return Ok((proj, sol));
    }
    // Dichotomie entre la limite et l'extrémité fermée.
    let (mut open_v, mut closed_v) = (limit, closed_end);
    for _ in 0..ITERATIONS {
        let mid = 0.5 * (open_v + closed_v);
        match probe(mid) {
            Some(pr) if pr.ratio >= target => open_v = mid,
            _ => closed_v = mid,
        }
        if (open_v - closed_v).abs() < 1e-4 {
            break;
        }
    }
    // La valeur la plus proche de la cible, arrondie au centième.
    let candidates = [open_v, closed_v, (open_v * 100.0).round() / 100.0];
    let mut best: Option<(f64, f64)> = None;
    for v in candidates {
        let v = more_open(v.min(hi_limit).max(lo_limit), v);
        if let Some(pr) = probe(v).filter(|pr| pr.ok) {
            if best.is_none_or(|b| (pr.ratio - target).abs() < (b.1 - target).abs()) {
                best = Some((v, pr.ratio));
            }
        }
    }
    let v = best.map_or(open_v, |b| b.0);
    let (proj, mut sol) = finish(v, &mut probe);
    sol.evaluations = evaluations;
    Ok((proj, sol))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Grid, GridPattern, Plate};

    fn grid() -> Project {
        Project {
            layout: Layout::Grid(Grid {
                pattern: GridPattern::Hexagonal,
                pitch_x: 6.0,
                pitch_y: 6.0,
                ..Grid::default()
            }),
            plate: Plate {
                width: 120.0,
                height: 80.0,
                margin: 5.0,
                ..Plate::default()
            },
            ..Project::default()
        }
    }

    #[test]
    fn size_hits_the_target() {
        let p = grid();
        let (q, s) = solve_open_ratio(&p, 40.0, SolveParam::Size).unwrap();
        assert!(s.reached, "{s:?}");
        assert!((s.open_ratio - 40.0).abs() <= TOLERANCE);
        assert_eq!(get(&q, SolveParam::Size), Some(s.value));
        assert!(s.bridge_min.unwrap() >= p.fabrication.min_bridge);
    }

    #[test]
    fn spacing_hits_the_target() {
        let (_, s) = solve_open_ratio(&grid(), 25.0, SolveParam::Spacing).unwrap();
        assert!(s.reached, "{s:?}");
    }

    #[test]
    fn unreachable_target_gives_the_best_possible() {
        // 95 % d'ouverture avec des nervures d'au moins 1 mm : impossible.
        let (_, s) = solve_open_ratio(&grid(), 95.0, SolveParam::Size).unwrap();
        assert!(!s.reached);
        // Pas de 6 mm, nervure 1 mm, marge 5 mm : environ 46 % au plus.
        assert!(s.open_ratio > 40.0 && s.open_ratio < 95.0, "{s:?}");
        assert!(s.bridge_min.unwrap() >= 1.0 - 1e-6);
    }

    #[test]
    fn voronoi_rib_closes_when_it_grows() {
        let mut p = grid();
        p.layout = Layout::Voronoi(crate::project::Voronoi::default());
        let (_, s) = solve_open_ratio(&p, 50.0, SolveParam::Size).unwrap();
        assert!(s.reached, "{s:?}");
    }
}
