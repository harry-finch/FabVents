//! Champs d'influence : valeur d'un canal en un point (ADR-0004).
//!
//! Chaque contrôleur donne un poids `w ∈ [0, 1]` : 1 sur le contrôleur, 0 à
//! sa portée, transition *smoothstep* (ou seuil franc) ; pour une image, la
//! luminance (sombre = 1, avec inversion et contraste). Les contrôleurs d'un
//! canal se mélangent ainsi :
//!
//! ```text
//! cible  = Σ wᵢ·tᵢ / Σ wᵢ        (moyenne des cibles, pondérée)
//! valeur = base + max(wᵢ) · (cible − base)
//! ```
//!
//! Loin de tout contrôleur la valeur reste la valeur de base ; sur un
//! contrôleur elle atteint sa cible ; deux contrôleurs qui se recouvrent
//! donnent une valeur intermédiaire, jamais au-delà de leurs cibles.
//!
//! Options : côté unique et prolongement (ligne, courbe ouverte), intérieur
//! rempli (tracé fermé), cible symétrique de l'autre côté du contrôleur,
//! valeur arrondie à un pas, disparition aléatoire (canal Taille).

use crate::curve::{flatten, Polyline};
use crate::geom::{Vec2, DEG};
use crate::project::{Channel, Controller, CullMode, Geometry, Influence, Side};
use crate::raster::Luma;
use crate::rng::hash_unit;
use std::f64::consts::TAU;

/// Flèche des congés et ellipses des contrôleurs (mm).
const CURVE_TOLERANCE: f64 = 0.05;

enum Form {
    Point(Vec2),
    Circle {
        c: Vec2,
        r: f64,
    },
    Path(Polyline),
    Image {
        at: Vec2,
        width: f64,
        height: f64,
        luma: Option<Luma>,
    },
}

/// Un contrôleur prêt à être évalué (tracés aplatis, image décodée).
pub struct Source<'a> {
    pub ctrl: &'a Controller,
    form: Form,
}

impl<'a> Source<'a> {
    pub fn new(ctrl: &'a Controller) -> Self {
        let form = match &ctrl.geometry {
            Geometry::Point { at } => Form::Point(*at),
            Geometry::Line { a, b } => Form::Path(Polyline::new(vec![*a, *b], false, ctrl.extend)),
            Geometry::Curve(c) => Form::Path(Polyline::new(
                flatten(c, CURVE_TOLERANCE),
                c.closed,
                ctrl.extend,
            )),
            Geometry::Circle { center, radius } => Form::Circle {
                c: *center,
                r: *radius,
            },
            Geometry::Ellipse {
                center,
                rx,
                ry,
                angle,
            } => {
                let n = 96;
                let pts = (0..n)
                    .map(|k| {
                        let t = TAU * k as f64 / n as f64;
                        *center + Vec2::new(rx * t.cos(), ry * t.sin()).rotated(angle * DEG)
                    })
                    .collect();
                Form::Path(Polyline::new(pts, true, false))
            }
            Geometry::Polygon {
                center,
                radius,
                sides,
                angle,
            } => {
                let n = (*sides).max(3) as usize;
                let pts = (0..n)
                    .map(|k| {
                        *center
                            + Vec2::from_angle(angle * DEG + TAU * k as f64 / n as f64) * *radius
                    })
                    .collect();
                Form::Path(Polyline::new(pts, true, false))
            }
            Geometry::Image {
                at,
                width,
                height,
                raster,
            } => Form::Image {
                at: *at,
                width: *width,
                height: *height,
                luma: Luma::decode(raster),
            },
        };
        Self { ctrl, form }
    }

