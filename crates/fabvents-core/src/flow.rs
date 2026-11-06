//! Lignes fluides : lignes de courant d'un champ de directions (angle de
//! base + canal Angle), semées le long d'une droite transverse au pas du
//! canal Espacement et arrêtées avant le bord, un trou de fixation ou une
//! ligne voisine trop proche ; puis rubans d'épaisseur variable (canal
//! Taille), nettoyés de leurs auto-intersections.

use crate::boolean::cut;
use crate::field::ChannelField;
use crate::geom::{signed_area, Vec2, DEG};
use crate::project::{Flow, FlowEnds};
use crate::region::Region;
use std::collections::HashMap;
use std::f64::consts::PI;

/// Lignes au plus.
pub const MAX_LINES: usize = 2_000;
/// Points au plus par ligne.
const MAX_STEPS: usize = 20_000;

/// Ce que doit éviter un ruban : bord de la plaque (moins la marge), zone
/// perforée, trous de fixation et zones réservées (avec leur garde).
pub struct Bounds<'a> {
    pub plate: &'a Region,
    pub margin: f64,
    pub zone: Option<&'a Region>,
    pub keepouts: &'a [(Region, f64)],
}

impl Bounds<'_> {
    /// Un disque de rayon `r` centré en `p` tient-il dans la zone utile ?
    fn free(&self, p: Vec2, r: f64) -> bool {
        self.plate.sdf(p) <= -(self.margin + r)
            && self.zone.is_none_or(|z| z.sdf(p) <= -r)
            && self.keepouts.iter().all(|(k, c)| k.sdf(p) >= r + c)
    }
}

/// Lignes de courant (polylignes ouvertes, pas d'intégration `step`).
pub fn streamlines(
    f: &Flow,
    center: Vec2,
    half_diag: f64,
    bounds: &Bounds,
    spacing: &dyn Fn(Vec2) -> f64,
    angle: &ChannelField,
    width: &dyn Fn(Vec2) -> f64,
) -> Result<Vec<Vec<Vec2>>, usize> {
    let dir = |p: Vec2| Vec2::from_angle(angle.value(f.angle, p) * DEG);
    let s_min = spacing(center).min(f.spacing).max(0.2);
    let step = (s_min / 4.0).clamp(0.1, 1.0);
    // Points des lignes déjà tracées, pour l'écart minimal entre lignes.
    let cell = s_min.max(0.5);
    let key = |p: Vec2| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
    let mut taken: HashMap<(i64, i64), Vec<Vec2>> = HashMap::new();
    let crowded = |p: Vec2, taken: &HashMap<(i64, i64), Vec<Vec2>>| {
        // Axes à au moins une demi-largeur d'écart, et assez loin pour
        // que deux rubans ne se touchent pas.
        let d = (0.5 * spacing(p)).max(1.25 * width(p));
        let (gx, gy) = key(p);
        let reach = (d / cell).ceil() as i64;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if let Some(v) = taken.get(&(gx + dx, gy + dy)) {
                    if v.iter().any(|q| q.distance(p) < d) {
                        return true;
                    }
                }
            }
        }
        false
    };
    let ok = |p: Vec2| bounds.free(p, width(p) / 2.0);

    // Germes le long de la droite transverse passant par le centre.
    let across = Vec2::from_angle(f.angle * DEG).left();
    let mut starts = Vec::new();
    let mut t = 0.0;
    while t <= half_diag {
        starts.push(center + across * t);
        if t > 0.0 {
            starts.push(center - across * t);
        }
        t += spacing(center + across * t).max(0.2);
        if starts.len() > 2 * MAX_LINES {
            return Err(starts.len());
        }
    }
    starts.sort_by(|a, b| a.distance(center).total_cmp(&b.distance(center)));

    let mut lines = Vec::new();
    for s in starts {
        if !ok(s) || crowded(s, &taken) {
            continue;
        }
        let trace = |sign: f64| {
            let mut out = Vec::new();
            let mut p = s;
            for _ in 0..MAX_STEPS {
                // Point milieu (Runge-Kutta 2).
                let mid = p + dir(p) * (sign * step / 2.0);
                let q = p + dir(mid) * (sign * step);
                if !ok(q) || crowded(q, &taken) {
                    break;
                }
                out.push(q);
                p = q;
            }
            out
        };
        let mut back = trace(-1.0);
        back.reverse();
        back.push(s);
        back.extend(trace(1.0));
        // Ligne trop courte pour un ruban : abandonnée.
        let len: f64 = back.windows(2).map(|w| w[0].distance(w[1])).sum();
        if len < 2.0 * width(s) {
            continue;
        }
        for &q in &back {
            taken.entry(key(q)).or_default().push(q);
        }
        lines.push(back);
        if lines.len() > MAX_LINES {
            return Err(lines.len());
        }
    }
    Ok(lines)
}

