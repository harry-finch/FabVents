//! Export DXF R12 (AC1009), le format le plus largement lu (LightBurn,
//! Inkscape, FreeCAD, Fusion 360, VCarve, logiciels de jet d'eau). Unités en
//! millimètres (`$INSUNITS = 4`), deux calques :
//!
//! - `CUT_INNER` (couleur 5, bleu) : perforations, à découper d'abord ;
//! - `CUT_OUTER` (couleur 1, rouge) : contour de la plaque, en dernier.
//!
//! Cercles en `CIRCLE`, autres contours en `POLYLINE` fermées dont les arcs
//! sont exacts (code 42, *bulge*).

use super::{num, Drawing};
use crate::contour::Contour;
use std::fmt::Write;

const DECIMALS: usize = 6;
pub const LAYER_INNER: &str = "CUT_INNER";
pub const LAYER_OUTER: &str = "CUT_OUTER";

struct Dxf(String);

impl Dxf {
    fn pair(&mut self, code: u16, value: impl std::fmt::Display) {
        let _ = write!(self.0, "{code:>3}\n{value}\n");
    }

    fn real(&mut self, code: u16, v: f64) {
        let s = num(v, DECIMALS);
        // Un réel DXF garde son point décimal (`0.0`, `12.0`).
        if s.contains('.') {
            self.pair(code, s);
        } else {
            self.pair(code, format!("{s}.0"));
        }
    }

    fn contour(&mut self, layer: &str, c: &Contour) {
        match c {
            Contour::Circle { center, radius } => {
                self.pair(0, "CIRCLE");
                self.pair(8, layer);
                self.real(10, center.x);
                self.real(20, center.y);
                self.real(30, 0.0);
                self.real(40, *radius);
            }
            Contour::Path(v) if v.len() >= 2 => {
                self.pair(0, "POLYLINE");
                self.pair(8, layer);
                self.pair(66, 1);
                self.real(10, 0.0);
                self.real(20, 0.0);
                self.real(30, 0.0);
                self.pair(70, 1);
                for x in v {
                    self.pair(0, "VERTEX");
                    self.pair(8, layer);
                    self.real(10, x.p.x);
                    self.real(20, x.p.y);
                    self.real(30, 0.0);
                    if x.bulge != 0.0 {
                        self.real(42, x.bulge);
                    }
                }
                self.pair(0, "SEQEND");
                self.pair(8, layer);
            }
            Contour::Path(_) => {}
        }
    }
}

pub fn dxf(d: &Drawing) -> String {
    let ext = d.project.plate.extent();
    let mut w = Dxf(String::with_capacity(1024 + d.holes.len() * 96));
    w.pair(999, format!("FabVents {}", crate::ENGINE_VERSION));

    w.pair(0, "SECTION");
    w.pair(2, "HEADER");
    w.pair(9, "$ACADVER");
    w.pair(1, "AC1009");
    w.pair(9, "$INSUNITS");
    w.pair(70, 4);
    w.pair(9, "$MEASUREMENT");
    w.pair(70, 1);
    w.pair(9, "$EXTMIN");
    w.real(10, 0.0);
    w.real(20, 0.0);
    w.pair(9, "$EXTMAX");
    w.real(10, ext.x);
    w.real(20, ext.y);
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "TABLES");
    w.pair(0, "TABLE");
    w.pair(2, "LTYPE");
    w.pair(70, 1);
    w.pair(0, "LTYPE");
    w.pair(2, "CONTINUOUS");
    w.pair(70, 0);
    w.pair(3, "Solid line");
    w.pair(72, 65);
    w.pair(73, 0);
    w.real(40, 0.0);
    w.pair(0, "ENDTAB");
    w.pair(0, "TABLE");
    w.pair(2, "LAYER");
    w.pair(70, 2);
    for (name, color) in [(LAYER_INNER, 5), (LAYER_OUTER, 1)] {
        w.pair(0, "LAYER");
        w.pair(2, name);
        w.pair(70, 0);
        w.pair(62, color);
        w.pair(6, "CONTINUOUS");
    }
    w.pair(0, "ENDTAB");
    w.pair(0, "ENDSEC");

    w.pair(0, "SECTION");
    w.pair(2, "ENTITIES");
    for h in d.holes {
        w.contour(LAYER_INNER, &h.contour());
    }
    w.contour(LAYER_OUTER, &d.project.plate.contour());
    w.pair(0, "ENDSEC");
    w.pair(0, "EOF");
    w.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Pipeline;
    use crate::project::{Project, ShapeKind};

    fn export(p: &Project) -> (String, usize) {
        let mut pl = Pipeline::new();
        let f = pl.compute(p);
        (
            dxf(&Drawing {
                project: p,
                holes: f.holes,
            }),
            f.holes.len(),
        )
    }

    #[test]
    fn structure_and_units() {
        let (out, n) = export(&Project::example());
        assert!(out.contains("$INSUNITS\n 70\n4\n"));
        assert_eq!(out.matches("\nCIRCLE\n").count(), n);
        // Contour de la plaque : polyligne à 4 coins arrondis (8 sommets, 4 arcs).
        assert_eq!(out.matches("\nPOLYLINE\n").count(), 1);
        assert_eq!(out.matches("\n 42\n").count(), 4);
        assert!(out.ends_with("  0\nEOF\n"));
        // Chaque code est suivi d'une valeur : nombre de lignes pair.
        assert_eq!(out.lines().count() % 2, 0);
    }

    #[test]
    fn slots_have_exact_arcs() {
        let mut p = Project::example();
        p.shape.kind = ShapeKind::Slot { aspect: 0.4 };
        let (out, n) = export(&p);
        assert_eq!(out.matches("\nPOLYLINE\n").count(), n + 1);
        assert_eq!(out.matches("\n 42\n1.0\n").count(), 2 * n);
    }
}
