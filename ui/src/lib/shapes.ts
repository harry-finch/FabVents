// Formes dessinées (atelier de formes) : calques des primitives cotées et
// conversion d'une forme simple en forme dessinée. Repère de la forme :
// 1 = la taille de la forme, centré sur le pivot, Y vers le haut.

import type { CustomShape, ShapeKind, ShapeLayer, ShapeNode, Vec2 } from './types';

export type Primitive = 'rectangle' | 'disc' | 'polygon' | 'star' | 'slot';
export const PRIMITIVES: Primitive[] = ['rectangle', 'disc', 'polygon', 'star', 'slot'];

const node = (x: number, y: number, fillet = 0): ShapeNode => (fillet > 0 ? { at: { x, y }, fillet } : { at: { x, y } });

const layerOf = (nodes: ShapeNode[]): ShapeLayer => ({ nodes, role: 'union', response: 1 });

/** Rectangle `w × h` centré, coins arrondis de `corner`. */
export function rectangleLayer(w: number, h: number, corner = 0): ShapeLayer {
  const c = Math.min(corner, w / 2, h / 2);
  return layerOf([node(-w / 2, -h / 2, c), node(w / 2, -h / 2, c), node(w / 2, h / 2, c), node(-w / 2, h / 2, c)]);
}

/** Disque exact : carré dont les congés valent la moitié du côté. */
export const discLayer = (d: number) => rectangleLayer(d, d, d / 2);

/** Polygone régulier inscrit dans le cercle de diamètre `d`, base à plat. */
export function polygonLayer(sides: number, d: number): ShapeLayer {
  const start = -Math.PI / 2 + Math.PI / sides;
  return layerOf(
    Array.from({ length: sides }, (_, i) => {
      const a = start + (2 * Math.PI * i) / sides;
      return node((d / 2) * Math.cos(a), (d / 2) * Math.sin(a));
    }),
  );
}

/** Étoile à `points` branches, rayon intérieur `inner` × rayon extérieur. */
export function starLayer(points: number, inner: number, d: number): ShapeLayer {
  return layerOf(
    Array.from({ length: 2 * points }, (_, i) => {
      const a = Math.PI / 2 + (Math.PI * i) / points;
      const r = (d / 2) * (i % 2 === 0 ? 1 : inner);
      return node(r * Math.cos(a), r * Math.sin(a));
    }),
  );
}

/** Fente : longueur 1, largeur `aspect`, bouts ronds. */
export const slotLayer = (aspect: number) => rectangleLayer(1, aspect, aspect / 2);

export function primitiveLayer(p: Primitive): ShapeLayer {
  switch (p) {
    case 'rectangle':
      return rectangleLayer(0.8, 0.5, 0.05);
    case 'disc':
      return discLayer(0.5);
    case 'polygon':
      return polygonLayer(6, 0.6);
    case 'star':
      return starLayer(5, 0.45, 0.8);
    case 'slot':
      return slotLayer(0.3);
  }
}

/** Forme dessinée équivalente à une forme simple (même aspect à la taille 1). */
export function toCustom(kind: ShapeKind, size: number): CustomShape {
  const k = size > 0 ? 1 / size : 1;
  const layer = (() => {
    switch (kind.type) {
      case 'custom':
        return null;
      case 'circle':
        return discLayer(1);
      case 'rectangle':
        return rectangleLayer(1, kind.aspect, kind.corner * k);
      case 'polygon':
        return polygonLayer(kind.sides, 1);
      case 'star':
        return starLayer(kind.points, kind.inner, 1);
      case 'slot':
        return slotLayer(kind.aspect);
    }
  })();
  if (kind.type === 'custom') return structuredClone({ layers: kind.layers, pivot: kind.pivot });
  return { layers: [layer!], pivot: { x: 0, y: 0 } };
}

/** Segment le plus proche de `p` dans un calque (indice du nœud de départ). */
export function nearestSegment(layer: ShapeLayer, p: Vec2): { index: number; at: Vec2; dist: number } {
  let best = { index: 0, at: p, dist: Infinity };
  const n = layer.nodes.length;
  for (let i = 0; i < n; i++) {
    const a = layer.nodes[i].at;
    const b = layer.nodes[(i + 1) % n].at;
    const ex = b.x - a.x;
    const ey = b.y - a.y;
    const len2 = ex * ex + ey * ey;
    const t = len2 > 0 ? Math.max(0, Math.min(1, ((p.x - a.x) * ex + (p.y - a.y) * ey) / len2)) : 0;
    const q = { x: a.x + t * ex, y: a.y + t * ey };
    const d = Math.hypot(p.x - q.x, p.y - q.y);
    if (d < best.dist) best = { index: i, at: q, dist: d };
  }
  return best;
}

/** Poignées symétriques d'un nœud lissé, le long de la direction voisin → voisin. */
export function smoothHandles(layer: ShapeLayer, i: number): { handle_in: Vec2; handle_out: Vec2 } {
  const n = layer.nodes.length;
  const prev = layer.nodes[(i + n - 1) % n].at;
  const next = layer.nodes[(i + 1) % n].at;
  const dx = next.x - prev.x;
  const dy = next.y - prev.y;
  const k = 1 / 6;
  return { handle_in: { x: -dx * k, y: -dy * k }, handle_out: { x: dx * k, y: dy * k } };
}
