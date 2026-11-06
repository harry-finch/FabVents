//! Formes des perforations : âme (point, segment ou polygone) + arrondi,
//! pour une taille et un angle donnés (voir `contour`).

use crate::boolean::{combine, Shapes};
use crate::contour::Contour;
use crate::curve::flatten;
use crate::geom::{signed_area, Vec2, DEG};
use crate::project::{
    Curve, CustomShape, LayerRole, Shape, ShapeKind, ShapeLayer, ShapeNode, Symmetry,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};

/// Âme et arrondi d'un trou, en coordonnées de la plaque.
#[derive(Clone, Debug, PartialEq)]
pub struct HoleGeometry {
    pub core: Vec<Vec2>,
    pub round: f64,
    /// Rayon du cercle englobant, depuis le centre.
    pub bound: f64,
    /// Aire exacte.
    pub area: f64,
}

/// Forme `shape` à la taille de référence `size`, tournée de `angle` degrés,
/// centrée en `center` (sans morphing). Pour beaucoup de trous, préparer la
/// forme une fois avec [`PreparedShape`].
pub fn hole_geometry(shape: &Shape, size: f64, angle: f64, center: Vec2) -> HoleGeometry {
    PreparedShape::new(shape).geometry(size, angle, 0.0, center)
}

/// Échantillons d'un contour pour le morphing.
const MORPH_SAMPLES: usize = 128;

/// Forme préparée une fois par calcul : contour de la forme dessinée (à la
/// taille de base, ou par taille si ses calques répondent différemment au
/// canal Taille), contours échantillonnés du morphing.
pub struct PreparedShape {
    shape: Shape,
    custom: Option<CustomPrep>,
    /// Départ et cible du morphing, en unités de taille, appariés.
    morph: Option<(Vec<Vec2>, Vec<Vec2>)>,
}

struct CustomPrep {
    /// Contour quand tous les calques suivent la taille (unités de taille).
    uniform: Option<Vec<Vec2>>,
    /// Contours par taille (au centième de millimètre), sinon.
    cache: RefCell<HashMap<i64, Vec<Vec2>>>,
    /// Morceaux écartés et îlots de matière à la taille de base.
    dropped: usize,
}

impl PreparedShape {
    pub fn new(shape: &Shape) -> PreparedShape {
        let base = shape.size.max(1e-6);
        let custom = match &shape.kind {
            ShapeKind::Custom(c) => {
                let uniform = c.layers.iter().all(|l| l.response == 1.0);
                let (poly, dropped) = custom_polygon(c, base, base);
                Some(CustomPrep {
                    uniform: uniform.then_some(poly),
                    cache: RefCell::new(HashMap::new()),
                    dropped,
                })
            }
            _ => None,
        };
        let morph = shape.morph.as_deref().map(|target| {
            let a = resample(&unit_outline(&shape.kind, shape.size), MORPH_SAMPLES);
            let b = resample(&unit_outline(target, shape.size), MORPH_SAMPLES);
            (a.clone(), align(&a, &b))
        });
        PreparedShape {
            shape: shape.clone(),
            custom,
            morph,
        }
    }

    /// Morceaux écartés (contour en plusieurs pièces, îlots) d'une forme dessinée.
    pub fn dropped_parts(&self) -> usize {
        self.custom.as_ref().map_or(0, |c| c.dropped)
    }

    /// Perforation à la taille `size`, tournée de `angle` degrés, déformée à
    /// `morph` (0 : la forme, 1 : la cible), posée en `center`.
    pub fn geometry(&self, size: f64, angle: f64, morph: f64, center: Vec2) -> HoleGeometry {
        let rot = angle * DEG;
        if let Some((a, b)) = &self.morph {
            if morph >= 1.0 - 1e-9 {
                let target = Shape {
                    kind: self.shape.morph.as_deref().cloned().unwrap_or_default(),
                    morph: None,
                    ..self.shape.clone()
                };
                return PreparedShape::new(&target).geometry(size, angle, 0.0, center);
            }
            if morph > 1e-9 {
                let poly: Vec<Vec2> = a.iter().zip(b).map(|(p, q)| p.lerp(*q, morph)).collect();
                return polygon_geometry(&poly, size, rot, center);
            }
        }
        match (&self.shape.kind, &self.custom) {
            (ShapeKind::Custom(c), Some(prep)) => {
                if let Some(poly) = &prep.uniform {
                    return polygon_geometry(poly, size, rot, center);
                }
                let key = (size * 100.0).round() as i64;
                let mut cache = prep.cache.borrow_mut();
                let poly = cache
                    .entry(key)
                    .or_insert_with(|| custom_polygon(c, self.shape.size, key as f64 / 100.0).0);
                polygon_geometry(poly, size, rot, center)
            }
            (kind, _) => primitive(kind, self.shape.size, size, angle, center),
        }
    }
}

