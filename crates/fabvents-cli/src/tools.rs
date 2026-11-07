//! Opérations communes à la ligne de commande et au serveur MCP : lire un
//! projet, le mesurer, viser un taux d'ouverture, l'exporter.

use fabvents_core::export::{self, step};
use fabvents_core::solve::{solve_open_ratio, SolveParam};
use fabvents_core::{Pipeline, Project};
use serde_json::{json, Value};

/// Formats d'export.
pub const FORMATS: [&str; 6] = ["svg", "dxf", "step", "step_zip", "png", "project"];

pub fn read_project(text: &str) -> Result<Project, String> {
    Project::from_json(text).map_err(|e| format!("projet illisible ({})", e.code()))
}

/// Mesures, statistiques et messages du moteur.
pub fn report(project: &Project) -> Value {
    let mut pl = Pipeline::new();
    let f = pl.compute(project);
    json!({
        "metrics": f.metrics,
        "stats": f.stats,
        "issues": f.issues,
        "plate": { "width": project.plate.extent().x, "height": project.plate.extent().y, "thickness": project.plate.thickness },
    })
}

/// Contenu d'un export (texte ou octets) et son extension.
pub fn export_bytes(
    project: &Project,
    format: &str,
    png_width: u32,
) -> Result<(Vec<u8>, &'static str), String> {
    let mut pl = Pipeline::new();
    let f = pl.compute(project);
    if f.has_errors() && format != "project" {
        let codes: Vec<&str> = f
            .issues
            .iter()
            .filter(|i| i.severity == fabvents_core::Severity::Error)
            .map(|i| i.code)
            .collect();
        return Err(format!(
            "le motif contient des erreurs : {}",
            codes.join(", ")
        ));
    }
    let now = timestamp();
    Ok(match format {
        "svg" => (
            export::svg::svg(&export::Drawing::new(project, &f)).into_bytes(),
            "svg",
        ),
        "dxf" => (
            export::dxf::dxf(&export::Drawing::new(project, &f)).into_bytes(),
            "dxf",
        ),
        "step" => (
            step::step_file(project, &f, &now)
                .map_err(String::from)?
                .into_bytes(),
            "step",
        ),
        "step_zip" => (
            step::step_zip(project, &f, &now).map_err(String::from)?,
            "zip",
        ),
        "png" => (crate::render::png(project, &f, png_width)?, "png"),
        "project" => (project.to_json_pretty().into_bytes(), "fabvents.json"),
        other => {
            return Err(format!(
                "format inconnu « {other} » (attendus : {})",
                FORMATS.join(", ")
            ))
        }
    })
}

pub fn parse_param(s: &str) -> Result<SolveParam, String> {
    match s {
        "size" | "taille" => Ok(SolveParam::Size),
        "spacing" | "espacement" => Ok(SolveParam::Spacing),
        o => Err(format!("réglage inconnu « {o} » (size ou spacing)")),
    }
}

/// Projet réglé pour viser `target` % d'ouverture, et le bilan.
pub fn solve(
    project: &Project,
    target: f64,
    param: SolveParam,
) -> Result<(Project, Value), String> {
    let (p, s) = solve_open_ratio(project, target, param).map_err(String::from)?;
    Ok((p, serde_json::to_value(&s).map_err(|e| e.to_string())?))
}

/// Date ISO 8601 (UTC) pour l'en-tête STEP, sans dépendance d'horloge
/// calendaire : secondes depuis 1970 converties à la main.
fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Jours depuis 1970 → date (algorithme de H. Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Guide du format de projet pour un agent (outil `fabvents_guide`).
pub const GUIDE: &str = r#"# Projet FabVents (format JSON `fabvents.project`)

Unités : millimètres et degrés. Origine en bas à gauche de la plaque, Y vers le haut.
Partez d'un projet existant (outil `fabvents_new_project`) et modifiez seulement ce qui compte ; les champs absents prennent leur valeur par défaut.

- `plate` : `outline` (`rectangle`, `circle`, `ellipse`, `custom` + `custom: [{x,y}…]`), `width`, `height`, `corner_radius`, `margin` (bande sans perforation), `thickness`, `zone` (`{"type":"plate"}`, `{"type":"circle","center":{x,y},"diameter"}`, `{"type":"rectangle","center","width","height","corner"}`).
- `layout.mode` :
  - `grid` : `pattern` (`square`, `hexagonal`, `staggered`), `pitch_x`, `pitch_y`, `angle`, `offset_x`, `offset_y` ;
  - `random` : `spacing`, `seed` ; `spiral` : `pitch`, `spacing`, `start`, `clockwise` ; `concentric` : `pitch`, `spacing`, `start`, `phase` ; `fibonacci` : `spacing`, `divergence` ; `cross_hatch` : `step`, `line_spacing`, `angle`, `cross` ; `path` : `paths` (courbes), `spacing`, `rows`, `row_gap` ;
  - `voronoi` : `spacing` (taille des cellules), `relax`, `seed`, `rib` (nervure), `fillet` ;
  - `flow` : `spacing` (écart des lignes), `angle`, `width` (rubans), `ends` (`round`, `flat`) ;
  - `script` : `source` (script Rhai qui renvoie des points), `params` (curseurs).
- `shape` : `kind` (`{"type":"circle"}`, `{"type":"rectangle","aspect","corner"}`, `{"type":"polygon","sides"}`, `{"type":"star","points","inner"}`, `{"type":"slot","aspect"}`, `{"type":"custom","layers":[…],"pivot":{x,y}}`), `size` (mm), `rotation`, `morph` (forme cible du canal Forme).
- `edge` : `center`, `whole` (trous entiers seulement), `clip` (découpe au bord).
- `fabrication` : `min_size`, `min_bridge` (nervure mini), `draft` (dépouille STEP, degrés), `tiles` ([colonnes, rangées] de l'export STEP).
- `controllers` : `[{ "id", "geometry": {"type":"point","at":{x,y}} | {"type":"line","a","b"} | {"type":"circle","center","radius"} | …, "size"|"spacing"|"angle"|"morph": {"reach": mm, "target": valeur} }]`. Près du contrôleur, le canal va vers `target` (taille en mm, espacement en mm, angle en degrés, morphing de 0 à 1).
- `features` : trous de fixation et zones réservées `[{ "id", "shape": {"type":"circle","center","diameter"} | {"type":"rectangle",…}, "cut": true|false, "clearance" }]`.

Le moteur ne corrige jamais une valeur : lisez `issues` (code, gravité, champ). Une erreur empêche le calcul ; `bridge.thin` signale des nervures plus fines que `min_bridge`.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_iso() {
        let t = timestamp();
        assert_eq!(t.len(), 19);
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "T");
        assert!(t.starts_with("20"));
    }

    #[test]
    fn every_format_exports_the_example() {
        let p = Project::example();
        for f in FORMATS {
            let (bytes, _) = export_bytes(&p, f, 300).unwrap();
            assert!(!bytes.is_empty(), "{f}");
        }
    }
}
