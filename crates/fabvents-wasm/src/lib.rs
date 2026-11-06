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
    zone_points: Vec<f64>,
    zone_offsets: Vec<u32>,
    alert_points: Vec<f64>,
    alert_offsets: Vec<u32>,
    tile_cuts: Vec<f64>,
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

    /// Contours de la zone perforable : `x, y` à la suite.
    pub fn take_zone_points(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.zone_points)
    }

    /// Début de chaque contour de zone dans `zone_points`, plus la fin.
    pub fn take_zone_offsets(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.zone_offsets)
    }

    /// Perforations en cause pour l'export STEP (dépouille) : `x, y` à la suite.
    pub fn take_alert_points(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.alert_points)
    }

    /// Début de chaque contour en cause dans `alert_points`, plus la fin.
    pub fn take_alert_offsets(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.alert_offsets)
    }

    /// Lignes de coupe des dalles : nombre de coupes en x, puis les x, puis les y.
    pub fn take_tile_cuts(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.tile_cuts)
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
        let add =
            |c: Contour, circles: &mut Vec<f64>, pts: &mut Vec<f64>, off: &mut Vec<u32>| match c {
                Contour::Circle { center, radius } => circles.extend([center.x, center.y, radius]),
                c => {
                    push_points(pts, &c);
                    off.push((pts.len() / 2) as u32);
                }
            };
        // Trous exacts, découpes (contours et îlots : remplissage pair-impair),
        // trous de fixation.
        for c in export::Drawing::new(&project, &frame).inner_contours() {
            add(c, &mut circles, &mut poly_points, &mut poly_offsets);
        }
        let mut zone_points = Vec::new();
        let mut zone_offsets = vec![0u32];
        for c in project.plate.perforable() {
            for p in c {
                zone_points.extend([p.x, p.y]);
            }
            zone_offsets.push((zone_points.len() / 2) as u32);
        }
        let mut outline = Vec::new();
        push_points(&mut outline, &project.plate.contour());
        let mut alert_points = Vec::new();
        let mut alert_offsets = vec![0u32];
        for c in &frame.plan.alerts {
            for p in c {
                alert_points.extend([p.x, p.y]);
            }
            alert_offsets.push((alert_points.len() / 2) as u32);
        }
        let mut tile_cuts = vec![frame.plan.cuts_x.len() as f64];
        tile_cuts.extend(&frame.plan.cuts_x);
        tile_cuts.extend(&frame.plan.cuts_y);
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
            zone_points,
            zone_offsets,
            alert_points,
            alert_offsets,
            tile_cuts,
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
        Ok(f(&export::Drawing::new(&project, &frame)))
    }

    pub fn export_svg(&mut self, project_json: &str) -> Result<String, JsError> {
        self.drawing_export(project_json, export::svg::svg)
    }

    pub fn export_dxf(&mut self, project_json: &str) -> Result<String, JsError> {
        self.drawing_export(project_json, export::dxf::dxf)
    }

    /// STEP de toute la plaque (une pièce par dalle). `timestamp` : date ISO
    /// 8601 de l'en-tête.
    pub fn export_step(&mut self, project_json: &str, timestamp: &str) -> Result<String, JsError> {
        let project = read(project_json)?;
        let frame = self.pipeline.compute(&project);
        export::step::step_file(&project, &frame, timestamp).map_err(JsError::new)
    }

    /// Archive ZIP d'un STEP par dalle.
    pub fn export_step_zip(
        &mut self,
        project_json: &str,
        timestamp: &str,
    ) -> Result<Vec<u8>, JsError> {
        let project = read(project_json)?;
        let frame = self.pipeline.compute(&project);
        export::step::step_zip(&project, &frame, timestamp).map_err(JsError::new)
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

/// Contour importé d'un SVG ou d'un DXF : `{ outer, inner, width, height }`
/// (points `{x, y}` en mm, coin bas gauche à l'origine).
#[wasm_bindgen]
pub fn import_outline(text: &str) -> Result<String, JsError> {
    let r = fabvents_core::import::import_outline(text).map_err(|e| JsError::new(e.code()))?;
    let pts = |c: &[fabvents_core::Vec2]| {
        c.iter()
            .map(|p| json!({ "x": p.x, "y": p.y }))
            .collect::<Vec<_>>()
    };
    Ok(json!({
        "outer": pts(&r.outer),
        "inner": r.inner.iter().map(|c| pts(c)).collect::<Vec<_>>(),
        "width": r.width,
        "height": r.height,
    })
    .to_string())
}

/// Gabarit de grille de ventilateur (40, 60, 80, 92, 120 ou 140 mm).
#[wasm_bindgen]
pub fn fan_template(size: u32) -> Result<String, JsError> {
    fabvents_core::templates::fan_grill(size)
        .map(|p| p.to_json())
        .ok_or_else(|| JsError::new("template.unknown_size"))
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
        assert_eq!(
            report["stages"],
            serde_json::json!([true, true, true, true, false])
        );
        assert_eq!(d.take_zone_offsets().len(), 2);
    }

    #[test]
    fn step_exports_are_exposed() {
        let mut e = Engine::new();
        let step = e
            .export_step(&example_project(), "2026-11-06T00:00:00")
            .unwrap();
        assert!(step.contains("MANIFOLD_SOLID_BREP"));
        let zip = e
            .export_step_zip(&example_project(), "2026-11-06T00:00:00")
            .unwrap();
        assert_eq!(&zip[..2], b"PK");
        let mut d = e.compute(&example_project()).unwrap();
        assert_eq!(d.take_tile_cuts(), vec![0.0]);
        assert_eq!(d.take_alert_offsets(), vec![0]);
    }

    #[test]
    fn templates_and_import_are_exposed() {
        let p = Project::from_json(&fan_template(120).unwrap()).unwrap();
        assert_eq!(p.features.len(), 4);
        let mut e = Engine::new();
        let svg = e.export_svg(&p.to_json()).unwrap();
        let imported: serde_json::Value =
            serde_json::from_str(&import_outline(&svg).unwrap()).unwrap();
        assert!((imported["width"].as_f64().unwrap() - 120.0).abs() < 1e-3);
    }

    #[test]
    fn read_project_accepts_exported_svg() {
        let mut e = Engine::new();
        let svg = e.export_svg(&example_project()).unwrap();
        let back = read_project(&svg).unwrap();
        assert_eq!(Project::from_json(&back).unwrap(), Project::example());
    }
}