    /// Distance au contrôleur (0 à l'intérieur d'un tracé rempli) et côté
    /// (+1 / -1). Le côté d'un tracé fermé est l'intérieur (+1) ou
    /// l'extérieur ; celui d'un point, la droite (+1) ou la gauche.
    fn probe(&self, p: Vec2) -> (f64, f64) {
        match &self.form {
            Form::Point(at) => (p.distance(*at), if p.x >= at.x { 1.0 } else { -1.0 }),
            Form::Circle { c, r } => {
                let d = p.distance(*c);
                let inside = d < *r;
                let dist = if inside && self.ctrl.fill {
                    0.0
                } else {
                    (d - r).abs()
                };
                (dist, if inside { 1.0 } else { -1.0 })
            }
            Form::Path(pl) => {
                let (d, side) = pl.probe(p);
                if pl.closed {
                    let inside = pl.contains(p);
                    let dist = if inside && self.ctrl.fill { 0.0 } else { d };
                    (dist, if inside { 1.0 } else { -1.0 })
                } else {
                    (d, side)
                }
            }
            Form::Image { .. } => (0.0, 1.0),
        }
    }

    /// Poids et côté de l'influence `inf` au point `p`.
    pub fn weight(&self, inf: &Influence, p: Vec2) -> (f64, f64) {
        if let Form::Image {
            at,
            width,
            height,
            luma,
        } = &self.form
        {
            let Some(l) = luma
                .as_ref()
                .and_then(|l| l.sample(*at, *width, *height, p))
            else {
                return (0.0, 1.0);
            };
            let dark = if inf.invert { l } else { 1.0 - l };
            let w = ((dark - 0.5) * inf.contrast + 0.5).clamp(0.0, 1.0);
            let w = if inf.hard_edge {
                f64::from(u8::from(w >= 0.5))
            } else {
                w
            };
            return (w, 1.0);
        }
        let (d, side) = self.probe(p);
        let wanted = match self.ctrl.side {
            Side::Both => 0.0,
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        if wanted != 0.0 && self.ctrl.geometry.is_open_path() && side != wanted {
            return (0.0, side);
        }
        (falloff(inf, d), side)
    }

    /// Proximité linéaire `1 − d / portée` dans la portée (poids pour une image).
    fn proximity(&self, inf: &Influence, p: Vec2) -> Option<f64> {
        if matches!(self.form, Form::Image { .. }) {
            let (w, _) = self.weight(inf, p);
            return (w > 0.0).then_some(w);
        }
        let (w, _) = self.weight(inf, p);
        if w <= 0.0 {
            return None;
        }
        let (d, _) = self.probe(p);
        Some((1.0 - d / inf.reach).clamp(0.0, 1.0))
    }
}

/// Atténuation avec la distance.
pub fn falloff(inf: &Influence, d: f64) -> f64 {
    if inf.hard_edge {
        return if d <= inf.reach { 1.0 } else { 0.0 };
    }
    if inf.reach <= 0.0 {
        return 0.0;
    }
    let t = (1.0 - d / inf.reach).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Tous les contrôleurs d'un projet, préparés une fois par calcul.
pub struct Field<'a> {
    sources: Vec<Source<'a>>,
}

impl<'a> Field<'a> {
    pub fn new(controllers: &'a [Controller]) -> Self {
        Self {
            sources: controllers.iter().map(Source::new).collect(),
        }
    }

    pub fn channel(&self, ch: Channel) -> ChannelField<'_> {
        ChannelField {
            items: self
                .sources
                .iter()
                .filter_map(|s| s.ctrl.influence(ch).map(|i| (s, i)))
                .collect(),
        }
    }
}

/// Contrôleurs d'un canal.
pub struct ChannelField<'a> {
    items: Vec<(&'a Source<'a>, &'a Influence)>,
}

