//! Exports : SVG (découpe laser, projet embarqué), DXF R12 (laser, CNC).
//!
//! Conventions communes : millimètres, contour de la plaque sur le calque /
//! la couleur « extérieur » (rouge, découpé en dernier), perforations sur
//! « intérieur » (bleu). Les cercles restent des cercles et les arcs des
//! arcs (voir `contour`).

pub mod dxf;
pub mod svg;

use crate::pipeline::Hole;
use crate::project::{Project, ProjectError};

/// Formatage compact d'une coordonnée : au plus `decimals` décimales, sans
/// zéros inutiles, jamais `-0`.
pub(crate) fn num(v: f64, decimals: usize) -> String {
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Identifiant de la balise `<metadata>` qui porte le projet dans un SVG.
pub const SVG_METADATA_ID: &str = "fabvents-project";

/// Projet embarqué dans un SVG exporté par FabVents, s'il y en a un.
pub fn project_from_svg(svg: &str) -> Option<Result<Project, ProjectError>> {
    let tag = format!("<metadata id=\"{SVG_METADATA_ID}\">");
    let mut rest = &svg[svg.find(&tag)? + tag.len()..];
    let mut json = String::new();
    loop {
        rest = rest.trim_start();
        let Some(after) = rest.strip_prefix("<![CDATA[") else {
            break;
        };
        let end = after.find("]]>")?;
        json.push_str(&after[..end]);
        rest = &after[end + 3..];
    }
    Some(Project::from_json(&json))
}

/// Une plaque et ses perforations, prêtes à exporter.
pub struct Drawing<'a> {
    pub project: &'a Project,
    pub holes: &'a [Hole],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_numbers() {
        assert_eq!(num(12.5, 4), "12.5");
        assert_eq!(num(3.0, 4), "3");
        assert_eq!(num(-0.00001, 4), "0");
        assert_eq!(num(0.123456, 4), "0.1235");
    }
}
