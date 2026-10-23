//! Image en niveaux de gris d'un contrôleur Image : décodage (base64) et
//! lecture de la luminance en un point de la plaque (interpolation bilinéaire).

use crate::geom::Vec2;
use crate::project::Raster;

/// Taille maximale d'une image de contrôleur (pixels).
pub const MAX_PIXELS: u64 = 512 * 512;

pub struct Luma {
    w: usize,
    h: usize,
    /// Luminance 0 à 1, ligne du haut d'abord.
    px: Vec<f32>,
}

impl Luma {
    /// `None` si les données ne correspondent pas aux dimensions.
    pub fn decode(r: &Raster) -> Option<Luma> {
        let (w, h) = (r.width as usize, r.height as usize);
        if w == 0 || h == 0 || (w as u64) * (h as u64) > MAX_PIXELS {
            return None;
        }
        let bytes = base64_decode(&r.luma)?;
        if bytes.len() != w * h {
            return None;
        }
        Some(Luma {
            w,
            h,
            px: bytes.into_iter().map(|b| b as f32 / 255.0).collect(),
        })
    }

    /// Luminance au point `p` d'une image posée en `at` (coin bas gauche),
    /// de `width` × `height` mm ; `None` hors de l'image.
    pub fn sample(&self, at: Vec2, width: f64, height: f64, p: Vec2) -> Option<f64> {
        let u = (p.x - at.x) / width;
        let v = 1.0 - (p.y - at.y) / height;
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }
        let x = (u * self.w as f64 - 0.5).clamp(0.0, (self.w - 1) as f64);
        let y = (v * self.h as f64 - 0.5).clamp(0.0, (self.h - 1) as f64);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = (x - x0 as f64, y - y0 as f64);
        let at = |xx: usize, yy: usize| self.px[yy * self.w + xx] as f64;
        let top = at(x0, y0) * (1.0 - fx) + at(x1, y0) * fx;
        let bottom = at(x0, y1) * (1.0 - fx) + at(x1, y1) * fx;
        Some(top * (1.0 - fy) + bottom * fy)
    }
}

/// Décodage base64 standard (avec ou sans `=` final, blancs ignorés).
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        acc = (acc << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_base64() {
        assert_eq!(base64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(base64_decode("TWE=").unwrap(), b"Ma");
        assert_eq!(base64_decode("TQ==").unwrap(), b"M");
        assert!(base64_decode("T*==").is_none());
    }

    #[test]
    fn samples_top_row_first() {
        // 2 × 2 : ligne du haut noire (0), ligne du bas blanche (255).
        let r = Raster {
            width: 2,
            height: 2,
            luma: "AAD//w==".into(),
        };
        let img = Luma::decode(&r).unwrap();
        let at = Vec2::ZERO;
        assert!(img.sample(at, 10.0, 10.0, Vec2::new(5.0, 9.9)).unwrap() < 0.05);
        assert!(img.sample(at, 10.0, 10.0, Vec2::new(5.0, 0.1)).unwrap() > 0.95);
        assert!(img.sample(at, 10.0, 10.0, Vec2::new(11.0, 5.0)).is_none());
        let bad = Raster {
            luma: "AAA=".into(),
            ..r
        };
        assert!(Luma::decode(&bad).is_none());
    }
}
