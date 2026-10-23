// Géométrie des contrôleurs côté interface : poignées, contour (pour le
// dessin et la sélection), déplacement d'une poignée, poignée de portée et
// conversion d'un tracé fermé en forme exacte. Repère de la plaque (mm, Y
// vers le haut). Les congés sont aplatis comme dans `fabvents-core::curve`.

import type { CtrlGeometry, Curve, Vec2 } from './types';

export const DEG = Math.PI / 180;

export const add = (a: Vec2, b: Vec2): Vec2 => ({ x: a.x + b.x, y: a.y + b.y });
export const sub = (a: Vec2, b: Vec2): Vec2 => ({ x: a.x - b.x, y: a.y - b.y });
export const mul = (a: Vec2, k: number): Vec2 => ({ x: a.x * k, y: a.y * k });
export const len = (a: Vec2) => Math.hypot(a.x, a.y);
export const dist = (a: Vec2, b: Vec2) => Math.hypot(a.x - b.x, a.y - b.y);
export const dir = (deg: number): Vec2 => ({ x: Math.cos(deg * DEG), y: Math.sin(deg * DEG) });
export const angleOf = (v: Vec2) => Math.atan2(v.y, v.x) / DEG;
export const round2 = (p: Vec2): Vec2 => ({ x: Math.round(p.x * 100) / 100, y: Math.round(p.y * 100) / 100 });
const rnd = (v: number, k = 100) => Math.round(v * k) / k;

export function distSeg(p: Vec2, a: Vec2, b: Vec2): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const l2 = dx * dx + dy * dy;
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2)) : 0;
  return Math.hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t));
}

export function distPolyline(p: Vec2, pts: Vec2[], closed: boolean): number {
  let best = Infinity;
  const n = pts.length;
  if (n === 1) return dist(p, pts[0]);
  for (let i = 0; i + 1 < n; i++) best = Math.min(best, distSeg(p, pts[i], pts[i + 1]));
  if (closed && n > 2) best = Math.min(best, distSeg(p, pts[n - 1], pts[0]));
  return best;
}

/** Polyligne d'un tracé, congés aplatis (même règle que le moteur). */
export function flattenCurve(c: Curve, segs = 10): Vec2[] {
  const p = c.points;
  const n = p.length;
  const out: Vec2[] = [];
  for (let i = 0; i < n; i++) {
    const cur = p[i];
    const r = c.fillets?.[i] ?? 0;
    const interior = c.closed || (i > 0 && i + 1 < n);
    if (r <= 0 || !interior || n < 3) {
      out.push(cur);
      continue;
    }
    const prev = p[(i + n - 1) % n];
    const next = p[(i + 1) % n];
    const lp = dist(prev, cur);
    const ln = dist(next, cur);
    if (lp < 1e-9 || ln < 1e-9) {
      out.push(cur);
      continue;
    }
    const uIn = mul(sub(prev, cur), 1 / lp);
    const uOut = mul(sub(next, cur), 1 / ln);
    const alpha = Math.acos(Math.max(-1, Math.min(1, uIn.x * uOut.x + uIn.y * uOut.y)));
    if (alpha < 1e-6 || alpha > Math.PI - 1e-6) {
      out.push(cur);
      continue;
    }
    const tanHalf = Math.tan(alpha / 2);
    const setback = Math.min(r / tanHalf, 0.5 * Math.min(lp, ln));
    const radius = setback * tanHalf;
    const bis = add(uIn, uOut);
    const center = add(cur, mul(bis, radius / Math.sin(alpha / 2) / len(bis)));
    const tIn = add(cur, mul(uIn, setback));
    const tOut = add(cur, mul(uOut, setback));
    const a0 = Math.atan2(tIn.y - center.y, tIn.x - center.x);
    let sweep = Math.atan2(tOut.y - center.y, tOut.x - center.x) - a0;
    while (sweep > Math.PI) sweep -= 2 * Math.PI;
    while (sweep < -Math.PI) sweep += 2 * Math.PI;
    for (let k = 0; k <= segs; k++) {
      const a = a0 + (sweep * k) / segs;
      out.push({ x: center.x + Math.cos(a) * radius, y: center.y + Math.sin(a) * radius });
    }
  }
  return out;
}

