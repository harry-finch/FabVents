//! Aléatoire reproductible : même graine, même motif, sur toutes les
//! plateformes (SplitMix64, sans dépendance).

use crate::geom::Vec2;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Réel uniforme dans `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Tirage uniforme `[0, 1)` attaché à une position (au micromètre près) et à
/// une graine : une perforation garde son tirage quand le reste du motif change.
pub fn hash_unit(seed: u64, p: Vec2) -> f64 {
    let qx = (p.x * 1e3).round() as i64 as u64;
    let qy = (p.y * 1e3).round() as i64 as u64;
    let h = mix(mix(seed ^ 0xD1B5_4A32_D192_ED03)
        ^ qx.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ qy.rotate_left(32));
    (h >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let mut c = Rng::new(43);
        let va: Vec<f64> = (0..8).map(|_| a.unit()).collect();
        let vb: Vec<f64> = (0..8).map(|_| b.unit()).collect();
        let vc: Vec<f64> = (0..8).map(|_| c.unit()).collect();
        assert_eq!(va, vb);
        assert_ne!(va, vc);
        assert!(va.iter().all(|v| (0.0..1.0).contains(v)));
    }

    #[test]
    fn positional_draw_is_stable_and_spread() {
        let p = Vec2::new(12.5, 3.25);
        assert_eq!(hash_unit(7, p), hash_unit(7, p));
        assert_ne!(hash_unit(7, p), hash_unit(8, p));
        let mean = (0..1000)
            .map(|i| hash_unit(1, Vec2::new(i as f64 * 0.7, 3.0)))
            .sum::<f64>()
            / 1000.0;
        assert!((mean - 0.5).abs() < 0.05, "{mean}");
    }
}