/// Polygone en unités de taille → perforation (âme sans arrondi).
fn polygon_geometry(poly: &[Vec2], size: f64, rot: f64, center: Vec2) -> HoleGeometry {
    if poly.len() < 3 || size <= 0.0 {
        return HoleGeometry {
            core: vec![center],
            round: 0.0,
            bound: 0.0,
            area: 0.0,
        };
    }
    let core: Vec<Vec2> = poly
        .iter()
        .map(|&v| center + (v * size).rotated(rot))
        .collect();
    let bound = poly.iter().map(|v| v.length()).fold(0.0, f64::max) * size;
    HoleGeometry {
        area: signed_area(poly).abs() * size * size,
        core,
        round: 0.0,
        bound,
    }
}

/// Contour d'une forme, en unités de taille, autour de son centre ou de
/// son pivot (sens trigonométrique, finement facettisé).
fn unit_outline(kind: &ShapeKind, base: f64) -> Vec<Vec2> {
    let base = base.max(1e-6);
    match kind {
        ShapeKind::Custom(c) => custom_polygon(c, base, base).0,
        k => {
            let g = primitive(k, base, base, 0.0, Vec2::ZERO);
            let mut pts = Contour::rounded(&g.core, g.round).tessellated(0.002 * base);
            if g.core.len() == 2 && g.round <= 0.0 {
                pts = g.core.clone();
            }
            pts.into_iter().map(|p| p * (1.0 / base)).collect()
        }
    }
}

