//! Contrôle des valeurs du projet avant calcul. Rien n'est corrigé : une
//! valeur hors domaine donne une erreur (motif non calculé) ou un
//! avertissement qui dit ce que le moteur en fait.

use crate::geom::Vec2;
use crate::issue::Issue;
use crate::project::{
    Curve, FeatureShape, Geometry, GridPattern, Influence, Layout, Outline, Project, ShapeKind,
    Zone,
};
use crate::raster::Luma;

pub const MAX_POLYGON_SIDES: u32 = 64;
pub const MAX_PATH_ROWS: u32 = 50;
/// Dépouille maximale (degrés).
pub const MAX_DRAFT: f64 = 45.0;
/// Dalles au plus par direction.
pub const MAX_TILES: u32 = 20;

/// Erreurs et avertissements de saisie.
pub fn validate(p: &Project) -> Vec<Issue> {
    let mut out = Vec::new();

    // Plaque.
    let pl = &p.plate;
    positive(pl.width, "plate.width", &mut out);
    if pl.outline != Outline::Circle {
        positive(pl.height, "plate.height", &mut out);
    }
    if pl.outline == Outline::Custom && pl.custom.len() < 3 {
        out.push(Issue::error("plate.custom_missing", "plate.outline").count(pl.custom.len()));
    }
    match &pl.zone {
        Zone::Plate => {}
        Zone::Circle { center, diameter } => {
            point(*center, "plate.zone", &mut out);
            positive(*diameter, "plate.zone.diameter", &mut out);
        }
        Zone::Rectangle {
            center,
            width,
            height,
            corner,
        } => {
            point(*center, "plate.zone", &mut out);
            positive(*width, "plate.zone.width", &mut out);
            positive(*height, "plate.zone.height", &mut out);
            non_negative(*corner, "plate.zone.corner", &mut out);
        }
    }
    for f in &p.features {
        let base = format!("features.{}", f.id);
        non_negative(f.clearance, &format!("{base}.clearance"), &mut out);
        match &f.shape {
            FeatureShape::Circle { center, diameter } => {
                point(*center, &base, &mut out);
                positive(*diameter, &format!("{base}.diameter"), &mut out);
            }
            FeatureShape::Rectangle {
                center,
                width,
                height,
                corner,
            } => {
                point(*center, &base, &mut out);
                positive(*width, &format!("{base}.width"), &mut out);
                positive(*height, &format!("{base}.height"), &mut out);
                non_negative(*corner, &format!("{base}.corner"), &mut out);
            }
            FeatureShape::Curve(c) => {
                curve(c, &base, &mut out);
                if !c.closed {
                    out.push(Issue::error("controller.curve_too_short", base.clone()).count(0));
                }
            }
        }
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
        Layout::Random(r) => positive(r.spacing, "layout.spacing", &mut out),
        Layout::Spiral(l) => {
            positive(l.pitch, "layout.pitch", &mut out);
            positive(l.spacing, "layout.spacing", &mut out);
            non_negative(l.start, "layout.start", &mut out);
            finite(l.offset_x, "layout.offset_x", &mut out);
            finite(l.offset_y, "layout.offset_y", &mut out);
        }
        Layout::Concentric(l) => {
            positive(l.pitch, "layout.pitch", &mut out);
            positive(l.spacing, "layout.spacing", &mut out);
            non_negative(l.start, "layout.start", &mut out);
            finite(l.phase, "layout.phase", &mut out);
            finite(l.offset_x, "layout.offset_x", &mut out);
            finite(l.offset_y, "layout.offset_y", &mut out);
        }
        Layout::Fibonacci(l) => {
            positive(l.spacing, "layout.spacing", &mut out);
            finite(l.divergence, "layout.divergence", &mut out);
            finite(l.offset_x, "layout.offset_x", &mut out);
            finite(l.offset_y, "layout.offset_y", &mut out);
        }
        Layout::CrossHatch(l) => {
            positive(l.line_spacing, "layout.line_spacing", &mut out);
            positive(l.step, "layout.step", &mut out);
            finite(l.angle, "layout.angle", &mut out);
            finite(l.cross, "layout.cross", &mut out);
            finite(l.offset_x, "layout.offset_x", &mut out);
            finite(l.offset_y, "layout.offset_y", &mut out);
        }
        Layout::Path(l) => {
            positive(l.spacing, "layout.spacing", &mut out);
            non_negative(l.row_gap, "layout.row_gap", &mut out);
            if !(1..=MAX_PATH_ROWS).contains(&l.rows) {
                out.push(Issue::error("value.out_of_range", "layout.rows").value(l.rows as f64));
            }
            for (i, c) in l.paths.iter().enumerate() {
                curve(c, &format!("layout.paths.{i}"), &mut out);
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
    let draft = p.fabrication.draft;
    if !(draft.is_finite() && draft.abs() <= MAX_DRAFT) {
        out.push(Issue::error("fabrication.draft_range", "fabrication.draft").value(MAX_DRAFT));
    }
    for (k, field) in ["fabrication.tiles_x", "fabrication.tiles_y"]
        .into_iter()
        .enumerate()
    {
        let n = p.fabrication.tiles[k];
        if !(1..=MAX_TILES).contains(&n) {
            out.push(Issue::error("fabrication.tiles_range", field).value(MAX_TILES as f64));
        }
    }

    // Contrôleurs.
    for c in &p.controllers {
        let base = format!("controllers.{}", c.id);
        let g = format!("{base}.geometry");
        let image = matches!(c.geometry, Geometry::Image { .. });
        match &c.geometry {
            Geometry::Point { at } => point(*at, &g, &mut out),
            Geometry::Line { a, b } => {
                point(*a, &g, &mut out);
                point(*b, &g, &mut out);
            }
            Geometry::Curve(cv) => curve(cv, &g, &mut out),
            Geometry::Circle { center, radius } => {
                point(*center, &g, &mut out);
                positive(*radius, &format!("{g}.radius"), &mut out);
            }
            Geometry::Ellipse {
                center,
                rx,
                ry,
                angle,
            } => {
                point(*center, &g, &mut out);
                positive(*rx, &format!("{g}.rx"), &mut out);
                positive(*ry, &format!("{g}.ry"), &mut out);
                finite(*angle, &format!("{g}.angle"), &mut out);
            }
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => {
                point(*center, &g, &mut out);
                positive(*radius, &format!("{g}.radius"), &mut out);
                finite(*angle, &format!("{g}.angle"), &mut out);
                if !(3..=MAX_POLYGON_SIDES).contains(sides) {
                    out.push(
                        Issue::error("shape.sides_out_of_range", format!("{g}.sides"))
                            .value(*sides as f64),
                    );
                }
            }
            Geometry::Image {
                at,
                width,
                height,
                raster,
            } => {
                point(*at, &g, &mut out);
                positive(*width, &format!("{g}.width"), &mut out);
                positive(*height, &format!("{g}.height"), &mut out);
                if Luma::decode(raster).is_none() {
                    out.push(Issue::error("controller.image_invalid", g.clone()));
                }
            }
        }
        if let Some(i) = &c.size {
            influence(i, &format!("{base}.size"), image, &mut out);
            non_negative(i.target, &format!("{base}.size.target"), &mut out);
            if let Some(cull) = &i.cull {
                if !(cull.amount.is_finite() && (0.0..=1.0).contains(&cull.amount)) {
                    out.push(
                        Issue::error("value.out_of_range", format!("{base}.size.cull"))
                            .value(cull.amount),
                    );
                }
            }
        }
        if let Some(i) = &c.spacing {
            influence(i, &format!("{base}.spacing"), image, &mut out);
            positive(i.target, &format!("{base}.spacing.target"), &mut out);
        }
        if let Some(i) = &c.angle {
            influence(i, &format!("{base}.angle"), image, &mut out);
            finite(i.target, &format!("{base}.angle.target"), &mut out);
            non_negative(i.step, &format!("{base}.angle.step"), &mut out);
        }
    }

    out
}

fn influence(i: &Influence, base: &str, image: bool, out: &mut Vec<Issue>) {
    if !image {
        positive(i.reach, &format!("{base}.reach"), out);
    }
    non_negative(i.contrast, &format!("{base}.contrast"), out);
}

fn point(p: Vec2, field: &str, out: &mut Vec<Issue>) {
    if !(p.x.is_finite() && p.y.is_finite()) {
        out.push(Issue::error("value.must_be_finite", field));
    }
}

/// Tracé : au moins 2 nœuds (3 s'il est fermé), congés positifs, un par nœud.
fn curve(c: &Curve, field: &str, out: &mut Vec<Issue>) {
    let min = if c.closed { 3 } else { 2 };
    if c.points.len() < min {
        out.push(Issue::error("controller.curve_too_short", field).count(c.points.len()));
    }
    for &q in &c.points {
        point(q, field, out);
    }
    if !c.fillets.is_empty() && c.fillets.len() != c.points.len() {
        out.push(Issue::error("controller.fillets_invalid", field));
    }
    if c.fillets.iter().any(|r| !(r.is_finite() && *r >= 0.0)) {
        out.push(Issue::error(
            "value.must_be_non_negative",
            format!("{field}.fillets"),
        ));
    }
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
    fn new_controllers_are_checked() {
        use crate::project::{Controller, Raster};
        let mut p = Project::example();
        p.controllers.push(Controller {
            spacing: Some(Influence::new(10.0, 0.0)),
            ..Controller::new(
                2,
                Geometry::Curve(Curve {
                    points: vec![Vec2::ZERO, Vec2::new(1.0, 0.0)],
                    closed: true,
                    fillets: vec![1.0],
                }),
            )
        });
        p.controllers.push(Controller {
            size: Some(Influence::new(0.0, 4.0)),
            ..Controller::new(
                3,
                Geometry::Image {
                    at: Vec2::ZERO,
                    width: 10.0,
                    height: 10.0,
                    raster: Raster {
                        width: 3,
                        height: 3,
                        luma: "AAAA".into(),
                    },
                },
            )
        });
        let codes: Vec<(&str, String)> = validate(&p)
            .into_iter()
            .map(|i| (i.code, i.field.unwrap_or_default()))
            .collect();
        assert!(codes.contains(&(
            "controller.curve_too_short",
            "controllers.2.geometry".into()
        )));
        assert!(codes.contains(&(
            "controller.fillets_invalid",
            "controllers.2.geometry".into()
        )));
        assert!(codes.contains(&(
            "value.must_be_positive",
            "controllers.2.spacing.target".into()
        )));
        assert!(codes.contains(&("controller.image_invalid", "controllers.3.geometry".into())));
        // Une image n'a pas de portée : 0 est accepté.
        assert!(!codes.iter().any(|(_, f)| f == "controllers.3.size.reach"));
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
