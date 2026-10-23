//! Contrôle des valeurs du projet avant calcul. Rien n'est corrigé : une
//! valeur hors domaine donne une erreur (motif non calculé) ou un
//! avertissement qui dit ce que le moteur en fait.

use crate::issue::Issue;
use crate::project::{Geometry, GridPattern, Layout, Outline, Project, ShapeKind};

pub const MAX_POLYGON_SIDES: u32 = 64;

/// Erreurs et avertissements de saisie.
pub fn validate(p: &Project) -> Vec<Issue> {
    let mut out = Vec::new();

    // Plaque.
    let pl = &p.plate;
    positive(pl.width, "plate.width", &mut out);
    if pl.outline == Outline::Rectangle {
        positive(pl.height, "plate.height", &mut out);
    }
    positive(pl.thickness, "plate.thickness", &mut out);
    non_negative(pl.margin, "plate.margin", &mut out);
    if pl.outline == Outline::Rectangle {
        non_negative(pl.corner_radius, "plate.corner_radius", &mut out);
        let max = pl.width.min(pl.height) / 2.0;
        if pl.corner_radius > max && max > 0.0 {
            out.push(
                Issue::warning("plate.corner_limited")
                    .field("plate.corner_radius")
                    .value(max),
            );
        }
    }
    let ext = pl.extent();
    if pl.margin.is_finite() && ext.x > 0.0 && ext.y > 0.0 && 2.0 * pl.margin >= ext.x.min(ext.y) {
        out.push(Issue::error("plate.margin_too_large", "plate.margin").value(pl.margin));
    }

    // Disposition.
    match &p.layout {
        Layout::Grid(g) => {
            positive(g.pitch_x, "layout.pitch_x", &mut out);
            if g.pattern != GridPattern::Hexagonal {
                positive(g.pitch_y, "layout.pitch_y", &mut out);
            }
            for (v, f) in [
                (g.angle, "layout.angle"),
                (g.offset_x, "layout.offset_x"),
                (g.offset_y, "layout.offset_y"),
                (g.row_twist, "layout.row_twist"),
                (g.col_twist, "layout.col_twist"),
            ] {
                finite(v, f, &mut out);
            }
        }
    }

    // Forme.
    let s = &p.shape;
    positive(s.size, "shape.size", &mut out);
    finite(s.rotation, "shape.rotation", &mut out);
    match s.kind {
        ShapeKind::Circle => {}
        ShapeKind::Rectangle { aspect, corner } => {
            positive(aspect, "shape.aspect", &mut out);
            non_negative(corner, "shape.corner", &mut out);
            let max = s.size.min(s.size * aspect) / 2.0;
            if corner > max && max > 0.0 {
                out.push(
                    Issue::warning("shape.corner_limited")
                        .field("shape.corner")
                        .value(max),
                );
            }
        }
        ShapeKind::Polygon { sides } => {
            if !(3..=MAX_POLYGON_SIDES).contains(&sides) {
                out.push(
                    Issue::error("shape.sides_out_of_range", "shape.sides").value(sides as f64),
                );
            }
        }
        ShapeKind::Star { points, inner } => {
            if !(3..=MAX_POLYGON_SIDES).contains(&points) {
                out.push(
                    Issue::error("shape.sides_out_of_range", "shape.points").value(points as f64),
                );
            }
            if !(inner.is_finite() && inner > 0.0 && inner < 1.0) {
                out.push(Issue::error("shape.ratio_out_of_range", "shape.inner").value(inner));
            }
        }
        ShapeKind::Slot { aspect } => {
            if !(aspect.is_finite() && aspect > 0.0 && aspect <= 1.0) {
                out.push(Issue::error("shape.ratio_out_of_range", "shape.aspect").value(aspect));
            }
        }
    }

    // Fabrication.
    non_negative(p.fabrication.min_size, "fabrication.min_size", &mut out);
    non_negative(p.fabrication.min_bridge, "fabrication.min_bridge", &mut out);

    // Contrôleurs.
    for c in &p.controllers {
        let base = format!("controllers.{}", c.id);
        let pts = match c.geometry {
            Geometry::Point { at } => vec![at],
            Geometry::Line { a, b } => vec![a, b],
        };
        if pts.iter().any(|q| !(q.x.is_finite() && q.y.is_finite())) {
            out.push(Issue::error(
                "value.must_be_finite",
                format!("{base}.geometry"),
            ));
        }
        if let Some(i) = &c.size {
            positive(i.reach, &format!("{base}.size.reach"), &mut out);
            non_negative(i.target, &format!("{base}.size.target"), &mut out);
        }
        if let Some(i) = &c.angle {
            positive(i.reach, &format!("{base}.angle.reach"), &mut out);
            finite(i.target, &format!("{base}.angle.target"), &mut out);
        }
    }

    out
}

fn positive(v: f64, field: &str, out: &mut Vec<Issue>) {
    if !(v.is_finite() && v > 0.0) {
        out.push(Issue::error("value.must_be_positive", field).value(v));
    }
}

fn non_negative(v: f64, field: &str, out: &mut Vec<Issue>) {
    if !(v.is_finite() && v >= 0.0) {
        out.push(Issue::error("value.must_be_non_negative", field).value(v));
    }
}

fn finite(v: f64, field: &str, out: &mut Vec<Issue>) {
    if !v.is_finite() {
        out.push(Issue::error("value.must_be_finite", field));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Severity;
    use crate::project::{Grid, Shape};

    #[test]
    fn example_is_valid() {
        assert!(validate(&Project::example()).is_empty());
    }

    #[test]
    fn zero_pitch_is_an_error_not_a_clamp() {
        let mut p = Project::example();
        p.layout = Layout::Grid(Grid {
            pitch_x: 0.0,
            ..Grid::default()
        });
        let issues = validate(&p);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].field.as_deref(), Some("layout.pitch_x"));
    }

    #[test]
    fn oversized_corner_is_reported() {
        let mut p = Project::example();
        p.shape = Shape {
            kind: ShapeKind::Rectangle {
                aspect: 0.5,
                corner: 4.0,
            },
            size: 10.0,
            rotation: 0.0,
        };
        let issues = validate(&p);
        assert_eq!(issues[0].code, "shape.corner_limited");
        assert_eq!(issues[0].value, Some(2.5));
    }
}
