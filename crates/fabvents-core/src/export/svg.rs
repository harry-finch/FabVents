//! Export SVG en millimètres (`width="200mm"`, `viewBox` en mm), traits fins
//! sans remplissage : rouge pour le contour, bleu pour les perforations
//! (couleurs reconnues comme calques par LightBurn et la plupart des logiciels
//! de découpe). Le projet complet est embarqué dans `<metadata>` : rouvrir
//! le SVG dans FabVents redonne le motif modifiable.

use super::{num, Drawing, SVG_METADATA_ID};
use crate::contour::Contour;
use crate::geom::{bulge_angle, bulge_arc, Vec2};
use std::fmt::Write;

const DECIMALS: usize = 4;
pub const COLOR_OUTER: &str = "#ff0000";
pub const COLOR_INNER: &str = "#0000ff";

pub fn svg(d: &Drawing) -> String {
    let p = d.project;
    let ext = p.plate.extent();
    let h = ext.y;
    let mut s = String::with_capacity(128 + d.holes.len() * 64);
    let _ = writeln!(s, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}mm" height="{hh}mm" viewBox="0 0 {w} {hh}">"#,
        w = num(ext.x, DECIMALS),
        hh = num(h, DECIMALS)
    );
    let title = if p.name.trim().is_empty() {
        "FabVents"
    } else {
        p.name.trim()
    };
    let _ = writeln!(s, "  <title>{}</title>", escape(title));
    let _ = writeln!(
        s,
        "  <desc>FabVents {} — {} perforations</desc>",
        crate::ENGINE_VERSION,
        d.holes.len()
    );
    let json = p.to_json().replace("]]>", "]]]]><![CDATA[>");
    let _ = writeln!(
        s,
        "  <metadata id=\"{SVG_METADATA_ID}\"><![CDATA[{json}]]></metadata>"
    );

    let _ = writeln!(
        s,
        r#"  <g id="cut-outer" fill="none" stroke="{COLOR_OUTER}" stroke-width="0.1">"#
    );
    contour(&mut s, &p.plate.contour(), h);
    let _ = writeln!(s, "  </g>");
    let _ = writeln!(
        s,
        r#"  <g id="cut-inner" fill="none" stroke="{COLOR_INNER}" stroke-width="0.1">"#
    );
    for c in d.inner_contours() {
        contour(&mut s, &c, h);
    }
    let _ = writeln!(s, "  </g>");
    s.push_str("</svg>\n");
    s
}

/// Un contour, Y retourné (le SVG a l'axe Y vers le bas).
fn contour(s: &mut String, c: &Contour, h: f64) {
    let pt = |v: Vec2| format!("{} {}", num(v.x, DECIMALS), num(h - v.y, DECIMALS));
    match c {
        Contour::Circle { center, radius } => {
            let _ = writeln!(
                s,
                r#"    <circle cx="{}" cy="{}" r="{}"/>"#,
                num(center.x, DECIMALS),
                num(h - center.y, DECIMALS),
                num(*radius, DECIMALS)
            );
        }
        Contour::Path(v) if v.len() >= 2 => {
            let n = v.len();
            let mut d = format!("M{}", pt(v[0].p));
            for i in 0..n {
                let (a, b) = (v[i], v[(i + 1) % n]);
                if a.bulge == 0.0 {
                    if i + 1 < n {
                        let _ = write!(d, "L{}", pt(b.p));
                    }
                } else {
                    let (_, r) = bulge_arc(a.p, b.p, a.bulge);
                    let large = u8::from(bulge_angle(a.bulge).abs() > std::f64::consts::PI);
                    // Un arc trigonométrique (Y vers le haut) devient un arc de
                    // sens négatif une fois Y retourné : drapeau de balayage 0.
                    let sweep = u8::from(a.bulge < 0.0);
                    let rr = num(r, DECIMALS);
                    let _ = write!(d, "A{rr} {rr} 0 {large} {sweep} {}", pt(b.p));
                }
            }
            d.push('Z');
            let _ = writeln!(s, r#"    <path d="{d}"/>"#);
        }
        Contour::Path(_) => {}
    }
}

fn escape(t: &str) -> String {
    t.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::project_from_svg;
    use crate::pipeline::Pipeline;
    use crate::project::{Project, ShapeKind};

    #[test]
    fn svg_embeds_the_project() {
        let mut p = Project::example();
        p.name = "Grille <test> ]]> & co".into();
        let mut pl = Pipeline::new();
        let f = pl.compute(&p);
        let out = svg(&Drawing::new(&p, &f));
        assert!(out.contains(r#"width="200mm""#));
        assert!(out.contains("&lt;test&gt;"));
        let back = project_from_svg(&out).unwrap().unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn circles_stay_circles_and_slots_use_arcs() {
        let mut p = Project::example();
        let mut pl = Pipeline::new();
        let n = pl.compute(&p).holes.len();
        let f = pl.compute(&p);
        let out = svg(&Drawing::new(&p, &f));
        assert_eq!(out.matches("<circle").count(), n);
        p.shape.kind = ShapeKind::Slot { aspect: 0.4 };
        let f = pl.compute(&p);
        let out = svg(&Drawing::new(&p, &f));
        assert!(out.contains("A"));
        assert_eq!(out.matches("<path").count(), f.holes.len() + 1);
    }

    #[test]
    fn foreign_svg_has_no_project() {
        assert!(project_from_svg("<svg></svg>").is_none());
    }
}
