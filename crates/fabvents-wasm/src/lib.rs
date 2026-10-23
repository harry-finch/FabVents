//! Moteur FabVents pour l'interface : appelé depuis un Web Worker, il reçoit
//! le projet en JSON et renvoie la géométrie à dessiner en tableaux plats
//! (transférables sans copie vers le fil principal) et un rapport JSON
//! (mesures, statistiques, messages).
//!
//! Les erreurs de lecture de projet sont renvoyées par leur code stable
//! (`project.not_fabvents`…), traduit par l'interface.

use fabvents_core::{export, Contour, Pipeline, Project};
use serde_json::json;
use wasm_bindgen::prelude::*;

/// Flèche maximale des arcs dessinés à l'écran (mm).
const DISPLAY_TOLERANCE: f64 = 0.02;

#[wasm_bindgen]
pub struct Engine {
    pipeline: Pipeline,
}

/// Géométrie d'un calcul, prête à dessiner.
#[wasm_bindgen]
pub struct Drawing {
    circles: Vec<f64>,
    poly_points: Vec<f64>,
    poly_offsets: Vec<u32>,
    outline: Vec<f64>,
    report: String,
}

#[wasm_bindgen]
impl Drawing {
    /// Cercles : `x, y, r` à la suite.
    pub fn take_circles(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.circles)
    }

    /// Sommets des autres contours : `x, y` à la suite.
    pub fn take_poly_points(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.poly_points)
    }

    /// Début de chaque contour dans `poly_points` (en sommets), plus la fin.
    pub fn take_poly_offsets(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.poly_offsets)
    }

    /// Contour de la plaque : `x, y` à la suite.
    pub fn take_outline(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.outline)
    }

    /// Mesures, statistiques et messages (JSON).
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

fn read(project_json: &str) -> Result<Project, JsError> {
    Project::from_json(project_json).map_err(|e| JsError::new(e.code()))
}

fn push_points(out: &mut Vec<f64>, c: &Contour) {
    let mut pts = Vec::new();
    c.tessellate(DISPLAY_TOLERANCE, &mut pts);
    for p in pts {
        out.push(p.x);
        out.push(p.y);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            pipeline: Pipeline::new(),
        }
    }

    pub fn compute(&mut self, project_json: &str) -> Result<Drawing, JsError> {
        let project = read(project_json)?;
        let frame = self.pipeline.compute(&project);
        let mut circles = Vec::new();
        let mut poly_points = Vec::new();
        let mut poly_offsets = vec![0u32];
        for h in frame.holes {
            match h.contour() {
                Contour::Circle { center, radius } => circles.extend([center.x, center.y, radius]),
                c => {
                    push_points(&mut poly_points, &c);
                    poly_offsets.push((poly_points.len() / 2) as u32);
                }
            }
        }
        let mut outline = Vec::new();
        push_points(&mut outline, &project.plate.contour());
        let report = json!({
            "metrics": frame.metrics,
            "stats": frame.stats,
            "issues": frame.issues,
            "stages": self.pipeline.last_runs,
        })
        .to_string();
        Ok(Drawing {
            circles,
            poly_points,
            poly_offsets,
            outline,
            report,
        })
    }

    fn drawing_export(
        &mut self,
        project_json: &str,
        f: fn(&export::Drawing) -> String,
    ) -> Result<String, JsError> {
        let project = read(project_json)?;
        let frame = self.pipeline.compute(&project);
        if frame.has_errors() {
            return Err(JsError::new("export.invalid_project"));
        }
        Ok(f(&export::Drawing {
            project: &project,
            holes: frame.holes,
        }))
    }

    pub fn export_svg(&mut self, project_json: &str) -> Result<String, JsError> {
        self.drawing_export(project_json, export::svg::svg)
    }

    pub fn export_dxf(&mut self, project_json: &str) -> Result<String, JsError> {
        self.drawing_export(project_json, export::dxf::dxf)
    }
}

/// Lit un projet `.fabvents.json` ou un SVG exporté par FabVents ; renvoie
/// le projet normalisé (champs manquants complétés, version courante).
#[wasm_bindgen]
pub fn read_project(text: &str) -> Result<String, JsError> {
    let trimmed = text.trim_start();
    let project = if trimmed.starts_with('<') {
        match export::project_from_svg(text) {
            Some(r) => r,
            None => return Err(JsError::new("project.svg_without_project")),
        }
    } else {
        Project::from_json(text)
    };
    project
        .map(|p| p.to_json())
        .map_err(|e| JsError::new(e.code()))
}

/// Projet vide (plaque et grille par défaut, aucun contrôleur).
#[wasm_bindgen]
pub fn default_project() -> String {
    Project::default().to_json()
}

/// Projet de démonstration.
#[wasm_bindgen]
pub fn example_project() -> String {
    Project::example().to_json()
}

#[wasm_bindgen]
pub fn engine_version() -> String {
    fabvents_core::ENGINE_VERSION.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_returns_flat_geometry_and_report() {
        let mut e = Engine::new();
        let mut d = e.compute(&example_project()).unwrap();
        let circles = d.take_circles();
        assert!(!circles.is_empty());
        assert_eq!(circles.len() % 3, 0);
        assert_eq!(d.take_poly_offsets(), vec![0]);
        assert!(d.take_outline().len() >= 8);
        let report: serde_json::Value = serde_json::from_str(&d.report()).unwrap();
        assert_eq!(
            report["metrics"]["holes"].as_u64().unwrap() as usize,
            circles.len() / 3
        );
        assert_eq!(report["stages"], serde_json::json!([true, true, true]));
    }

    #[test]
    fn read_project_accepts_exported_svg() {
        let mut e = Engine::new();
        let svg = e.export_svg(&example_project()).unwrap();
        let back = read_project(&svg).unwrap();
        assert_eq!(Project::from_json(&back).unwrap(), Project::example());
    }
}