/// `n` points régulièrement espacés le long d'un contour fermé.
fn resample(poly: &[Vec2], n: usize) -> Vec<Vec2> {
    let m = poly.len();
    if m < 2 {
        return vec![poly.first().copied().unwrap_or(Vec2::ZERO); n];
    }
    let mut poly = poly.to_vec();
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    let lens: Vec<f64> = (0..m)
        .map(|i| poly[i].distance(poly[(i + 1) % m]))
        .collect();
    let total: f64 = lens.iter().sum();
    let mut out = Vec::with_capacity(n);
    let (mut seg, mut acc) = (0usize, 0.0);
    for k in 0..n {
        let target = total * k as f64 / n as f64;
        while seg < m - 1 && acc + lens[seg] < target {
            acc += lens[seg];
            seg += 1;
        }
        let t = if lens[seg] > 0.0 {
            ((target - acc) / lens[seg]).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push(poly[seg].lerp(poly[(seg + 1) % m], t));
    }
    out
}

/// `b` décalé pour suivre `a` au plus près (pas de vrillage au morphing).
fn align(a: &[Vec2], b: &[Vec2]) -> Vec<Vec2> {
    let n = a.len();
    let cost = |k: usize| {
        (0..n)
            .map(|i| a[i].distance(b[(i + k) % n]).powi(2))
            .sum::<f64>()
    };
    let best = (0..n)
        .min_by(|&x, &y| cost(x).total_cmp(&cost(y)))
        .unwrap_or(0);
    (0..n).map(|i| b[(i + best) % n]).collect()
}

/// Contour d'une forme dessinée pour une perforation de taille `size`
/// (taille de base `base`), en unités de taille autour du pivot : calques
/// facettisés, copiés par symétrie, combinés dans l'ordre. Renvoie le plus
/// grand morceau et le nombre de morceaux et d'îlots écartés.
fn custom_polygon(c: &CustomShape, base: f64, size: f64) -> (Vec<Vec2>, usize) {
    let size = size.max(1e-6);
    // Flèche 0,004 mm à la taille de la perforation.
    let tol = 0.004 / size;
    let mut acc: Shapes = Vec::new();
    for layer in &c.layers {
        if layer.nodes.len() < 3 {
            continue;
        }
        // Échelle du calque relative à la perforation (réponse au canal Taille).
        let k = (base + layer.response * (size - base)) / size;
        let pts: Vec<Vec2> = flatten_layer(layer, tol / k.max(1e-6))
            .into_iter()
            .map(|p| (p - c.pivot) * k)
            .collect();
        let copies = symmetric(&pts, layer.symmetry);
        acc = combine(&acc, &copies, layer.role == LayerRole::Union);
    }
    let mut best: Option<(f64, Vec<Vec2>)> = None;
    let mut dropped = 0usize;
    for s in &acc {
        dropped += s.len().saturating_sub(1);
        let area = signed_area(&s[0]).abs();
        if best.as_ref().is_none_or(|b| area > b.0) {
            if best.is_some() {
                dropped += 1;
            }
            best = Some((area, s[0].clone()));
        } else {
            dropped += 1;
        }
    }
    let mut poly = best.map(|b| b.1).unwrap_or_default();
    if poly.len() >= 3 {
        let mut closed = poly.clone();
        closed.push(poly[0]);
        poly = crate::fit::simplify(&closed, tol * 0.5);
        if signed_area(&poly) < 0.0 {
            poly.reverse();
        }
    }
    (poly, dropped)
}

/// Contour d'un calque : segments droits ou courbes de Bézier cubiques,
/// congés aux coins entre deux segments droits.
pub fn flatten_layer(layer: &ShapeLayer, tol: f64) -> Vec<Vec2> {
    let n = layer.nodes.len();
    let curved = |i: usize| {
        let (a, b) = (&layer.nodes[i], &layer.nodes[(i + 1) % n]);
        a.handle_out.is_some() || b.handle_in.is_some()
    };
    let mut points = Vec::with_capacity(4 * n);
    let mut fillets = Vec::with_capacity(4 * n);
    for i in 0..n {
        let a = &layer.nodes[i];
        let straight = !curved(i) && !curved((i + n - 1) % n);
        points.push(a.at);
        fillets.push(if straight { a.fillet } else { 0.0 });
        if curved(i) {
            let b = &layer.nodes[(i + 1) % n];
            let c1 = a.at + a.handle_out.unwrap_or(Vec2::ZERO);
            let c2 = b.at + b.handle_in.unwrap_or(Vec2::ZERO);
            let span = a.at.distance(c1) + c1.distance(c2) + c2.distance(b.at);
            let k = ((span / tol.max(1e-6)).sqrt().ceil() as usize).clamp(4, 64);
            for j in 1..k {
                let t = j as f64 / k as f64;
                let u = 1.0 - t;
                points.push(
                    a.at * (u * u * u)
                        + c1 * (3.0 * u * u * t)
                        + c2 * (3.0 * u * t * t)
                        + b.at * (t * t * t),
                );
                fillets.push(0.0);
            }
        }
    }
    flatten(
        &Curve {
            points,
            closed: true,
            fillets,
        },
        tol,
    )
}

/// Copies d'un contour par symétrie autour de l'origine (le pivot).
fn symmetric(pts: &[Vec2], sym: Symmetry) -> Vec<Vec<Vec2>> {
    let map = |f: &dyn Fn(Vec2) -> Vec2| -> Vec<Vec2> {
        let mut v: Vec<Vec2> = pts.iter().map(|&p| f(p)).collect();
        if signed_area(&v) < 0.0 {
            v.reverse();
        }
        v
    };
    let mut out = vec![map(&|p| p)];
    match sym {
        Symmetry::None => {}
        Symmetry::Mirror {
            vertical,
            horizontal,
        } => {
            if vertical {
                out.push(map(&|p| Vec2::new(-p.x, p.y)));
            }
            if horizontal {
                out.push(map(&|p| Vec2::new(p.x, -p.y)));
            }
            if vertical && horizontal {
                out.push(map(&|p| Vec2::new(-p.x, -p.y)));
            }
        }
        Symmetry::Radial { count } => {
            for k in 1..count.max(1) {
                let a = TAU * k as f64 / count as f64;
                out.push(map(&|p| p.rotated(a)));
            }
        }
    }
    out
}

/// Forme dessinée depuis un contour importé (SVG, DXF) : le contour
/// extérieur en union, les contours intérieurs en soustraction, centrée sur
/// sa boîte et ramenée à une taille 1 (le plus grand côté).
pub fn shape_from_import(imp: &crate::import::Imported) -> CustomShape {
    let (mut lo, mut hi) = (
        Vec2::new(f64::INFINITY, f64::INFINITY),
        Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in &imp.outer {
        lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let c = lo.lerp(hi, 0.5);
    let k = 1.0 / (hi.x - lo.x).max(hi.y - lo.y).max(1e-9);
    let layer = |pts: &[Vec2], role: LayerRole| {
        let mut closed: Vec<Vec2> = pts.iter().map(|&p| (p - c) * k).collect();
        closed.push(closed[0]);
        ShapeLayer {
            nodes: crate::fit::simplify(&closed, 0.001)
                .into_iter()
                .map(ShapeNode::at)
                .collect(),
            role,
            symmetry: Symmetry::None,
            response: 1.0,
        }
    };
    let mut layers = vec![layer(&imp.outer, LayerRole::Union)];
    layers.extend(
        imp.inner
            .iter()
            .filter(|i| i.len() >= 3)
            .map(|i| layer(i, LayerRole::Subtract)),
    );
    CustomShape {
        layers,
        pivot: Vec2::ZERO,
    }
}

/// Formes simples : âme + arrondi exacts.
fn primitive(
    kind: &ShapeKind,
    shape_size: f64,
    size: f64,
    angle: f64,
    center: Vec2,
) -> HoleGeometry {
    let rot = angle * DEG;
    let place = |v: Vec2| center + v.rotated(rot);
    match *kind {
        ShapeKind::Circle => {
            let r = size / 2.0;
            HoleGeometry {
                core: vec![center],
                round: r,
                bound: r,
                area: PI * r * r,
            }
        }
        ShapeKind::Rectangle { aspect, corner } => {
            let (w, h) = (size, size * aspect);
            let k = if shape_size > 0.0 {
                size / shape_size
            } else {
                1.0
            };
            let rc = (corner * k).clamp(0.0, w.min(h) / 2.0);
            let (hx, hy) = (w / 2.0 - rc, h / 2.0 - rc);
            let core = degenerate(&[
                Vec2::new(-hx, -hy),
                Vec2::new(hx, -hy),
                Vec2::new(hx, hy),
                Vec2::new(-hx, hy),
            ]);
            HoleGeometry {
                core: core.into_iter().map(place).collect(),
                round: rc,
                bound: Vec2::new(hx, hy).length() + rc,
                area: w * h - (4.0 - PI) * rc * rc,
            }
        }
        ShapeKind::Polygon { sides } => {
            let n = sides.max(3) as usize;
            let r = size / 2.0;
            let start = -PI / 2.0 + PI / n as f64;
            let core: Vec<Vec2> = (0..n)
                .map(|i| place(Vec2::from_angle(start + TAU * i as f64 / n as f64) * r))
                .collect();
            HoleGeometry {
                core,
                round: 0.0,
                bound: r,
                area: 0.5 * n as f64 * r * r * (TAU / n as f64).sin(),
            }
        }
        ShapeKind::Star { points, inner } => {
            let n = points.max(3) as usize;
            let r = size / 2.0;
            let ri = r * inner;
            let core: Vec<Vec2> = (0..2 * n)
                .map(|i| {
                    let a = PI / 2.0 + PI * i as f64 / n as f64;
                    place(Vec2::from_angle(a) * if i % 2 == 0 { r } else { ri })
                })
                .collect();
            // 2n triangles (centre, sommet extérieur, sommet intérieur).
            let area = n as f64 * r * ri * (PI / n as f64).sin();
            HoleGeometry {
                core,
                round: 0.0,
                bound: r,
                area,
            }
        }
        ShapeKind::Slot { aspect } => {
            let width = size * aspect;
            let half = (size - width).max(0.0) / 2.0;
            let core = degenerate(&[Vec2::new(-half, 0.0), Vec2::new(half, 0.0)]);
            let rr = width / 2.0;
            HoleGeometry {
                core: core.into_iter().map(place).collect(),
                round: rr,
                bound: half + rr,
                area: 2.0 * half * width + PI * rr * rr,
            }
        }
        // Une forme dessinée passe par `PreparedShape`.
        ShapeKind::Custom(_) => PreparedShape::new(&Shape {
            kind: kind.clone(),
            size: shape_size,
            rotation: 0.0,
            morph: None,
        })
        .geometry(size, angle, 0.0, center),
    }
}

/// Retire les sommets confondus (rectangle réduit à un segment ou à un point).
fn degenerate(pts: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(pts.len());
    for &p in pts {
        if !out.iter().any(|q| q.distance(p) < 1e-9) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::Contour;

    fn shape(kind: ShapeKind) -> Shape {
        Shape {
            kind,
            size: 10.0,
            rotation: 0.0,
            morph: None,
        }
    }

    #[test]
    fn areas_match_contours() {
        let kinds = [
            ShapeKind::Circle,
            ShapeKind::Rectangle {
                aspect: 0.6,
                corner: 1.5,
            },
            ShapeKind::Polygon { sides: 6 },
            ShapeKind::Star {
                points: 5,
                inner: 0.45,
            },
            ShapeKind::Slot { aspect: 0.3 },
        ];
        for k in kinds {
            let s = shape(k.clone());
            let h = hole_geometry(&s, 10.0, 17.0, Vec2::new(3.0, 4.0));
            let c = Contour::rounded(&h.core, h.round);
            assert!(
                (c.area() - h.area).abs() < 1e-9,
                "{k:?} : {} ≠ {}",
                c.area(),
                h.area
            );
        }
    }

    #[test]
    fn rectangle_with_full_corners_becomes_a_slot() {
        let s = shape(ShapeKind::Rectangle {
            aspect: 0.5,
            corner: 2.5,
        });
        let h = hole_geometry(&s, 10.0, 0.0, Vec2::ZERO);
        assert_eq!(h.core.len(), 2);
        assert_eq!(h.round, 2.5);
    }

    #[test]
    fn hexagon_has_flat_base() {
        let h = hole_geometry(
            &shape(ShapeKind::Polygon { sides: 6 }),
            10.0,
            0.0,
            Vec2::ZERO,
        );
        let min_y = h.core.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        assert_eq!(
            h.core.iter().filter(|p| (p.y - min_y).abs() < 1e-9).count(),
            2
        );
    }

    fn layer(pts: &[(f64, f64)]) -> ShapeLayer {
        ShapeLayer {
            nodes: pts
                .iter()
                .map(|&(x, y)| ShapeNode::at(Vec2::new(x, y)))
                .collect(),
            ..ShapeLayer::default()
        }
    }

    fn custom(layers: Vec<ShapeLayer>) -> Shape {
        shape(ShapeKind::Custom(CustomShape {
            layers,
            pivot: Vec2::ZERO,
        }))
    }

    #[test]
    fn custom_square_and_filleted_circle() {
        let s = custom(vec![ShapeLayer::default()]);
        let h = hole_geometry(&s, 10.0, 30.0, Vec2::new(5.0, 5.0));
        assert!((h.area - 100.0).abs() < 1e-6, "{}", h.area);
        // Carré dont les congés valent la moitié du côté : un disque.
        let mut round = ShapeLayer::default();
        for n in &mut round.nodes {
            n.fillet = 0.5;
        }
        let h = hole_geometry(&custom(vec![round]), 10.0, 0.0, Vec2::ZERO);
        // Facettes à 0,004 mm près.
        assert!((h.area - PI * 25.0).abs() < 0.1, "{}", h.area);
    }

    #[test]
    fn subtract_mirror_and_radial_layers() {
        // Carré moins son quart haut droit : aire 0,75.
        let cut = ShapeLayer {
            role: LayerRole::Subtract,
            ..layer(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
        };
        let h = hole_geometry(
            &custom(vec![ShapeLayer::default(), cut]),
            10.0,
            0.0,
            Vec2::ZERO,
        );
        assert!((h.area - 75.0).abs() < 1e-6, "{}", h.area);
        // Demi-carré droit en miroir vertical : le carré entier.
        let half = ShapeLayer {
            symmetry: Symmetry::Mirror {
                vertical: true,
                horizontal: false,
            },
            ..layer(&[(0.0, -0.5), (0.5, -0.5), (0.5, 0.5), (0.0, 0.5)])
        };
        let h = hole_geometry(&custom(vec![half]), 10.0, 0.0, Vec2::ZERO);
        assert!((h.area - 100.0).abs() < 1e-6, "{}", h.area);
        // Barre 1 × 0,2 répétée 2 fois (90°) : croix d'aire 0,36.
        let bar = ShapeLayer {
            symmetry: Symmetry::Radial { count: 2 },
            ..layer(&[(-0.5, -0.1), (0.5, -0.1), (0.5, 0.1), (-0.5, 0.1)])
        };
        let h = hole_geometry(&custom(vec![bar]), 10.0, 0.0, Vec2::ZERO);
        assert!((h.area - 20.0).abs() < 1e-6, "{}", h.area);
        let bar4 = ShapeLayer {
            symmetry: Symmetry::Radial { count: 4 },
            ..layer(&[(-0.5, -0.1), (0.5, -0.1), (0.5, 0.1), (-0.5, 0.1)])
        };
        let h = hole_geometry(&custom(vec![bar4]), 10.0, 0.0, Vec2::ZERO);
        assert!((h.area - 36.0).abs() < 1e-6, "{}", h.area);
    }

    #[test]
    fn layer_response_and_bezier_nodes() {
        // Calque fixe (réponse 0) : sa taille ne suit pas le canal Taille.
        let fixed = ShapeLayer {
            response: 0.0,
            ..ShapeLayer::default()
        };
        let s = custom(vec![fixed]);
        let p = PreparedShape::new(&s);
        assert!((p.geometry(20.0, 0.0, 0.0, Vec2::ZERO).area - 100.0).abs() < 1e-6);
        // Cercle de Bézier (4 nœuds, poignées 0,5523 r) : aire π r² à 0,1 % près.
        let k = 0.5523 * 0.5;
        let n = |x: f64, y: f64, hx: f64, hy: f64| ShapeNode {
            handle_in: Some(Vec2::new(-hx, -hy)),
            handle_out: Some(Vec2::new(hx, hy)),
            ..ShapeNode::at(Vec2::new(x, y))
        };
        let circle = ShapeLayer {
            nodes: vec![
                n(0.5, 0.0, 0.0, k),
                n(0.0, 0.5, -k, 0.0),
                n(-0.5, 0.0, 0.0, -k),
                n(0.0, -0.5, k, 0.0),
            ],
            ..ShapeLayer::default()
        };
        let h = hole_geometry(&custom(vec![circle]), 10.0, 0.0, Vec2::ZERO);
        assert!((h.area / (PI * 25.0) - 1.0).abs() < 1e-3, "{}", h.area);
    }

    #[test]
    fn morph_goes_from_shape_to_target() {
        let s = Shape {
            morph: Some(Box::new(ShapeKind::Rectangle {
                aspect: 1.0,
                corner: 0.0,
            })),
            ..shape(ShapeKind::Circle)
        };
        let p = PreparedShape::new(&s);
        let a = p.geometry(10.0, 0.0, 0.0, Vec2::ZERO);
        assert_eq!(a.core.len(), 1);
        let b = p.geometry(10.0, 0.0, 1.0, Vec2::ZERO);
        assert!((b.area - 100.0).abs() < 1e-9);
        let m = p.geometry(10.0, 0.0, 0.5, Vec2::ZERO);
        assert!(
            m.area > a.area && m.area < b.area,
            "{} {} {}",
            a.area,
            m.area,
            b.area
        );
        assert_eq!(m.core.len(), MORPH_SAMPLES);
    }

    #[test]
    fn imported_outline_becomes_layers() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="10mm" viewBox="0 0 20 10"><path d="M0 0 H20 V10 H0 Z M5 3 H15 V7 H5 Z"/></svg>"#;
        let imp = crate::import::import_outline(svg).unwrap();
        let c = shape_from_import(&imp);
        assert_eq!(c.layers.len(), 2);
        assert_eq!(c.layers[1].role, LayerRole::Subtract);
        // 20 × 10 ramené à 1 × 0,5. Le contour intérieur laisserait un îlot
        // de matière dans la perforation : écarté, et compté.
        let s = custom(c.layers);
        let h = hole_geometry(&s, 10.0, 0.0, Vec2::ZERO);
        assert!((h.area - 50.0).abs() < 1e-6, "{}", h.area);
        assert_eq!(PreparedShape::new(&s).dropped_parts(), 1);
    }

    #[test]
    fn corner_scales_with_size() {
        let s = shape(ShapeKind::Rectangle {
            aspect: 1.0,
            corner: 2.0,
        });
        assert_eq!(hole_geometry(&s, 20.0, 0.0, Vec2::ZERO).round, 4.0);
    }
}
