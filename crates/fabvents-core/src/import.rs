//! Import d'un contour de plaque depuis un SVG ou un DXF : le plus grand
//! contour fermé devient la plaque, les contours fermés qu'il contient
//! deviennent des découpes (trous de fixation, passages). Millimètres, Y vers
//! le haut, coin bas gauche de l'emprise ramené à l'origine.

use crate::geom::{bulge_angle, bulge_arc, point_in_polygon, signed_area, Vec2};
use std::f64::consts::TAU;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    /// Contour de la plaque (sens trigonométrique).
    pub outer: Vec<Vec2>,
    /// Contours fermés à l'intérieur (à découper).
    pub inner: Vec<Vec<Vec2>>,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    Syntax(String),
    NoClosedContour,
}

impl ImportError {
    pub fn code(&self) -> &'static str {
        match self {
            ImportError::Syntax(_) => "import.syntax",
            ImportError::NoClosedContour => "import.no_closed_contour",
        }
    }
}

/// Flèche des courbes et arcs importés (mm).
const TOL: f64 = 0.02;

/// SVG (texte commençant par `<`) ou DXF.
pub fn import_outline(text: &str) -> Result<Imported, ImportError> {
    let contours = if text.trim_start().starts_with('<') {
        svg_contours(text)?
    } else {
        dxf_contours(text)?
    };
    arrange(contours)
}