export function isClosed(g: CtrlGeometry): boolean {
  return g.type === 'circle' || g.type === 'ellipse' || g.type === 'polygon' || g.type === 'rect' || (g.type === 'curve' && g.closed);
}

export function isOpenPath(g: CtrlGeometry): boolean {
  return g.type === 'line' || (g.type === 'curve' && !g.closed);
}

export function polygonVertices(center: Vec2, radius: number, sides: number, angle: number): Vec2[] {
  const n = Math.max(3, Math.round(sides));
  return Array.from({ length: n }, (_, k) => add(center, mul(dir(angle + (360 * k) / n), radius)));
}

function ellipsePoints(center: Vec2, rx: number, ry: number, angle: number, n = 72): Vec2[] {
  const c = Math.cos(angle * DEG);
  const s = Math.sin(angle * DEG);
  return Array.from({ length: n }, (_, k) => {
    const t = (2 * Math.PI * k) / n;
    const x = rx * Math.cos(t);
    const y = ry * Math.sin(t);
    return { x: center.x + x * c - y * s, y: center.y + x * s + y * c };
  });
}

/** Contour d'un contrôleur : polyligne et fermeture (null pour un point). */
export function outline(g: CtrlGeometry): { pts: Vec2[]; closed: boolean } | null {
  switch (g.type) {
    case 'point':
      return null;
    case 'line':
      return { pts: [g.a, g.b], closed: false };
    case 'curve':
      return { pts: flattenCurve(g), closed: g.closed };
    case 'circle':
      return { pts: ellipsePoints(g.center, g.radius, g.radius, 0), closed: true };
    case 'ellipse':
      return { pts: ellipsePoints(g.center, g.rx, g.ry, g.angle), closed: true };
    case 'polygon':
      return { pts: polygonVertices(g.center, g.radius, g.sides, g.angle), closed: true };
    case 'image':
      return {
        pts: [g.at, { x: g.at.x + g.width, y: g.at.y }, { x: g.at.x + g.width, y: g.at.y + g.height }, { x: g.at.x, y: g.at.y + g.height }],
        closed: true,
      };
    case 'rect':
      return { pts: roundedRect(g.center, g.width, g.height, g.corner), closed: true };
  }
}

/** Rectangle centré à coins arrondis, facettisé. */
export function roundedRect(c: Vec2, w: number, h: number, corner: number, segs = 8): Vec2[] {
  const r = Math.max(0, Math.min(corner, w / 2, h / 2));
  const hx = w / 2 - r;
  const hy = h / 2 - r;
  const out: Vec2[] = [];
  const corners: [number, number, number][] = [
    [hx, -hy, -90],
    [hx, hy, 0],
    [-hx, hy, 90],
    [-hx, -hy, 180],
  ];
  for (const [x, y, a0] of corners) {
    for (let k = 0; k <= (r > 0 ? segs : 0); k++) {
      const a = (a0 + (90 * k) / segs) * DEG;
      out.push({ x: c.x + x + r * Math.cos(a), y: c.y + y + r * Math.sin(a) });
    }
  }
  return out;
}

/** Distance d'un point au contrôleur (à son tracé). */
export function distanceTo(g: CtrlGeometry, p: Vec2): number {
  if (g.type === 'point') return dist(p, g.at);
  if (g.type === 'circle') return Math.abs(dist(p, g.center) - g.radius);
  const o = outline(g)!;
  return distPolyline(p, o.pts, o.closed);
}

export function insideImage(g: CtrlGeometry, p: Vec2): boolean {
  return g.type === 'image' && p.x >= g.at.x && p.y >= g.at.y && p.x <= g.at.x + g.width && p.y <= g.at.y + g.height;
}

