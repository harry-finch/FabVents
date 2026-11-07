//! Dispositions scriptées (« IA sur mesure ») : un script Rhai place les
//! germes et déclare ses propres curseurs. Bac à sable : pas d'accès aux
//! fichiers ni au réseau, pas d'horloge, nombre d'opérations borné ;
//! l'aléatoire est tiré de la graine du projet (même projet, même motif).
//!
//! Dans le script :
//! - `WIDTH`, `HEIGHT` : dimensions de la plaque (mm) ; `CX`, `CY` : son centre ;
//! - `slider("nom", min, max, défaut)` : déclare un curseur, rend sa valeur ;
//! - `point(x, y)` ou `point(x, y, angle)` : pose un germe (angle en degrés) ;
//! - `rand()` : nombre de [0, 1) tiré de la graine ;
//! - fonctions mathématiques de Rhai (`sin`, `cos`, `sqrt`, `PI()`…).

use crate::geom::Vec2;
use crate::issue::{Issue, Severity};
use crate::layout::Seed;
use crate::project::{Plate, ScriptLayout};
use crate::rng::Rng;
use rhai::{Engine, EvalAltResult, Position, Scope};
use serde::Serialize;
use std::cell::RefCell;
use std::rc::Rc;

/// Opérations au plus par exécution.
pub const MAX_OPERATIONS: u64 = 20_000_000;
/// Germes au plus.
pub const MAX_POINTS: usize = 200_000;

/// Curseur déclaré par un script.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Slider {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// Valeur utilisée (celle du projet, bornée, ou la valeur par défaut).
    pub value: f64,
}

/// Germes et curseurs d'un script.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScriptRun {
    pub seeds: Vec<Seed>,
    pub sliders: Vec<Slider>,
}

/// Exécute le script ; une erreur de syntaxe ou d'exécution devient un
/// message `layout.script_error` (ligne, colonne, texte de Rhai).
pub fn run(l: &ScriptLayout, plate: &Plate) -> Result<ScriptRun, Issue> {
    let points: Rc<RefCell<Vec<Seed>>> = Rc::default();
    let sliders: Rc<RefCell<Vec<Slider>>> = Rc::default();
    let rng = Rc::new(RefCell::new(Rng::new(l.seed)));
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(10_000);
    engine.set_max_array_size(MAX_POINTS);
    engine.set_max_map_size(10_000);
    engine.disable_symbol("eval");

    let too_many = || -> Box<EvalAltResult> { format!("plus de {MAX_POINTS} points").into() };
    {
        let pts = points.clone();
        engine.register_fn(
            "point",
            move |x: f64, y: f64| -> Result<(), Box<EvalAltResult>> {
                let mut p = pts.borrow_mut();
                if p.len() >= MAX_POINTS {
                    return Err(too_many());
                }
                p.push(Seed {
                    pos: Vec2::new(x, y),
                    angle: 0.0,
                    shape: None,
                });
                Ok(())
            },
        );
    }
    {
        let pts = points.clone();
        engine.register_fn(
            "point",
            move |x: f64, y: f64, a: f64| -> Result<(), Box<EvalAltResult>> {
                let mut p = pts.borrow_mut();
                if p.len() >= MAX_POINTS {
                    return Err(too_many());
                }
                p.push(Seed {
                    pos: Vec2::new(x, y),
                    angle: a,
                    shape: None,
                });
                Ok(())
            },
        );
    }
    {
        let s = sliders.clone();
        let values = l.params.clone();
        engine.register_fn(
            "slider",
            move |name: &str,
                  min: f64,
                  max: f64,
                  default: f64|
                  -> Result<f64, Box<EvalAltResult>> {
                if !(min.is_finite() && max.is_finite() && min < max) {
                    return Err(format!("curseur « {name} » : minimum et maximum invalides").into());
                }
                let value = values.get(name).copied().unwrap_or(default).clamp(min, max);
                let mut list = s.borrow_mut();
                if !list.iter().any(|x| x.name == name) {
                    list.push(Slider {
                        name: name.to_string(),
                        min,
                        max,
                        default,
                        value,
                    });
                }
                Ok(value)
            },
        );
    }
    {
        let r = rng.clone();
        engine.register_fn("rand", move || r.borrow_mut().unit());
    }

    let ext = plate.extent();
    let mut scope = Scope::new();
    scope.push_constant("WIDTH", ext.x);
    scope.push_constant("HEIGHT", ext.y);
    scope.push_constant("CX", ext.x / 2.0);
    scope.push_constant("CY", ext.y / 2.0);

    let error = |e: Box<EvalAltResult>| {
        let pos: Position = e.position();
        let place = match (pos.line(), pos.position()) {
            (Some(l), Some(c)) => format!("ligne {l}, colonne {c} : "),
            (Some(l), None) => format!("ligne {l} : "),
            _ => String::new(),
        };
        let text = match *e {
            EvalAltResult::ErrorTooManyOperations(_) => {
                "script trop long (opérations bornées)".to_string()
            }
            ref other => other.to_string(),
        };
        Issue::new("layout.script_error", Severity::Error)
            .field("layout.source")
            .detail(format!("{place}{text}"))
    };
    engine
        .run_with_scope(&mut scope, &l.source)
        .map_err(error)?;
    let seeds = std::mem::take(&mut *points.borrow_mut());
    let sliders = std::mem::take(&mut *sliders.borrow_mut());
    if seeds
        .iter()
        .any(|s| !(s.pos.x.is_finite() && s.pos.y.is_finite() && s.angle.is_finite()))
    {
        return Err(Issue::new("layout.script_error", Severity::Error)
            .field("layout.source")
            .detail("point non fini (NaN ou infini)"));
    }
    Ok(ScriptRun { seeds, sliders })
}