fn arrange(contours: Vec<Vec<Vec2>>) -> Result<Imported, ImportError> {
    let mut contours: Vec<Vec<Vec2>> = contours
        .into_iter()
        .filter(|c| c.len() >= 3 && signed_area(c).abs() > 1e-6)
        .collect();
    if contours.is_empty() {
        return Err(ImportError::NoClosedContour);
    }
    contours.sort_by(|a, b| signed_area(b).abs().total_cmp(&signed_area(a).abs()));
    let mut outer = contours.remove(0);
    if signed_area(&outer) < 0.0 {
        outer.reverse();
    }
    let inner: Vec<Vec<Vec2>> = contours
        .into_iter()
        .filter(|c| c.iter().all(|&p| point_in_polygon(p, &outer)))
        .collect();
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in &outer {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    let o = Vec2::new(x0, y0);
    let shift = |c: Vec<Vec2>| c.into_iter().map(|p| p - o).collect::<Vec<_>>();
    Ok(Imported {
        outer: shift(outer),
        inner: inner.into_iter().map(shift).collect(),
        width: x1 - x0,
        height: y1 - y0,
    })
}

// ---------------------------------------------------------------------------
// SVG

#[derive(Clone, Copy)]
struct M([f64; 6]);

impl M {
    fn mul(self, o: M) -> M {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = o.0;
        M([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }

    fn apply(self, x: f64, y: f64) -> Vec2 {
        let [a, b, c, d, e, f] = self.0;
        Vec2::new(a * x + c * y + e, b * x + d * y + f)
    }

    /// Facteur d'échelle moyen (pour la tolérance des courbes).
    fn scale(self) -> f64 {
        let [a, b, c, d, _, _] = self.0;
        (a * d - b * c).abs().sqrt().max(1e-9)
    }
}

/// Longueur SVG en millimètres (les unités CSS : 96 px par pouce).
fn length_mm(v: &str) -> Option<f64> {
    let l = svgtypes::Length::from_str(v).ok()?;
    use svgtypes::LengthUnit as U;
    let k = match l.unit {
        U::None | U::Px => 25.4 / 96.0,
        U::Mm => 1.0,
        U::Cm => 10.0,
        U::In => 25.4,
        U::Pt => 25.4 / 72.0,
        U::Pc => 25.4 / 6.0,
        _ => return None,
    };
    Some(l.number * k)
}

fn svg_contours(text: &str) -> Result<Vec<Vec<Vec2>>, ImportError> {
    let doc = roxmltree::Document::parse(text).map_err(|e| ImportError::Syntax(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(ImportError::Syntax("<svg> attendu".into()));
    }
    // Unités utilisateur → mm, Y retourné.
    let vb: Option<[f64; 4]> = root.attribute("viewBox").and_then(|v| {
        let n: Vec<f64> = v
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        (n.len() == 4 && n[2] > 0.0 && n[3] > 0.0).then(|| [n[0], n[1], n[2], n[3]])
    });
    let w = root.attribute("width").and_then(length_mm);
    let h = root.attribute("height").and_then(length_mm);
    let (sx, sy, tx, ty) = match (vb, w, h) {
        (Some(b), Some(w), Some(h)) => (w / b[2], h / b[3], -b[0], -b[1]),
        (Some(b), _, _) => (25.4 / 96.0, 25.4 / 96.0, -b[0], -b[1]),
        _ => (25.4 / 96.0, 25.4 / 96.0, 0.0, 0.0),
    };
    let base = M([sx, 0.0, 0.0, -sy, 0.0, 0.0]).mul(M([1.0, 0.0, 0.0, 1.0, tx, ty]));

    let mut out = Vec::new();
    for node in root.descendants().filter(|n| n.is_element()) {
        if node.ancestors().any(|a| {
            matches!(
                a.tag_name().name(),
                "defs" | "clipPath" | "mask" | "symbol" | "metadata" | "pattern" | "marker"
            )
        }) {
            continue;
        }
        // Transformation cumulée des ancêtres (du plus haut au plus proche).
        let mut m = base;
        let mut chain: Vec<roxmltree::Node> = node.ancestors().collect();
        chain.reverse();
        for a in chain {
            if let Some(t) = a.attribute("transform") {
                if let Ok(t) = svgtypes::Transform::from_str(t) {
                    m = m.mul(M([t.a, t.b, t.c, t.d, t.e, t.f]));
                }
            }
        }
        let num = |k: &str| {
            node.attribute(k)
                .and_then(|v| svgtypes::Length::from_str(v).ok())
                .map(|l| l.number)
                .unwrap_or(0.0)
        };
        let tol = TOL / m.scale();
        match node.tag_name().name() {
            "path" => {
                if let Some(d) = node.attribute("d") {
                    out.extend(
                        path_contours(d, tol)
                            .into_iter()
                            .map(|c| c.into_iter().map(|p| m.apply(p.x, p.y)).collect()),
                    );
                }
            }
            "rect" => {
                let (x, y, w, h) = (num("x"), num("y"), num("width"), num("height"));
                if w > 0.0 && h > 0.0 {
                    let rx = num("rx").max(num("ry")).min(w / 2.0).min(h / 2.0);
                    let core = [
                        Vec2::new(x + rx, y + rx),
                        Vec2::new(x + w - rx, y + rx),
                        Vec2::new(x + w - rx, y + h - rx),
                        Vec2::new(x + rx, y + h - rx),
                    ];
                    let c = crate::contour::Contour::rounded(&core, rx).tessellated(tol);
                    out.push(c.into_iter().map(|p| m.apply(p.x, p.y)).collect());
                }
            }
            "circle" | "ellipse" => {
                let (cx, cy) = (num("cx"), num("cy"));
                let (rx, ry) = if node.tag_name().name() == "circle" {
                    (num("r"), num("r"))
                } else {
                    (num("rx"), num("ry"))
                };
                if rx > 0.0 && ry > 0.0 {
                    let n = crate::geom::arc_segments(rx.max(ry), TAU, tol, 512).max(16);
                    out.push(
                        (0..n)
                            .map(|k| {
                                let t = TAU * k as f64 / n as f64;
                                m.apply(cx + rx * t.cos(), cy + ry * t.sin())
                            })
                            .collect(),
                    );
                }
            }
            "polygon" => {
                if let Some(pts) = node.attribute("points") {
                    out.push(
                        svgtypes::PointsParser::from(pts)
                            .map(|(x, y)| m.apply(x, y))
                            .collect(),
                    );
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Sous-chemins fermés d'un attribut `d` (coordonnées utilisateur).
fn path_contours(d: &str, tol: f64) -> Vec<Vec<Vec2>> {
    use svgtypes::SimplePathSegment as S;
    let mut out = Vec::new();
    let mut cur: Vec<Vec2> = Vec::new();
    let mut last = Vec2::ZERO;
    let flush = |cur: &mut Vec<Vec2>, closed: bool, out: &mut Vec<Vec<Vec2>>| {
        if cur.len() >= 3 {
            let closes = closed || cur[0].distance(*cur.last().unwrap()) < 1e-6;
            if closes {
                if cur[0].distance(*cur.last().unwrap()) < 1e-9 {
                    cur.pop();
                }
                out.push(std::mem::take(cur));
            }
        }
        cur.clear();
    };
    let steps = |len: f64| ((len / (8.0 * tol).sqrt().max(1e-6)).ceil() as usize).clamp(4, 256);
    for seg in svgtypes::SimplifyingPathParser::from(d).flatten() {
        match seg {
            S::MoveTo { x, y } => {
                flush(&mut cur, false, &mut out);
                last = Vec2::new(x, y);
                cur.push(last);
            }
            S::LineTo { x, y } => {
                last = Vec2::new(x, y);
                cur.push(last);
            }
            S::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let (p0, p1, p2, p3) =
                    (last, Vec2::new(x1, y1), Vec2::new(x2, y2), Vec2::new(x, y));
                let n = steps(p0.distance(p1) + p1.distance(p2) + p2.distance(p3));
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let u = 1.0 - t;
                    cur.push(
                        p0 * (u * u * u)
                            + p1 * (3.0 * u * u * t)
                            + p2 * (3.0 * u * t * t)
                            + p3 * (t * t * t),
                    );
                }
                last = p3;
            }
            S::Quadratic { x1, y1, x, y } => {
                let (p0, p1, p2) = (last, Vec2::new(x1, y1), Vec2::new(x, y));
                let n = steps(p0.distance(p1) + p1.distance(p2));
                for k in 1..=n {
                    let t = k as f64 / n as f64;
                    let u = 1.0 - t;
                    cur.push(p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t));
                }
                last = p2;
            }
            S::ClosePath => {
                let start = cur.first().copied();
                flush(&mut cur, true, &mut out);
                if let Some(s) = start {
                    last = s;
                    cur.push(s);
                }
            }
        }
    }
    flush(&mut cur, false, &mut out);
    out
}

// ---------------------------------------------------------------------------
// DXF

fn dxf_contours(text: &str) -> Result<Vec<Vec<Vec2>>, ImportError> {
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();
    if lines.len() < 2 {
        return Err(ImportError::Syntax("DXF vide".into()));
    }
    let pairs: Vec<(i32, &str)> = lines
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].parse::<i32>().unwrap_or(-9999), c[1]))
        .collect();
    if pairs.iter().all(|(c, _)| *c == -9999) {
        return Err(ImportError::Syntax("codes de groupe illisibles".into()));
    }
    // Unités ($INSUNITS) : 1 po, 2 pi, 4 mm, 5 cm, 6 m ; mm par défaut.
    let mut unit = 1.0;
    for w in pairs.windows(2) {
        if w[0] == (9, "$INSUNITS") && w[1].0 == 70 {
            unit = match w[1].1.parse::<i32>().unwrap_or(4) {
                1 => 25.4,
                2 => 304.8,
                5 => 10.0,
                6 => 1000.0,
                _ => 1.0,
            };
        }
    }
    let f = |v: &str| v.parse::<f64>().unwrap_or(0.0);

    // Entités : une suite de groupes jusqu'au prochain code 0.
    let start = pairs
        .iter()
        .position(|&(c, v)| c == 2 && v == "ENTITIES")
        .unwrap_or(0);
    let mut ents: Vec<(&str, Vec<(i32, &str)>)> = Vec::new();
    for &(c, v) in &pairs[start..] {
        if c == 0 {
            ents.push((v, Vec::new()));
        } else if let Some(e) = ents.last_mut() {
            e.1.push((c, v));
        }
    }

    let mut closed: Vec<Vec<Vec2>> = Vec::new();
    let mut open: Vec<Vec<Vec2>> = Vec::new();
    let mut poly: Option<(bool, Vec<(Vec2, f64)>)> = None;
    let bulged = |v: &[(Vec2, f64)], closed: bool| -> Vec<Vec2> {
        let mut pts = Vec::new();
        let n = v.len();
        for i in 0..n {
            let (p, b) = v[i];
            pts.push(p);
            if (i + 1 < n || closed) && b != 0.0 {
                let q = v[(i + 1) % n].0;
                let (c, r) = bulge_arc(p, q, b);
                let theta = bulge_angle(b);
                let segs = crate::geom::arc_segments(r, theta, TOL, 256);
                let a0 = (p - c).angle();
                for k in 1..segs {
                    pts.push(c + Vec2::from_angle(a0 + theta * k as f64 / segs as f64) * r);
                }
            }
        }
        pts
    };
    for (name, g) in &ents {
        let get = |code: i32| {
            g.iter()
                .find(|(c, _)| *c == code)
                .map(|(_, v)| f(v))
                .unwrap_or(0.0)
        };
        match *name {
            "LWPOLYLINE" => {
                let is_closed = (get(70) as i32) & 1 == 1;
                let mut v: Vec<(Vec2, f64)> = Vec::new();
                let mut x = None;
                for &(c, val) in g {
                    match c {
                        10 => x = Some(f(val)),
                        20 => {
                            if let Some(xx) = x.take() {
                                v.push((Vec2::new(xx, f(val)) * unit, 0.0));
                            }
                        }
                        42 => {
                            if let Some(last) = v.last_mut() {
                                last.1 = f(val);
                            }
                        }
                        _ => {}
                    }
                }
                let pts = bulged(&v, is_closed);
                if is_closed {
                    closed.push(pts)
                } else {
                    open.push(pts)
                }
            }
            "POLYLINE" => poly = Some(((get(70) as i32) & 1 == 1, Vec::new())),
            "VERTEX" => {
                if let Some((_, v)) = poly.as_mut() {
                    v.push((Vec2::new(get(10), get(20)) * unit, get(42)));
                }
            }
            "SEQEND" => {
                if let Some((is_closed, v)) = poly.take() {
                    let pts = bulged(&v, is_closed);
                    if is_closed {
                        closed.push(pts)
                    } else {
                        open.push(pts)
                    }
                }
            }
            "CIRCLE" => {
                let (c, r) = (Vec2::new(get(10), get(20)) * unit, get(40) * unit);
                if r > 0.0 {
                    let n = crate::geom::arc_segments(r, TAU, TOL, 512).max(16);
                    closed.push(
                        (0..n)
                            .map(|k| c + Vec2::from_angle(TAU * k as f64 / n as f64) * r)
                            .collect(),
                    );
                }
            }
            "ELLIPSE" => {
                let c = Vec2::new(get(10), get(20)) * unit;
                let major = Vec2::new(get(11), get(21)) * unit;
                let ratio = get(40);
                let (t0, mut t1) = (get(41), get(42));
                if t1 <= t0 {
                    t1 += TAU;
                }
                let a = major.length();
                let n = crate::geom::arc_segments(a, t1 - t0, TOL, 512).max(16);
                let rot = major.angle();
                let pts: Vec<Vec2> = (0..=n)
                    .map(|k| {
                        let t = t0 + (t1 - t0) * k as f64 / n as f64;
                        c + Vec2::new(a * t.cos(), a * ratio * t.sin()).rotated(rot)
                    })
                    .collect();
                if (t1 - t0 - TAU).abs() < 1e-6 {
                    closed.push(pts[..n].to_vec())
                } else {
                    open.push(pts)
                }
            }
            "LINE" => open.push(vec![
                Vec2::new(get(10), get(20)) * unit,
                Vec2::new(get(11), get(21)) * unit,
            ]),
            "ARC" => {
                let (c, r) = (Vec2::new(get(10), get(20)) * unit, get(40) * unit);
                let (a0, mut a1) = (get(50).to_radians(), get(51).to_radians());
                if a1 <= a0 {
                    a1 += TAU;
                }
                let n = crate::geom::arc_segments(r, a1 - a0, TOL, 256);
                open.push(
                    (0..=n)
                        .map(|k| c + Vec2::from_angle(a0 + (a1 - a0) * k as f64 / n as f64) * r)
                        .collect(),
                );
            }
            _ => {}
        }
    }
    closed.extend(chain(open));
    Ok(closed)
}

/// Assemble des morceaux ouverts (lignes, arcs) en contours fermés, bout à bout.
fn chain(mut pieces: Vec<Vec<Vec2>>) -> Vec<Vec<Vec2>> {
    const EPS: f64 = 1e-3;
    let mut out = Vec::new();
    while let Some(mut c) = pieces.pop() {
        loop {
            if c.len() > 2 && c[0].distance(*c.last().unwrap()) < EPS {
                c.pop();
                out.push(c);
                break;
            }
            let end = *c.last().unwrap();
            let Some(k) = pieces
                .iter()
                .position(|p| p[0].distance(end) < EPS || p.last().unwrap().distance(end) < EPS)
            else {
                break;
            };
            let mut p = pieces.swap_remove(k);
            if p[0].distance(end) >= EPS {
                p.reverse();
            }
            c.extend(p.into_iter().skip(1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_in_mm_with_a_hole() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">
            <rect x="0" y="0" width="100" height="50" rx="5"/>
            <circle cx="10" cy="10" r="2"/>
            <path d="M 200 200 L 210 200"/>
        </svg>"#;
        let r = import_outline(svg).unwrap();
        assert!((r.width - 100.0).abs() < 1e-6 && (r.height - 50.0).abs() < 1e-6);
        assert_eq!(r.inner.len(), 1);
        // Y retourné : le cercle à 10 mm du haut est à 40 mm du bas.
        let c = r.inner[0].iter().fold(Vec2::ZERO, |a, &p| a + p) * (1.0 / r.inner[0].len() as f64);
        assert!(
            (c.x - 10.0).abs() < 0.01 && (c.y - 40.0).abs() < 0.01,
            "{c:?}"
        );
        assert!(signed_area(&r.outer) > 0.0);
    }

    #[test]
    fn svg_path_with_curves_and_transform() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20">
            <g transform="translate(10 10)"><path d="M -5 0 A 5 5 0 1 1 5 0 A 5 5 0 1 1 -5 0 Z"/></g>
        </svg>"#;
        let r = import_outline(svg).unwrap();
        assert!((r.width - 10.0).abs() < 0.01, "{}", r.width);
        assert!((signed_area(&r.outer) - std::f64::consts::PI * 25.0).abs() < 0.1);
    }

    #[test]
    fn dxf_lines_are_chained_and_units_applied() {
        let mut s = String::from(
            "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n5\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n",
        );
        for (a, b) in [
            ((0.0, 0.0), (8.0, 0.0)),
            ((8.0, 0.0), (8.0, 4.0)),
            ((0.0, 4.0), (8.0, 4.0)),
            ((0.0, 4.0), (0.0, 0.0)),
        ] {
            s += &format!(
                "0\nLINE\n8\n0\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n",
                a.0, a.1, b.0, b.1
            );
        }
        s += "0\nCIRCLE\n8\n0\n10\n2\n20\n2\n40\n0.5\n0\nENDSEC\n0\nEOF\n";
        let r = import_outline(&s).unwrap();
        // Centimètres : plaque 80 × 40 mm, un trou de Ø 10 mm.
        assert!((r.width - 80.0).abs() < 1e-6 && (r.height - 40.0).abs() < 1e-6);
        assert_eq!(r.inner.len(), 1);
    }

    #[test]
    fn our_own_dxf_roundtrips() {
        let p = crate::project::Project::example();
        let mut pl = crate::pipeline::Pipeline::new();
        let f = pl.compute(&p);
        let dxf = crate::export::dxf::dxf(&crate::export::Drawing::new(&p, &f));
        let r = import_outline(&dxf).unwrap();
        assert!((r.width - 200.0).abs() < 1e-3 && (r.height - 150.0).abs() < 1e-3);
        assert_eq!(r.inner.len(), f.holes.len());
    }

    #[test]
    fn nothing_closed_is_an_error() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0 L10 0"/></svg>"#;
        assert_eq!(import_outline(svg), Err(ImportError::NoClosedContour));
    }
}
