//! Aperçu PNG d'un motif, sans fenêtre (`tiny-skia`) : la plaque en gris,
//! les perforations vides (fond transparent), à la largeur demandée.

use fabvents_core::{export::Drawing, Contour, Frame, Project};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

/// Flèche des arcs dans l'aperçu (mm).
const TOLERANCE: f64 = 0.02;

fn add(pb: &mut PathBuilder, c: &Contour, map: &dyn Fn(f64, f64) -> (f32, f32)) {
    let pts = c.tessellated(TOLERANCE);
    for (i, p) in pts.iter().enumerate() {
        let (x, y) = map(p.x, p.y);
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
}

/// PNG de `width` pixels de large (la hauteur suit les proportions).
pub fn png(project: &Project, frame: &Frame, width: u32) -> Result<Vec<u8>, String> {
    let ext = project.plate.extent();
    if ext.x <= 0.0 || ext.y <= 0.0 {
        return Err("plaque vide".into());
    }
    let width = width.clamp(16, 4096);
    let k = width as f64 / ext.x;
    let height = ((ext.y * k).ceil() as u32).max(1);
    let mut pixmap = Pixmap::new(width, height).ok_or("taille d'image invalide")?;
    // Y vers le haut dans le moteur, vers le bas dans l'image.
    let map = move |x: f64, y: f64| ((x * k) as f32, ((ext.y - y) * k) as f32);
    let mut pb = PathBuilder::new();
    add(&mut pb, &project.plate.contour(), &map);
    for c in Drawing::new(project, frame).inner_contours() {
        add(&mut pb, &c, &map);
    }
    let path = pb.finish().ok_or("contour vide")?;
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(0x3a, 0x3d, 0x42, 0xff));
    paint.anti_alias = true;
    // Pair-impair : les perforations percent la plaque, les îlots restent pleins.
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::EvenOdd,
        Transform::identity(),
        None,
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// Base64 (RFC 4648) pour les images MCP.
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_reference_values() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn example_preview_is_a_png() {
        let p = Project::example();
        let mut pl = fabvents_core::Pipeline::new();
        let f = pl.compute(&p);
        let img = png(&p, &f, 400).unwrap();
        assert_eq!(&img[1..4], b"PNG");
    }
}
