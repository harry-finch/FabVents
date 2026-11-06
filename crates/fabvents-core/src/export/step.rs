//! Export STEP (ISO 10303-21, AP214) : chaque pièce est un solide B-Rep
//! explicite (`MANIFOLD_SOLID_BREP`), sans géométrie constructive.
//!
//! - faces du dessus (z = épaisseur) et du dessous (z = 0) planes, bordées
//!   par le contour et les trous ;
//! - une face de paroi par segment : plan pour un segment droit (incliné en
//!   dépouille), cylindre pour un arc, cône pour un arc en dépouille ;
//! - arcs et cercles exacts (`CIRCLE`), un cercle complet coupé en deux
//!   demi-arcs (aucune couture à recoudre par l'importeur).
//!
//! Les boucles arrivent orientées matière à gauche (voir `brep`) ; le fond
//! de chaque boucle a le même nombre de sommets que le dessus.

use super::num;
use crate::brep::{Loop, Seg};
use crate::geom::Vec2;
use crate::pipeline::Frame;
use crate::project::Project;
use crate::solid::{self, SolidPlan};
use std::fmt::Write;

/// Paroi : boucle du dessus et boucle du dessous correspondante.
#[derive(Clone, Debug)]
pub struct Wall {
    pub top: Loop,
    pub bottom: Loop,
}

impl Wall {
    pub fn vertical(top: Loop) -> Wall {
        Wall {
            bottom: top.clone(),
            top,
        }
    }
}

/// Une pièce : contour extérieur (sens trigonométrique) et trous (sens horaire).
#[derive(Clone, Debug)]
pub struct Solid {
    pub outer: Wall,
    pub holes: Vec<Wall>,
}