impl ChannelField<'_> {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn value(&self, base: f64, p: Vec2) -> f64 {
        let (mut wmax, mut wsum, mut acc) = (0.0f64, 0.0, 0.0);
        let mut step = f64::INFINITY;
        for (src, inf) in &self.items {
            let (w, side) = src.weight(inf, p);
            if w > 0.0 {
                let target = if inf.symmetric {
                    base + side * (inf.target - base)
                } else {
                    inf.target
                };
                wmax = wmax.max(w);
                wsum += w;
                acc += w * target;
                if inf.step > 0.0 {
                    step = step.min(inf.step);
                }
            }
        }
        if wsum <= 0.0 {
            return base;
        }
        let v = base + wmax * (acc / wsum - base);
        if step.is_finite() {
            base + ((v - base) / step).round() * step
        } else {
            v
        }
    }

    /// Plus petite et plus grande valeur que le canal peut prendre.
    pub fn bounds(&self, base: f64) -> (f64, f64) {
        let mut lo = base;
        let mut hi = base;
        for (_, inf) in &self.items {
            for t in [
                inf.target,
                if inf.symmetric {
                    2.0 * base - inf.target
                } else {
                    inf.target
                },
            ] {
                lo = lo.min(t);
                hi = hi.max(t);
            }
        }
        (lo, hi)
    }

    /// Vrai si une disparition aléatoire retire la perforation centrée en `p`.
    pub fn culled(&self, p: Vec2) -> bool {
        for (src, inf) in &self.items {
            let Some(cull) = &inf.cull else { continue };
            let Some(t) = src.proximity(inf, p) else {
                continue;
            };
            let prob = cull.amount.clamp(0.0, 1.0)
                * match cull.mode {
                    CullMode::Center => t,
                    CullMode::Edge => 1.0 - t,
                    CullMode::Uniform => 1.0,
                };
            if hash_unit(cull.seed ^ ((src.ctrl.id as u64) << 32), p) < prob {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Cull, Curve, Raster};

    fn point(id: u32, x: f64, reach: f64, target: f64) -> Controller {
        Controller {
            size: Some(Influence::new(reach, target)),
            ..Controller::new(
                id,
                Geometry::Point {
                    at: Vec2::new(x, 0.0),
                },
            )
        }
    }

    fn value(cs: &[Controller], ch: Channel, base: f64, p: Vec2) -> f64 {
        Field::new(cs).channel(ch).value(base, p)
    }

    #[test]
    fn base_far_target_near() {
        let cs = [point(1, 0.0, 10.0, 8.0)];
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(0.0, 0.0)), 8.0);
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(10.0, 0.0)), 4.0);
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(5.0, 0.0)), 6.0);
    }

    #[test]
    fn overlapping_controllers_never_overshoot() {
        let cs = [point(1, 0.0, 20.0, 8.0), point(2, 10.0, 20.0, 2.0)];
        for i in 0..=40 {
            let v = value(&cs, Channel::Size, 5.0, Vec2::new(-10.0 + i as f64, 0.0));
            assert!((2.0..=8.0).contains(&v), "{v}");
        }
        assert!((value(&cs, Channel::Size, 5.0, Vec2::new(5.0, 0.0)) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn hard_edge_is_a_step() {
        let mut c = point(1, 0.0, 10.0, 8.0);
        c.size.as_mut().unwrap().hard_edge = true;
        let cs = [c];
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(9.9, 0.0)), 8.0);
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(10.1, 0.0)), 4.0);
    }

    #[test]
    fn channels_are_independent() {
        let cs = [point(1, 0.0, 10.0, 8.0)];
        assert!(Field::new(&cs).channel(Channel::Angle).is_empty());
    }

    fn line(side: Side, extend: bool) -> Controller {
        Controller {
            side,
            extend,
            size: Some(Influence::new(10.0, 8.0)),
            ..Controller::new(
                1,
                Geometry::Line {
                    a: Vec2::new(0.0, 0.0),
                    b: Vec2::new(10.0, 0.0),
                },
            )
        }
    }

    #[test]
    fn one_sided_line_acts_on_its_left_only() {
        let cs = [line(Side::Left, false)];
        assert!(value(&cs, Channel::Size, 4.0, Vec2::new(5.0, 2.0)) > 4.0);
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(5.0, -2.0)), 4.0);
    }

    #[test]
    fn extended_line_reaches_beyond_its_ends() {
        let short = [line(Side::Both, false)];
        let long = [line(Side::Both, true)];
        let p = Vec2::new(40.0, 1.0);
        assert_eq!(value(&short, Channel::Size, 4.0, p), 4.0);
        assert!(value(&long, Channel::Size, 4.0, p) > 7.0);
    }

    #[test]
    fn filled_closed_curve_targets_its_inside() {
        let square = Curve {
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(40.0, 0.0),
                Vec2::new(40.0, 40.0),
                Vec2::new(0.0, 40.0),
            ],
            closed: true,
            fillets: vec![],
        };
        let mut c = Controller {
            size: Some(Influence::new(5.0, 8.0)),
            ..Controller::new(1, Geometry::Curve(square))
        };
        let centre = Vec2::new(20.0, 20.0);
        assert_eq!(value(&[c.clone()], Channel::Size, 4.0, centre), 4.0);
        c.fill = true;
        assert_eq!(value(&[c], Channel::Size, 4.0, centre), 8.0);
    }

    #[test]
    fn symmetric_angle_and_steps() {
        let mut c = line(Side::Both, true);
        c.size = None;
        c.angle = Some(Influence {
            symmetric: true,
            ..Influence::new(10.0, 40.0)
        });
        let cs = [c.clone()];
        let up = value(&cs, Channel::Angle, 0.0, Vec2::new(5.0, 1.0));
        let down = value(&cs, Channel::Angle, 0.0, Vec2::new(5.0, -1.0));
        assert!(up > 30.0 && (up + down).abs() < 1e-9);
        c.angle.as_mut().unwrap().step = 15.0;
        let stepped = value(&[c], Channel::Angle, 0.0, Vec2::new(5.0, 3.0));
        assert_eq!(stepped % 15.0, 0.0);
    }

    #[test]
    fn image_darkness_drives_the_value() {
        // 2 × 1 : pixel gauche noir, droit blanc.
        let c = Controller {
            size: Some(Influence::new(1.0, 8.0)),
            ..Controller::new(
                1,
                Geometry::Image {
                    at: Vec2::ZERO,
                    width: 20.0,
                    height: 10.0,
                    raster: Raster {
                        width: 2,
                        height: 1,
                        luma: "AP8=".into(),
                    },
                },
            )
        };
        let cs = [c.clone()];
        assert!((value(&cs, Channel::Size, 4.0, Vec2::new(2.0, 5.0)) - 8.0).abs() < 1e-9);
        assert!((value(&cs, Channel::Size, 4.0, Vec2::new(18.0, 5.0)) - 4.0).abs() < 1e-9);
        assert_eq!(value(&cs, Channel::Size, 4.0, Vec2::new(30.0, 5.0)), 4.0);
        let mut inv = c;
        inv.size.as_mut().unwrap().invert = true;
        assert!((value(&[inv], Channel::Size, 4.0, Vec2::new(18.0, 5.0)) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn cull_is_reproducible_and_proportional() {
        let mut c = point(1, 0.0, 1000.0, 5.0);
        c.size.as_mut().unwrap().cull = Some(Cull {
            amount: 0.5,
            mode: CullMode::Uniform,
            seed: 3,
        });
        let cs = [c];
        let f = Field::new(&cs);
        let ch = f.channel(Channel::Size);
        let pts: Vec<Vec2> = (0..2000)
            .map(|i| Vec2::new((i % 50) as f64, (i / 50) as f64))
            .collect();
        let a: Vec<bool> = pts.iter().map(|&p| ch.culled(p)).collect();
        let b: Vec<bool> = pts.iter().map(|&p| ch.culled(p)).collect();
        assert_eq!(a, b);
        let n = a.iter().filter(|x| **x).count();
        assert!((800..1200).contains(&n), "{n}");
    }
}