/** Poignées déplaçables (hors corps et portée). Nœuds d'un tracé : `n:3`. */
export function handles(g: CtrlGeometry): { part: string; pos: Vec2 }[] {
  switch (g.type) {
    case 'point':
      return [{ part: 'at', pos: g.at }];
    case 'line':
      return [
        { part: 'a', pos: g.a },
        { part: 'b', pos: g.b },
      ];
    case 'curve':
      return g.points.map((pos, i) => ({ part: `n:${i}`, pos }));
    case 'circle':
      return [
        { part: 'center', pos: g.center },
        { part: 'radius', pos: add(g.center, { x: g.radius, y: 0 }) },
      ];
    case 'ellipse':
      return [
        { part: 'center', pos: g.center },
        { part: 'rx', pos: add(g.center, mul(dir(g.angle), g.rx)) },
        { part: 'ry', pos: add(g.center, mul(dir(g.angle + 90), g.ry)) },
      ];
    case 'polygon':
      return [
        { part: 'center', pos: g.center },
        { part: 'vertex', pos: add(g.center, mul(dir(g.angle), g.radius)) },
      ];
    case 'image':
      return [{ part: 'corner', pos: { x: g.at.x + g.width, y: g.at.y + g.height } }];
    case 'rect':
      return [
        { part: 'center', pos: g.center },
        { part: 'size', pos: { x: g.center.x + g.width / 2, y: g.center.y + g.height / 2 } },
      ];
  }
}

export function translate(g: CtrlGeometry, d: Vec2): CtrlGeometry {
  const m = (p: Vec2) => round2(add(p, d));
  switch (g.type) {
    case 'point':
      return { ...g, at: m(g.at) };
    case 'line':
      return { ...g, a: m(g.a), b: m(g.b) };
    case 'curve':
      return { ...g, points: g.points.map(m) };
    case 'circle':
    case 'ellipse':
    case 'polygon':
    case 'rect':
      return { ...g, center: m(g.center) };
    case 'image':
      return { ...g, at: m(g.at) };
  }
}

/** Nouvelle géométrie après avoir glissé la poignée `part` de `d` (curseur en `mp`). */
export function dragPart(g: CtrlGeometry, part: string, d: Vec2, mp: Vec2): CtrlGeometry {
  if (part === 'body' || part === 'center' || part === 'at') return translate(g, d);
  switch (g.type) {
    case 'line':
      return part === 'a' ? { ...g, a: round2(add(g.a, d)) } : { ...g, b: round2(add(g.b, d)) };
    case 'curve': {
      const i = Number(part.slice(2));
      return { ...g, points: g.points.map((p, k) => (k === i ? round2(add(p, d)) : p)) };
    }
    case 'circle':
      return { ...g, radius: Math.max(0.1, rnd(dist(mp, g.center), 10)) };
    case 'ellipse': {
      const v = sub(mp, g.center);
      if (part === 'rx') return { ...g, rx: Math.max(0.1, rnd(len(v), 10)), angle: rnd(angleOf(v), 10) };
      const n = dir(g.angle + 90);
      return { ...g, ry: Math.max(0.1, rnd(Math.abs(v.x * n.x + v.y * n.y), 10)) };
    }
    case 'polygon': {
      const v = sub(mp, g.center);
      return { ...g, radius: Math.max(0.1, rnd(len(v), 10)), angle: rnd(angleOf(v), 10) };
    }
    case 'image': {
      const k = Math.max(0.05, Math.max((mp.x - g.at.x) / g.width, (mp.y - g.at.y) / g.height));
      return { ...g, width: rnd(g.width * k, 10), height: rnd(g.height * k, 10) };
    }
    case 'rect':
      return { ...g, width: Math.max(0.2, rnd(2 * Math.abs(mp.x - g.center.x), 10)), height: Math.max(0.2, rnd(2 * Math.abs(mp.y - g.center.y), 10)) };
    default:
      return g;
  }
}

/** Position de la poignée de portée (null pour une image, sans portée). */
export function reachAnchor(g: CtrlGeometry, reach: number): Vec2 | null {
  switch (g.type) {
    case 'point':
      return add(g.at, { x: reach, y: 0 });
    case 'circle':
      return add(g.center, { x: g.radius + reach, y: 0 });
    case 'ellipse':
      return add(g.center, mul(dir(g.angle), g.rx + reach));
    case 'polygon':
      return add(g.center, mul(dir(g.angle), g.radius + reach));
    case 'image':
    case 'rect':
      return null;
    default: {
      const pts = g.type === 'line' ? [g.a, g.b] : g.points;
      if (pts.length < 2) return null;
      const a = pts[0];
      const b = pts[1];
      const d = sub(b, a);
      const l = len(d) || 1;
      const n = { x: -d.y / l, y: d.x / l };
      return add(mul(add(a, b), 0.5), mul(n, reach));
    }
  }
}

