//! Champs d'influence : valeur d'un canal en un point (ADR-0004).
//!
//! Chaque contrôleur donne un poids `w ∈ [0, 1]` (1 sur le contrôleur, 0 à
//! sa portée, transition *smoothstep*, ou seuil franc). Les contrôleurs d'un
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

use crate::geom::Vec2;
use crate::project::{Channel, Controller, Influence};

/// Poids d'une influence à la distance `d` du contrôleur.
pub fn weight(inf: &Influence, d: f64) -> f64 {
    if inf.hard_edge {
        return if d <= inf.reach { 1.0 } else { 0.0 };
    }
    if inf.reach <= 0.0 {
        return 0.0;
    }
    let t = (1.0 - d / inf.reach).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Contrôleurs d'un canal, prêts à être évalués.
pub struct ChannelField<'a> {
    items: Vec<(&'a Controller, &'a Influence)>,
}

impl<'a> ChannelField<'a> {
    pub fn new(controllers: &'a [Controller], ch: Channel) -> Self {
        Self {
            items: controllers
                .iter()
                .filter_map(|c| c.influence(ch).map(|i| (c, i)))
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn value(&self, base: f64, p: Vec2) -> f64 {
        let (mut wmax, mut wsum, mut acc) = (0.0f64, 0.0, 0.0);
        for (c, inf) in &self.items {
            let w = weight(inf, c.geometry.distance(p));
            if w > 0.0 {
                wmax = wmax.max(w);
                wsum += w;
                acc += w * inf.target;
            }
        }
        if wsum <= 0.0 {
            base
        } else {
            base + wmax * (acc / wsum - base)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Geometry;

    fn point(id: u32, x: f64, reach: f64, target: f64) -> Controller {
        Controller {
            id,
            geometry: Geometry::Point {
                at: Vec2::new(x, 0.0),
            },
            size: Some(Influence {
                reach,
                target,
                hard_edge: false,
            }),
            angle: None,
        }
    }

    #[test]
    fn base_far_target_near() {
        let cs = [point(1, 0.0, 10.0, 8.0)];
        let f = ChannelField::new(&cs, Channel::Size);
        assert_eq!(f.value(4.0, Vec2::new(0.0, 0.0)), 8.0);
        assert_eq!(f.value(4.0, Vec2::new(10.0, 0.0)), 4.0);
        assert_eq!(f.value(4.0, Vec2::new(5.0, 0.0)), 6.0);
    }

    #[test]
    fn overlapping_controllers_never_overshoot() {
        let cs = [point(1, 0.0, 20.0, 8.0), point(2, 10.0, 20.0, 2.0)];
        let f = ChannelField::new(&cs, Channel::Size);
        for i in 0..=40 {
            let v = f.value(5.0, Vec2::new(-10.0 + i as f64, 0.0));
            assert!((2.0..=8.0).contains(&v), "{v}");
        }
        assert!((f.value(5.0, Vec2::new(5.0, 0.0)) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn hard_edge_is_a_step() {
        let mut c = point(1, 0.0, 10.0, 8.0);
        c.size.as_mut().unwrap().hard_edge = true;
        let cs = [c];
        let f = ChannelField::new(&cs, Channel::Size);
        assert_eq!(f.value(4.0, Vec2::new(9.9, 0.0)), 8.0);
        assert_eq!(f.value(4.0, Vec2::new(10.1, 0.0)), 4.0);
    }

    #[test]
    fn channels_are_independent() {
        let cs = [point(1, 0.0, 10.0, 8.0)];
        assert!(ChannelField::new(&cs, Channel::Angle).is_empty());
    }
}