/// Fichier STEP d'un ensemble de pièces de même épaisseur, dans un seul
/// produit. `timestamp` : date ISO 8601 de l'en-tête (le moteur n'a pas
/// d'horloge).
pub fn step(name: &str, solids: &[Solid], thickness: f64, timestamp: &str) -> String {
    let mut w = Writer::new();
    let origin = w.e("CARTESIAN_POINT('',(0.,0.,0.))".into());
    let z = w.e("DIRECTION('',(0.,0.,1.))".into());
    let mz = w.e("DIRECTION('',(0.,0.,-1.))".into());
    let x = w.e("DIRECTION('',(1.,0.,0.))".into());
    let world = w.e(format!("AXIS2_PLACEMENT_3D('',#{origin},#{z},#{x})"));
    let dirs = Dirs { z, mz, x };

    let mut items = vec![world];
    for (k, s) in solids.iter().enumerate() {
        items.push(w.solid(s, thickness, &dirs, &format!("{} {}", name, k + 1)));
    }

    // Contexte : millimètres, radians, incertitude 1e-7 mm.
    let mm = w.e("( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) )".into());
    let rad = w.e("( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) )".into());
    let sr = w.e("( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() )".into());
    let unc = w.e(format!(
        "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#{mm},'distance_accuracy_value','confusion accuracy')"
    ));
    let ctx = w.e(format!(
        "( GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{unc})) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{mm},#{rad},#{sr})) REPRESENTATION_CONTEXT('',''))"
    ));
    let list = items
        .iter()
        .map(|i| format!("#{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let pname = text(name);
    let rep = w.e(format!(
        "ADVANCED_BREP_SHAPE_REPRESENTATION('{pname}',({list}),#{ctx})"
    ));

    // Produit.
    let app =
        w.e("APPLICATION_CONTEXT('core data for automotive mechanical design processes')".into());
    w.e(format!(
        "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2000,#{app})"
    ));
    let pctx = w.e(format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
    let product = w.e(format!("PRODUCT('{pname}','{pname}','',(#{pctx}))"));
    w.e(format!(
        "PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#{product}))"
    ));
    let form = w.e(format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"));
    let dctx = w.e(format!(
        "PRODUCT_DEFINITION_CONTEXT('part definition',#{app},'design')"
    ));
    let def = w.e(format!("PRODUCT_DEFINITION('design','',#{form},#{dctx})"));
    let shape = w.e(format!("PRODUCT_DEFINITION_SHAPE('','',#{def})"));
    w.e(format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"));

    let file = text(&format!("{}.step", file_stem(name)));
    let version = crate::ENGINE_VERSION;
    let ts = text(timestamp);
    let mut out = String::with_capacity(w.out.len() + 512);
    out.push_str("ISO-10303-21;\nHEADER;\n");
    out.push_str("FILE_DESCRIPTION(('FabVents perforated plate'),'2;1');\n");
    let _ = writeln!(
        out,
        "FILE_NAME('{file}','{ts}',(''),(''),'FabVents {version}','FabVents','');"
    );
    out.push_str("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\nENDSEC;\nDATA;\n");
    out.push_str(&w.out);
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out
}

/// Pièces d'un projet calculé : celles du calcul si une dépouille ou des
/// dalles l'ont déjà demandé, sinon calculées ici.
fn plan_of(project: &Project, frame: &Frame) -> Result<SolidPlan, &'static str> {
    if frame.has_errors() {
        return Err("export.invalid_project");
    }
    // Un trou de fixation à cheval sur le bord ferait un solide invalide.
    if frame.issues.iter().any(|i| i.code == "features.outside") {
        return Err("export.step_features_outside");
    }
    let plan = if frame.plan.tiles.is_empty() && frame.plan.issues.is_empty() {
        solid::plan(project, frame.holes, frame.complex, frame.cut_contours)
    } else {
        frame.plan.clone()
    };
    if plan.blocked() {
        return Err("export.step_blocked");
    }
    Ok(plan)
}

/// Nom du projet pour un fichier : minuscules, chiffres et tirets.
pub fn file_stem(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars().flat_map(|c| c.to_lowercase()) {
        let c = match ch {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !out.ends_with('-') {
            out.push(c);
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "fabvents".into()
    } else {
        out
    }
}

/// Toute la plaque dans un seul fichier STEP (une pièce par dalle, les
/// îlots à part).
pub fn step_file(
    project: &Project,
    frame: &Frame,
    timestamp: &str,
) -> Result<String, &'static str> {
    let plan = plan_of(project, frame)?;
    let solids: Vec<Solid> = plan.tiles.into_iter().flat_map(|t| t.solids).collect();
    Ok(step(
        &display_name(project),
        &solids,
        plan.thickness,
        timestamp,
    ))
}

/// Une dalle par fichier STEP, dans une archive ZIP
/// (`<projet>-c<colonne>r<rangée>.step`, numérotés à partir de 1, rangée 1
/// en bas).
pub fn step_zip(
    project: &Project,
    frame: &Frame,
    timestamp: &str,
) -> Result<Vec<u8>, &'static str> {
    let plan = plan_of(project, frame)?;
    let stem = file_stem(&project.name);
    let name = display_name(project);
    let files: Vec<(String, Vec<u8>)> = plan
        .tiles
        .iter()
        .map(|t| {
            let tile = format!("{name} c{}r{}", t.col + 1, t.row + 1);
            (
                format!("{stem}-c{}r{}.step", t.col + 1, t.row + 1),
                step(&tile, &t.solids, plan.thickness, timestamp).into_bytes(),
            )
        })
        .collect();
    Ok(super::zip::zip(&files))
}

fn display_name(project: &Project) -> String {
    if project.name.trim().is_empty() {
        "FabVents".into()
    } else {
        project.name.trim().to_string()
    }
}

/// Chaîne STEP : apostrophes doublées, barre oblique inverse doublée,
/// caractères hors ASCII en `\X2\…\X0\` (UTF-16).
fn text(s: &str) -> String {
    let mut out = String::new();
    let mut wide = String::new();
    let flush = |out: &mut String, wide: &mut String| {
        if !wide.is_empty() {
            out.push_str("\\X2\\");
            out.push_str(wide);
            out.push_str("\\X0\\");
            wide.clear();
        }
    };
    for ch in s.chars() {
        if ch.is_ascii() && !ch.is_ascii_control() {
            flush(&mut out, &mut wide);
            match ch {
                '\'' => out.push_str("''"),
                '\\' => out.push_str("\\\\"),
                c => out.push(c),
            }
        } else if !ch.is_ascii_control() {
            let mut buf = [0u16; 2];
            for u in ch.encode_utf16(&mut buf) {
                let _ = write!(wide, "{u:04X}");
            }
        }
    }
    flush(&mut out, &mut wide);
    out
}

/// Réel STEP : toujours un point décimal (`12.`, `0.5`), 10 décimales :
/// un sommet doit rester à moins de l'incertitude déclarée (1e-7 mm) de
/// l'intersection exacte de ses deux courbes, sans quoi OpenCascade voit une
/// boucle qui se recoupe aux raccords entre arcs concaves.
fn real(v: f64) -> String {
    let s = num(v, 10);
    if s.contains('.') {
        s
    } else {
        format!("{s}.")
    }
}

struct Dirs {
    z: usize,
    mz: usize,
    x: usize,
}

struct Writer {
    out: String,
    next: usize,
}

/// Arêtes et sommets d'une boucle à une hauteur donnée.
struct Ring {
    verts: Vec<usize>,
    edges: Vec<usize>,
}

impl Writer {
    fn new() -> Self {
        Self {
            out: String::new(),
            next: 1,
        }
    }

    fn e(&mut self, body: String) -> usize {
        let id = self.next;
        self.next += 1;
        let _ = writeln!(self.out, "#{id}={body};");
        id
    }

    fn point(&mut self, p: Vec2, z: f64) -> usize {
        self.e(format!(
            "CARTESIAN_POINT('',({},{},{}))",
            real(p.x),
            real(p.y),
            real(z)
        ))
    }

    fn direction(&mut self, x: f64, y: f64, z: f64) -> usize {
        self.e(format!(
            "DIRECTION('',({},{},{}))",
            real(x),
            real(y),
            real(z)
        ))
    }

    fn placement(&mut self, at: Vec2, z: f64, axis: usize, refd: usize) -> usize {
        let p = self.point(at, z);
        self.e(format!("AXIS2_PLACEMENT_3D('',#{p},#{axis},#{refd})"))
    }

    /// Segment droit de `a` à `b` (en 3D).
    fn line(&mut self, a: (Vec2, f64), b: (Vec2, f64)) -> usize {
        let (dx, dy, dz) = (b.0.x - a.0.x, b.0.y - a.0.y, b.1 - a.1);
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        let p = self.point(a.0, a.1);
        let d = self.direction(dx / len, dy / len, dz / len);
        let v = self.e(format!("VECTOR('',#{d},{})", real(len)));
        self.e(format!("LINE('',#{p},#{v})"))
    }

    fn ring(&mut self, l: &Loop, z: f64, dirs: &Dirs) -> Ring {
        let n = l.len();
        let verts: Vec<usize> = l
            .pts
            .iter()
            .map(|&p| {
                let pt = self.point(p, z);
                self.e(format!("VERTEX_POINT('',#{pt})"))
            })
            .collect();
        let edges = (0..n)
            .map(|i| {
                let (a, b) = (l.pts[i], l.pts[(i + 1) % n]);
                let (va, vb) = (verts[i], verts[(i + 1) % n]);
                match l.segs[i] {
                    Seg::Line => {
                        let c = self.line((a, z), (b, z));
                        self.e(format!("EDGE_CURVE('',#{va},#{vb},#{c},.T.)"))
                    }
                    // Cercle orienté dans le sens de l'arête (axe -Z pour un arc horaire).
                    Seg::Arc { c, r, ccw } => {
                        let axis = if ccw { dirs.z } else { dirs.mz };
                        let pl = self.placement(c, z, axis, dirs.x);
                        let circle = self.e(format!("CIRCLE('',#{pl},{})", real(r)));
                        self.e(format!("EDGE_CURVE('',#{va},#{vb},#{circle},.T.)"))
                    }
                }
            })
            .collect();
        Ring { verts, edges }
    }

    fn face(&mut self, bounds: &[(usize, bool)], surface: usize, same_sense: bool) -> usize {
        let b: Vec<String> = bounds
            .iter()
            .enumerate()
            .map(|(k, &(lp, _))| {
                let kind = if k == 0 {
                    "FACE_OUTER_BOUND"
                } else {
                    "FACE_BOUND"
                };
                format!("#{}", self.e(format!("{kind}('',#{lp},.T.)")))
            })
            .collect();
        let s = if same_sense { ".T." } else { ".F." };
        self.e(format!(
            "ADVANCED_FACE('',({}),#{surface},{s})",
            b.join(",")
        ))
    }

    fn edge_loop(&mut self, edges: &[(usize, bool)]) -> usize {
        let oe: Vec<String> = edges
            .iter()
            .map(|&(e, fwd)| {
                let o = if fwd { ".T." } else { ".F." };
                format!("#{}", self.e(format!("ORIENTED_EDGE('',*,*,#{e},{o})")))
            })
            .collect();
        self.e(format!("EDGE_LOOP('',({}))", oe.join(",")))
    }

    fn solid(&mut self, s: &Solid, t: f64, dirs: &Dirs, name: &str) -> usize {
        let walls: Vec<&Wall> = std::iter::once(&s.outer).chain(&s.holes).collect();
        let mut faces = Vec::new();
        let mut top_loops = Vec::new();
        let mut bottom_loops = Vec::new();
        for wall in walls {
            let top = self.ring(&wall.top, t, dirs);
            let bot = self.ring(&wall.bottom, 0.0, dirs);
            let n = wall.top.len();
            let verticals: Vec<usize> = (0..n)
                .map(|i| {
                    let c = self.line((wall.top.pts[i], t), (wall.bottom.pts[i], 0.0));
                    let (a, b) = (top.verts[i], bot.verts[i]);
                    self.e(format!("EDGE_CURVE('',#{a},#{b},#{c},.T.)"))
                })
                .collect();
            for i in 0..n {
                let j = (i + 1) % n;
                let lp = self.edge_loop(&[
                    (top.edges[i], false),
                    (verticals[i], true),
                    (bot.edges[i], true),
                    (verticals[j], false),
                ]);
                let (surface, same) = self.wall_surface(wall, i, t, dirs);
                faces.push(self.face(&[(lp, true)], surface, same));
            }
            let tl: Vec<(usize, bool)> = top.edges.iter().map(|&e| (e, true)).collect();
            top_loops.push(self.edge_loop(&tl));
            let bl: Vec<(usize, bool)> = bot.edges.iter().rev().map(|&e| (e, false)).collect();
            bottom_loops.push(self.edge_loop(&bl));
        }
        let top_plane = self.placement(Vec2::ZERO, t, dirs.z, dirs.x);
        let top_plane = self.e(format!("PLANE('',#{top_plane})"));
        let bounds: Vec<(usize, bool)> = top_loops.iter().map(|&l| (l, true)).collect();
        faces.push(self.face(&bounds, top_plane, true));
        let bottom_plane = self.placement(Vec2::ZERO, 0.0, dirs.z, dirs.x);
        let bottom_plane = self.e(format!("PLANE('',#{bottom_plane})"));
        let bounds: Vec<(usize, bool)> = bottom_loops.iter().map(|&l| (l, true)).collect();
        faces.push(self.face(&bounds, bottom_plane, false));
        let list = faces
            .iter()
            .map(|f| format!("#{f}"))
            .collect::<Vec<_>>()
            .join(",");
        let shell = self.e(format!("CLOSED_SHELL('',({list}))"));
        self.e(format!("MANIFOLD_SOLID_BREP('{}',#{shell})", text(name)))
    }

    /// Surface de la paroi `i` et accord de sa normale avec l'extérieur de
    /// la matière (à droite du parcours).
    fn wall_surface(&mut self, wall: &Wall, i: usize, t: f64, dirs: &Dirs) -> (usize, bool) {
        let n = wall.top.len();
        let (p0, p1) = (wall.top.pts[i], wall.top.pts[(i + 1) % n]);
        let q0 = wall.bottom.pts[i];
        match (wall.top.segs[i], wall.bottom.segs[i]) {
            (Seg::Arc { c, r, ccw }, Seg::Arc { r: rb, .. }) => {
                // Centre à gauche (côté matière) : la normale naturelle, qui
                // s'éloigne de l'axe, sort de la matière.
                let surface = if (r - rb).abs() < 1e-9 {
                    let pl = self.placement(c, 0.0, dirs.z, dirs.x);
                    self.e(format!("CYLINDRICAL_SURFACE('',#{pl},{})", real(r)))
                } else {
                    // Cône posé sur sa plus petite section, ouvert vers la plus grande.
                    let (z0, r0, axis, half) = if r < rb {
                        (t, r, dirs.mz, ((rb - r) / t).atan())
                    } else {
                        (0.0, rb, dirs.z, ((r - rb) / t).atan())
                    };
                    let pl = self.placement(c, z0, axis, dirs.x);
                    self.e(format!(
                        "CONICAL_SURFACE('',#{pl},{},{})",
                        real(r0),
                        real(half)
                    ))
                };
                (surface, ccw)
            }
            _ => {
                // Plan passant par l'arête du dessus et le fond, normale vers
                // l'extérieur de la matière.
                let e1 = [p1.x - p0.x, p1.y - p0.y, 0.0];
                let w = [q0.x - p0.x, q0.y - p0.y, -t];
                let mut nrm = [
                    e1[1] * w[2] - e1[2] * w[1],
                    e1[2] * w[0] - e1[0] * w[2],
                    e1[0] * w[1] - e1[1] * w[0],
                ];
                let out = Vec2::new(e1[0], e1[1]).right();
                if nrm[0] * out.x + nrm[1] * out.y < 0.0 {
                    nrm = [-nrm[0], -nrm[1], -nrm[2]];
                }
                let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
                let axis = self.direction(nrm[0] / len, nrm[1] / len, nrm[2] / len);
                let el = (e1[0] * e1[0] + e1[1] * e1[1]).sqrt();
                let refd = self.direction(e1[0] / el, e1[1] / el, 0.0);
                let pl = self.placement(p0, t, axis, refd);
                (self.e(format!("PLANE('',#{pl})")), true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::Contour;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(text("l'été"), "l''\\X2\\00E9\\X0\\t\\X2\\00E9\\X0\\");
        assert_eq!(real(3.0), "3.");
        assert_eq!(real(-0.25), "-0.25");
    }

    #[test]
    fn a_plate_with_a_hole_is_one_closed_shell() {
        let outer = Loop::from_contour(&Contour::rounded(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(20.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            0.0,
        ))
        .oriented(true);
        let hole = Loop::from_contour(&Contour::Circle {
            center: Vec2::new(10.0, 5.0),
            radius: 2.0,
        })
        .oriented(false);
        let s = Solid {
            outer: Wall::vertical(outer),
            holes: vec![Wall::vertical(hole)],
        };
        let f = step("Plaque d'essai", &[s], 3.0, "2026-11-06T00:00:00");
        assert!(f.starts_with("ISO-10303-21;"));
        assert_eq!(f.matches("MANIFOLD_SOLID_BREP").count(), 1);
        // 4 côtés + 2 demi-cylindres + dessus + dessous.
        assert_eq!(f.matches("ADVANCED_FACE").count(), 8);
        assert_eq!(f.matches("CYLINDRICAL_SURFACE").count(), 2);
        // Chaque arête est utilisée deux fois, une fois dans chaque sens.
        let edges = f.matches("EDGE_CURVE").count();
        assert_eq!(f.matches("ORIENTED_EDGE").count(), 2 * edges);
        assert!(f.contains("'Plaque d''essai'"));
    }

    #[test]
    fn file_stems() {
        assert_eq!(
            file_stem("Grille ventilateur 120 mm"),
            "grille-ventilateur-120-mm"
        );
        assert_eq!(file_stem("  Façade été !"), "facade-ete");
        assert_eq!(file_stem(""), "fabvents");
    }

    #[test]
    fn example_exports_as_one_file_or_tiles() {
        let mut p = Project::example();
        let mut pl = crate::Pipeline::new();
        let f = step_file(&p, &pl.compute(&p), "2026-11-06T00:00:00").unwrap();
        assert_eq!(f.matches("MANIFOLD_SOLID_BREP").count(), 1);
        p.fabrication.tiles = [2, 2];
        let z = step_zip(&p, &pl.compute(&p), "2026-11-06T00:00:00").unwrap();
        assert_eq!(&z[..2], b"PK");
        p.fabrication.draft = 30.0;
        assert!(step_file(&p, &pl.compute(&p), "").is_err());
    }
}