// ---- Conversion d'un tracé fermé en forme exacte

function denseRing(c: Curve): Vec2[] {
  const pts = flattenCurve(c);
  const out: Vec2[] = [];
  const n = pts.length;
  for (let i = 0; i < n; i++) {
    const a = pts[i];
    const b = pts[(i + 1) % n];
    const k = Math.max(1, Math.ceil(dist(a, b) / 0.5));
    for (let j = 0; j < k; j++) out.push({ x: a.x + ((b.x - a.x) * j) / k, y: a.y + ((b.y - a.y) * j) / k });
  }
  return out;
}

const mean = (pts: Vec2[]) => mul(pts.reduce(add, { x: 0, y: 0 }), 1 / pts.length);

export function toCircle(c: Curve): CtrlGeometry {
  const ring = denseRing(c);
  const center = mean(ring);
  const radius = ring.reduce((s, p) => s + dist(p, center), 0) / ring.length;
  return { type: 'circle', center: round2(center), radius: rnd(radius, 10) };
}

/** Ellipse par les moments d'ordre 2 du contour (axes principaux). */
export function toEllipse(c: Curve): CtrlGeometry {
  const ring = denseRing(c);
  const center = mean(ring);
  let sxx = 0;
  let syy = 0;
  let sxy = 0;
  for (const p of ring) {
    const dx = p.x - center.x;
    const dy = p.y - center.y;
    sxx += dx * dx;
    syy += dy * dy;
    sxy += dx * dy;
  }
  sxx /= ring.length;
  syy /= ring.length;
  sxy /= ring.length;
  const angle = 0.5 * Math.atan2(2 * sxy, sxx - syy);
  const tr = (sxx + syy) / 2;
  const det = Math.sqrt(((sxx - syy) / 2) ** 2 + sxy * sxy);
  return {
    type: 'ellipse',
    center: round2(center),
    rx: rnd(Math.sqrt(2 * (tr + det)), 10),
    ry: rnd(Math.sqrt(2 * Math.max(tr - det, 0)), 10),
    angle: rnd(angle / DEG, 10),
  };
}

export function toPolygon(c: Curve): CtrlGeometry {
  const center = mean(c.points);
  const radius = c.points.reduce((s, p) => s + dist(p, center), 0) / c.points.length;
  return {
    type: 'polygon',
    center: round2(center),
    radius: rnd(radius, 10),
    sides: Math.min(64, Math.max(3, c.points.length)),
    angle: rnd(angleOf(sub(c.points[0], center)), 10),
  };
}

/** Contrôleur suivant le redimensionnement de la plaque : positions à
 *  l'échelle (kx, ky) ; avec `all`, longueurs à l'échelle k. Sans arrondi,
 *  pour qu'un redimensionnement continu ne dérive pas. */
export function scaleGeometry(g: CtrlGeometry, kx: number, ky: number, k: number, all: boolean): CtrlGeometry {
  const sp = (p: Vec2) => ({ x: p.x * kx, y: p.y * ky });
  const sl = (v: number) => (all ? v * k : v);
  switch (g.type) {
    case 'point':
      return { ...g, at: sp(g.at) };
    case 'line':
      return { ...g, a: sp(g.a), b: sp(g.b) };
    case 'curve':
      return { ...g, points: g.points.map(sp), fillets: g.fillets?.map(sl) };
    case 'circle':
      return { ...g, center: sp(g.center), radius: sl(g.radius) };
    case 'ellipse':
      return { ...g, center: sp(g.center), rx: sl(g.rx), ry: sl(g.ry) };
    case 'polygon':
      return { ...g, center: sp(g.center), radius: sl(g.radius) };
    case 'rect':
      return { ...g, center: sp(g.center), width: sl(g.width), height: sl(g.height), corner: sl(g.corner) };
    case 'image': {
      // L'image suit la plaque par son centre et garde ses proportions.
      const c = sp({ x: g.at.x + g.width / 2, y: g.at.y + g.height / 2 });
      const width = sl(g.width);
      const height = sl(g.height);
      return { ...g, at: { x: c.x - width / 2, y: c.y - height / 2 }, width, height };
    }
  }
}