/// Script de départ : vagues de perforations.
pub const EXAMPLE: &str = r#"// Disposition scriptée : posez des germes avec point(x, y) ou point(x, y, angle).
// WIDTH, HEIGHT : dimensions de la plaque (mm) ; CX, CY : son centre.
// slider("nom", min, max, défaut) ajoute un curseur dans le panneau.
let pas = slider("Pas", 3.0, 20.0, 7.0);
let onde = slider("Amplitude", 0.0, 10.0, 3.0);
let periode = slider("Période", 10.0, 200.0, 60.0);

let y = pas / 2.0;
while y < HEIGHT {
    let x = pas / 2.0;
    while x < WIDTH {
        let dy = onde * sin(2.0 * PI() * x / periode);
        point(x, y + dy);
        x += pas;
    }
    y += pas;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn plate() -> Plate {
        Plate {
            width: 100.0,
            height: 60.0,
            ..Plate::default()
        }
    }

    #[test]
    fn example_places_points_and_declares_sliders() {
        let l = ScriptLayout {
            source: EXAMPLE.into(),
            ..ScriptLayout::default()
        };
        let r = run(&l, &plate()).unwrap();
        assert!(r.seeds.len() > 50, "{}", r.seeds.len());
        assert_eq!(r.sliders.len(), 3);
        assert_eq!(r.sliders[0].name, "Pas");
        assert_eq!(r.sliders[0].value, 7.0);
        // Un curseur réglé dans le projet change le motif.
        let mut params = BTreeMap::new();
        params.insert("Pas".to_string(), 14.0);
        let r2 = run(&ScriptLayout { params, ..l }, &plate()).unwrap();
        assert!(r2.seeds.len() < r.seeds.len() / 2);
    }

    #[test]
    fn errors_are_reported_with_their_position() {
        let l = ScriptLayout {
            source: "let x = ;".into(),
            ..ScriptLayout::default()
        };
        let e = run(&l, &plate()).unwrap_err();
        assert_eq!(e.code, "layout.script_error");
        assert!(e.detail.unwrap().contains("ligne 1"));
    }

    #[test]
    fn endless_loops_are_stopped() {
        let l = ScriptLayout {
            source: "loop { }".into(),
            ..ScriptLayout::default()
        };
        let e = run(&l, &plate()).unwrap_err();
        assert!(e.detail.unwrap().contains("trop long"));
    }

    #[test]
    fn random_is_seeded() {
        let l = ScriptLayout {
            source: "for i in 0..10 { point(rand() * WIDTH, rand() * HEIGHT); }".into(),
            seed: 3,
            ..ScriptLayout::default()
        };
        let a = run(&l, &plate()).unwrap();
        let b = run(&l, &plate()).unwrap();
        assert_eq!(a.seeds, b.seeds);
        let c = run(&ScriptLayout { seed: 4, ..l }, &plate()).unwrap();
        assert_ne!(a.seeds, c.seeds);
    }
}