/// Contour du ruban d'une ligne : largeur `width` en chaque point,
/// extrémités rondes ou plates, auto-intersections fondues, sommets alignés
/// retirés (écart 0,005 mm). Sens trigonométrique.
pub fn ribbon(line: &[Vec2], width: &dyn Fn(Vec2) -> f64, ends: FlowEnds) -> Option<Vec<Vec2>> {
    let n = line.len();
    if n < 2 {
        return None;
    }
    let tangent = |i: usize| {
        let a = line[i.saturating_sub(1)];
        let b = line[(i + 1).min(n - 1)];
        (b - a).normalized()
    };
    let mut left = Vec::with_capacity(n);
    let mut right = Vec::with_capacity(n);
    for (i, &p) in line.iter().enumerate() {
        let nrm = tangent(i).left() * (width(p) / 2.0);
        left.push(p + nrm);
        right.push(p - nrm);
    }
    let cap = |p: Vec2, from: Vec2, out: &mut Vec<Vec2>| {
        if ends == FlowEnds::Round {
            // Demi-cercle de `from` (sur le ruban) autour de `p`, côté extérieur.
            let r = from.distance(p);
            let a0 = (from - p).angle();
            let k = ((PI * r / 0.05).ceil() as usize).clamp(6, 48);
            for j in 1..k {
                out.push(p + Vec2::from_angle(a0 - PI * j as f64 / k as f64) * r);
            }
        }
    };
    let mut poly = Vec::with_capacity(2 * n + 64);
    poly.extend(left.iter().copied());
    cap(line[n - 1], left[n - 1], &mut poly);
    poly.extend(right.iter().rev().copied());
    cap(line[0], right[0], &mut poly);
    // Fusion des boucles (virage plus serré que la demi-largeur).
    let (mut lo, mut hi) = (poly[0], poly[0]);
    for &q in &poly {
        lo = Vec2::new(lo.x.min(q.x), lo.y.min(q.y));
        hi = Vec2::new(hi.x.max(q.x), hi.y.max(q.y));
    }
    let frame = vec![
        lo - Vec2::new(1.0, 1.0),
        Vec2::new(hi.x + 1.0, lo.y - 1.0),
        hi + Vec2::new(1.0, 1.0),
        Vec2::new(lo.x - 1.0, hi.y + 1.0),
    ];
    let shapes = cut(&[poly], &[frame], &[]);
    let outer = shapes
        .into_iter()
        .map(|c| c.outer)
        .max_by(|a, b| signed_area(a).abs().total_cmp(&signed_area(b).abs()))?;
    let mut closed = outer.clone();
    closed.push(outer[0]);
    let mut out = crate::fit::simplify(&closed, 0.005);
    if signed_area(&out) < 0.0 {
        out.reverse();
    }
    (out.len() >= 3).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;
    use crate::project::{Channel, Plate};

    #[test]
    fn straight_ribbon_has_the_exact_area() {
        let line: Vec<Vec2> = (0..=20).map(|i| Vec2::new(i as f64, 0.0)).collect();
        let flat = ribbon(&line, &|_| 2.0, FlowEnds::Flat).unwrap();
        assert!((signed_area(&flat) - 40.0).abs() < 1e-6);
        assert!(flat.len() <= 6, "{}", flat.len());
        let round = ribbon(&line, &|_| 2.0, FlowEnds::Round).unwrap();
        // Bouts ronds facettisés puis simplifiés à 0,005 mm près.
        assert!(
            (signed_area(&round) - (40.0 + PI)).abs() < 0.1,
            "{}",
            signed_area(&round)
        );
    }

    #[test]
    fn lines_fill_the_plate_and_keep_apart() {
        let plate = Plate {
            width: 100.0,
            height: 60.0,
            margin: 5.0,
            ..Plate::default()
        };
        let region = plate.region();
        let bounds = Bounds {
            plate: &region,
            margin: plate.margin,
            zone: None,
            keepouts: &[],
        };
        let f = Flow::default();
        let field = Field::new(&[]);
        let lines = streamlines(
            &f,
            plate.center(),
            60.0,
            &bounds,
            &|_| f.spacing,
            &field.channel(Channel::Angle),
            &|_| f.width,
        )
        .unwrap();
        // Axes possibles entre y = 6,5 et 53,5 (marge + demi-ruban), tous
        // les 8 mm depuis le centre (y = 30) : 14, 22, 30, 38, 46.
        assert_eq!(lines.len(), 5, "{}", lines.len());
        for l in &lines {
            assert!(l.iter().all(|p| (p.y - l[0].y).abs() < 1e-9));
            let len = l.last().unwrap().x - l[0].x;
            assert!(len > 85.0, "{len}");
        }
    }
}
