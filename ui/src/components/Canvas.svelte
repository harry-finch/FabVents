<script lang="ts">
  // Canevas : plaque, perforations calculées par le moteur et éléments de
  // l'onglet affiché (contrôleurs du canal, ou tracés de la disposition
  // Chemin), avec leurs poignées. Repère de la plaque : Y vers le haut,
  // retourné à l'écran. Dessin en Canvas 2D, un chemin par famille
  // (cercles, polygones), redessiné à la trame suivante seulement.
  //
  // Souris : molette = zoom au curseur ; glisser le fond, bouton du milieu,
  // Espace + glisser ou outil Main = déplacer la vue ; glisser une poignée =
  // la déplacer (Maj : horizontal / vertical, ou par pas de 15° en traçant) ;
  // carré du contrôleur sélectionné = portée ; double-clic sur un tracé =
  // ajouter un nœud ; Alt + clic sur un nœud = le supprimer ; clic droit =
  // menu de l'élément.
  import { untrack } from 'svelte';
  import { Copy, Trash2, Circle, Hexagon, Spline, Plus, Minus, Lock, Unlock } from '@lucide/svelte';
  import ContextMenu, { type MenuEntry } from './ContextMenu.svelte';
  import { featureGeometry, sameKey as same, store, type Item, type ItemKey } from '../lib/store.svelte';
  import { theme } from '../lib/theme.svelte';
  import { t } from '../lib/i18n.svelte';
  import { fmtLen, numTrim, pct } from '../lib/format';
  import { add, angleOf, dist, distSeg, dragPart, handles, insideImage, outline, reachAnchor, round2, sub, distanceTo } from '../lib/geometry';
  import { rasterCanvas } from '../lib/image';
  import type { CtrlGeometry, Feature, FrameData, Tab, Vec2 } from '../lib/types';

  type Hit = { key: ItemKey; part: string };
  type Drag =
    | { kind: 'pan'; x: number; y: number; ox: number; oy: number }
    | { kind: 'item'; hit: Hit; start: Vec2; geom: CtrlGeometry }
    | { kind: 'line'; a: Vec2 }
    | { kind: 'ruler' };

  const HANDLE = 6;
  const HIT = 9;

  let host = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let size = $state({ w: 0, h: 0 });
  let hover = $state<Hit | null>(null);
  let ghost = $state<{ a: Vec2; b: Vec2 } | null>(null);
  /** Tracé en cours (outil Courbe) et position du curseur. */
  let draft = $state<Vec2[] | null>(null);
  let pointer = $state<Vec2 | null>(null);
  let ruler = $state<{ a: Vec2; b: Vec2 } | null>(null);
  let panning = $state(false);
  let menu = $state<{ x: number; y: number; entries: MenuEntry[] } | null>(null);
  let drag: Drag | null = null;
  let lastFit = -1;

  // ---- Repère

  const view = $derived(store.view);
  const toScreen = (p: Vec2) => ({ x: view.ox + p.x * view.scale, y: view.oy - p.y * view.scale });
  const toModel = (x: number, y: number): Vec2 => ({ x: (x - view.ox) / view.scale, y: (view.oy - y) / view.scale });

  // Taille du canevas (px CSS), partagée avec le store pour le zoom et le cadrage.
  $effect(() => {
    if (!host) return;
    const ro = new ResizeObserver(([e]) => {
      size = { w: e.contentRect.width, h: e.contentRect.height };
      store.viewport = size;
      if (!store.viewTouched) store.fit();
    });
    ro.observe(host);
    return () => ro.disconnect();
  });

  // Recadrage demandé (chargement, nouveau projet) ou premier affichage.
  $effect(() => {
    const seq = store.fitSeq;
    if (size.w > 0 && store.project && seq !== lastFit) {
      lastFit = seq;
      untrack(() => store.fit());
    }
  });

  // Changer d'outil abandonne le tracé en cours et la règle.
  $effect(() => {
    const tool = store.tool;
    untrack(() => {
      if (tool !== 'curve') draft = null;
      if (tool !== 'ruler') ruler = null;
    });
  });

  // ---- Couleurs (jetons du thème)

  let colors: Record<string, string> = {};
  const TOKENS = [
    'canvas-bg',
    'plate-fill',
    'plate-edge',
    'plate-margin',
    'hole-fill',
    'ctrl',
    'ctrl-edge',
    'ctrl-reach',
    'ctrl-reach-fill',
    'ctrl-active',
    'ctrl-active-fill',
    'canvas-ghost',
    'canvas-tile',
    'canvas-alert',
    'tooltip-bg',
    'tooltip-text',
    'font',
  ];
  function readColors() {
    const cs = getComputedStyle(document.documentElement);
    colors = Object.fromEntries(TOKENS.map((k) => [k, cs.getPropertyValue(`--${k}`).trim()]));
  }

  // ---- Dessin

  interface Scene {
    frame: FrameData | null;
    features: Feature[];
    items: Item[];
    tab: Tab;
    selection: ItemKey | null;
    node: number | null;
    hover: Hit | null;
    ghost: { a: Vec2; b: Vec2 } | null;
    draft: Vec2[] | null;
    pointer: Vec2 | null;
    ruler: { a: Vec2; b: Vec2 } | null;
    units: 'mm' | 'in';
    extent: Vec2;
  }

  let raf = 0;
  let pending: Scene | null = null;

  $effect(() => {
    void theme.applied;
    readColors();
    const scene: Scene = {
      frame: store.frame,
      features: $state.snapshot(store.p.features ?? []) as Feature[],
      items: $state.snapshot(store.items) as Item[],
      tab: store.tab,
      selection: $state.snapshot(store.selection),
      node: store.node,
      hover,
      ghost: ghost ? { ...ghost } : null,
      draft: draft ? draft.map((p) => ({ ...p })) : null,
      pointer: pointer ? { ...pointer } : null,
      ruler: ruler ? { a: { ...ruler.a }, b: { ...ruler.b } } : null,
      units: store.units,
      extent: store.plateExtent(),
    };
    void view.scale;
    void view.ox;
    void view.oy;
    void size.w;
    void size.h;
    pending = scene;
    if (!raf) raf = requestAnimationFrame(paint);
  });

  function paint() {
    raf = 0;
    const scene = pending;
    if (!scene || !canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const W = Math.max(1, Math.round(size.w * dpr));
    const H = Math.max(1, Math.round(size.h * dpr));
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    const ctx = canvas.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = colors['canvas-bg'];
    ctx.fillRect(0, 0, size.w, size.h);

    const f = scene.frame;
    const { scale: s, ox, oy } = view;
    const X = (x: number) => ox + x * s;
    const Y = (y: number) => oy - y * s;

    // Plaque.
    if (f && f.outline.length >= 6) {
      ctx.beginPath();
      ctx.moveTo(X(f.outline[0]), Y(f.outline[1]));
      for (let i = 2; i < f.outline.length; i += 2) ctx.lineTo(X(f.outline[i]), Y(f.outline[i + 1]));
      ctx.closePath();
      ctx.fillStyle = colors['plate-fill'];
      ctx.fill();
      ctx.lineWidth = 1;
      ctx.strokeStyle = colors['plate-edge'];
      ctx.stroke();
    }

    // Images des contrôleurs du canal, sous les perforations.
    for (const it of scene.items) {
      const g = it.geometry;
      if (g.type !== 'image') continue;
      const img = rasterCanvas(g.raster);
      if (!img) continue;
      const tl = toScreen({ x: g.at.x, y: g.at.y + g.height });
      ctx.globalAlpha = 0.45;
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(img, tl.x, tl.y, g.width * s, g.height * s);
      ctx.globalAlpha = 1;
    }

    // Zone perforable : plaque moins la marge, dans la zone perforée (pointillés).
    if (f) {
      ctx.beginPath();
      const zp = f.zonePoints;
      const zo = f.zoneOffsets;
      for (let k = 0; k + 1 < zo.length; k++) {
        if (zo[k + 1] - zo[k] < 2) continue;
        ctx.moveTo(X(zp[2 * zo[k]]), Y(zp[2 * zo[k] + 1]));
        for (let i = zo[k] + 1; i < zo[k + 1]; i++) ctx.lineTo(X(zp[2 * i]), Y(zp[2 * i + 1]));
        ctx.closePath();
      }
      ctx.setLineDash([4, 4]);
      ctx.strokeStyle = colors['plate-margin'];
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Zones réservées (hors onglet Plaque, où elles sont éditables).
    if (scene.tab !== 'plate') {
      for (const ft of scene.features) {
        if (ft.cut) continue;
        const o = outline(featureGeometry(ft.shape));
        if (!o) continue;
        tracePath(ctx, o.pts, true);
        ctx.setLineDash([2, 3]);
        ctx.lineWidth = 1;
        ctx.strokeStyle = colors['plate-margin'];
        ctx.stroke();
        ctx.setLineDash([]);
      }
    }

    // Perforations : le vide laisse voir le fond.
    if (f) {
      ctx.beginPath();
      const c = f.circles;
      for (let i = 0; i < c.length; i += 3) {
        const x = X(c[i]);
        const y = Y(c[i + 1]);
        const r = Math.max(c[i + 2] * s, 0.4);
        ctx.moveTo(x + r, y);
        ctx.arc(x, y, r, 0, Math.PI * 2);
      }
      const pts = f.polyPoints;
      const off = f.polyOffsets;
      for (let k = 0; k + 1 < off.length; k++) {
        const a = off[k];
        const b = off[k + 1];
        if (b - a < 2) continue;
        ctx.moveTo(X(pts[2 * a]), Y(pts[2 * a + 1]));
        for (let i = a + 1; i < b; i++) ctx.lineTo(X(pts[2 * i]), Y(pts[2 * i + 1]));
        ctx.closePath();
      }
      ctx.fillStyle = colors['hole-fill'];
      // Pair-impair : les îlots de matière entourés par une découpe restent pleins.
      ctx.fill('evenodd');
    }

    // Export STEP : perforations en cause (dépouille), puis coupes des dalles.
    if (f && f.alertOffsets.length > 1) {
      ctx.beginPath();
      const ap = f.alertPoints;
      const ao = f.alertOffsets;
      for (let k = 0; k + 1 < ao.length; k++) {
        if (ao[k + 1] - ao[k] < 2) continue;
        ctx.moveTo(X(ap[2 * ao[k]]), Y(ap[2 * ao[k] + 1]));
        for (let i = ao[k] + 1; i < ao[k + 1]; i++) ctx.lineTo(X(ap[2 * i]), Y(ap[2 * i + 1]));
        ctx.closePath();
      }
      ctx.lineWidth = 2;
      ctx.strokeStyle = colors['canvas-alert'];
      ctx.stroke();
      ctx.lineWidth = 1;
    }
    if (f && f.tileCuts.length > 1) {
      const nx = f.tileCuts[0];
      const ext = scene.extent;
      ctx.beginPath();
      for (let k = 1; k < f.tileCuts.length; k++) {
        const v = f.tileCuts[k];
        if (k <= nx) {
          ctx.moveTo(X(v), Y(0));
          ctx.lineTo(X(v), Y(ext.y));
        } else {
          ctx.moveTo(X(0), Y(v));
          ctx.lineTo(X(ext.x), Y(v));
        }
      }
      ctx.setLineDash([6, 4]);
      ctx.strokeStyle = colors['canvas-tile'];
      ctx.stroke();
      ctx.setLineDash([]);
    }

    for (const it of scene.items) drawItem(ctx, it, scene);

    // Ligne en cours de tracé.
    if (scene.ghost) strokeGhost(ctx, [scene.ghost.a, scene.ghost.b]);

    // Tracé en cours (outil Courbe).
    if (scene.draft?.length) {
      strokeGhost(ctx, scene.pointer ? [...scene.draft, scene.pointer] : scene.draft);
      for (const [i, p] of scene.draft.entries()) {
        const q = toScreen(p);
        ctx.beginPath();
        ctx.arc(q.x, q.y, i === 0 && scene.draft.length >= 3 ? HANDLE : HANDLE - 2, 0, Math.PI * 2);
        ctx.fillStyle = colors['ctrl'];
        ctx.fill();
        ctx.strokeStyle = colors['canvas-ghost'];
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
    }

    // Règle.
    if (scene.ruler) drawRuler(ctx, scene.ruler.a, scene.ruler.b, scene.units);
  }

  function strokeGhost(ctx: CanvasRenderingContext2D, pts: Vec2[]) {
    tracePath(ctx, pts, false);
    ctx.strokeStyle = colors['canvas-ghost'];
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  function tracePath(ctx: CanvasRenderingContext2D, pts: Vec2[], closed: boolean) {
    ctx.beginPath();
    pts.forEach((p, i) => {
      const q = toScreen(p);
      if (i === 0) ctx.moveTo(q.x, q.y);
      else ctx.lineTo(q.x, q.y);
    });
    if (closed) ctx.closePath();
  }

  function drawItem(ctx: CanvasRenderingContext2D, it: Item, scene: Scene) {
    const g = it.geometry;
    const inf = it.influence;
    const sel = same(it.key, scene.selection);
    const hov = same(it.key, scene.hover?.key ?? null);
    const s = view.scale;
    const o = outline(g);

    // Portée.
    if (inf && g.type !== 'image') {
      const fill = sel ? colors['ctrl-active-fill'] : colors['ctrl-reach-fill'];
      const stroke = sel ? colors['ctrl-active'] : colors['ctrl-reach'];
      const r = inf.reach * s;
      if (g.type === 'point' || g.type === 'line') {
        ctx.beginPath();
        if (g.type === 'point') {
          const p = toScreen(g.at);
          ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
        } else {
          capsulePath(ctx, toScreen(g.a), toScreen(g.b), r);
        }
        ctx.fillStyle = fill;
        ctx.fill();
        ctx.setLineDash(inf.hard_edge ? [] : [5, 4]);
        ctx.lineWidth = sel || hov ? 1.5 : 1;
        ctx.strokeStyle = stroke;
        ctx.stroke();
        ctx.setLineDash([]);
      } else if (o) {
        // Bande de portée autour du tracé ; intérieur plein si « remplir ».
        tracePath(ctx, o.pts, o.closed);
        ctx.lineJoin = 'round';
        ctx.lineCap = 'round';
        ctx.lineWidth = 2 * r;
        ctx.strokeStyle = fill;
        ctx.stroke();
        if (it.fill && o.closed) {
          ctx.fillStyle = fill;
          ctx.fill();
        }
        ctx.lineCap = 'butt';
      }
    }

    // Garde d'un élément de plaque.
    if (it.key.kind === 'feature' && o) {
      const id = it.key.id;
      const ft = scene.features.find((x) => x.id === id);
      if (ft && ft.clearance > 0) {
        tracePath(ctx, o.pts, true);
        ctx.lineJoin = 'round';
        ctx.lineWidth = 2 * ft.clearance * s;
        ctx.strokeStyle = sel ? colors['ctrl-active-fill'] : colors['ctrl-reach-fill'];
        ctx.stroke();
      }
    }

    // Tracé du contrôleur (ou du chemin, de l'élément de plaque).
    const main = sel ? colors['ctrl-active'] : colors['ctrl'];
    if (o) {
      tracePath(ctx, o.pts, o.closed);
      ctx.lineJoin = 'round';
      ctx.lineWidth = 4;
      ctx.strokeStyle = colors['ctrl-edge'];
      ctx.stroke();
      ctx.lineWidth = sel || hov ? 2 : 1.5;
      ctx.strokeStyle = main;
      if (g.type === 'image') ctx.setLineDash([6, 4]);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Poignées.
    for (const h of visibleHandles(g, sel)) {
      const p = toScreen(h.pos);
      const isNode = h.part.startsWith('n:');
      const active = sel && isNode && scene.node === Number(h.part.slice(2));
      ctx.beginPath();
      if (g.type === 'image') {
        ctx.rect(p.x - 4, p.y - 4, 8, 8);
      } else {
        const r = h.part === 'at' && hov && !sel ? HANDLE + 1 : h.part === 'at' || h.part === 'center' ? HANDLE : HANDLE - 1.5;
        ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      }
      ctx.fillStyle = active ? colors['ctrl-active'] : sel && !isNode ? main : colors['ctrl'];
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = sel && isNode ? colors['ctrl-active'] : colors['ctrl-edge'];
      ctx.stroke();
    }

    if (!sel || !inf) return;
    // Poignée de portée et valeur cible.
    const ra = reachAnchor(g, inf.reach);
    if (ra) {
      const r = toScreen(ra);
      ctx.fillStyle = colors['ctrl'];
      ctx.strokeStyle = colors['ctrl-active'];
      ctx.lineWidth = 1.5;
      ctx.fillRect(r.x - 4, r.y - 4, 8, 8);
      ctx.strokeRect(r.x - 4, r.y - 4, 8, 8);
    }
    const anchor = toScreen(handles(g)[0]?.pos ?? { x: 0, y: 0 });
    const label = scene.tab === 'angle' ? `${numTrim(inf.target, 1)}°` : scene.tab === 'morph' ? pct(inf.target * 100) : fmtLen(inf.target, scene.units, 2);
    tag(ctx, label, anchor.x + 10, anchor.y - 22);
  }

  function tag(ctx: CanvasRenderingContext2D, label: string, x: number, y: number) {
    ctx.font = `600 11px ${colors['font']}`;
    const tw = ctx.measureText(label).width;
    ctx.fillStyle = colors['tooltip-bg'];
    ctx.beginPath();
    ctx.roundRect(x, y, tw + 12, 18, 3);
    ctx.fill();
    ctx.fillStyle = colors['tooltip-text'];
    ctx.textBaseline = 'middle';
    ctx.fillText(label, x + 6, y + 9.5);
  }

  function drawRuler(ctx: CanvasRenderingContext2D, a: Vec2, b: Vec2, units: 'mm' | 'in') {
    const pa = toScreen(a);
    const pb = toScreen(b);
    ctx.beginPath();
    ctx.moveTo(pa.x, pa.y);
    ctx.lineTo(pb.x, pb.y);
    ctx.strokeStyle = colors['ctrl-edge'];
    ctx.lineWidth = 4;
    ctx.stroke();
    ctx.strokeStyle = colors['canvas-ghost'];
    ctx.lineWidth = 1.5;
    ctx.stroke();
    for (const p of [pa, pb]) {
      ctx.beginPath();
      ctx.arc(p.x, p.y, 3, 0, Math.PI * 2);
      ctx.fillStyle = colors['canvas-ghost'];
      ctx.fill();
    }
    const deg = ((angleOf(sub(b, a)) % 360) + 360) % 360;
    tag(ctx, `${fmtLen(dist(a, b), units, 2)} · ${numTrim(deg, 1)}°`, (pa.x + pb.x) / 2 + 8, (pa.y + pb.y) / 2 - 24);
  }

  /** Poignées affichées : nœuds et poignées secondaires seulement sur l'élément sélectionné. */
  function visibleHandles(g: CtrlGeometry, sel: boolean) {
    const hs = handles(g);
    if (sel) return hs;
    if (g.type === 'curve' || g.type === 'image') return [];
    if (g.type === 'circle' || g.type === 'ellipse' || g.type === 'polygon') return hs.filter((h) => h.part === 'center');
    return hs;
  }

  function capsulePath(ctx: CanvasRenderingContext2D, a: { x: number; y: number }, b: { x: number; y: number }, r: number) {
    const ang = Math.atan2(b.y - a.y, b.x - a.x);
    ctx.arc(b.x, b.y, r, ang - Math.PI / 2, ang + Math.PI / 2);
    ctx.arc(a.x, a.y, r, ang + Math.PI / 2, ang + (3 * Math.PI) / 2);
    ctx.closePath();
  }

  // ---- Interaction

  function local(e: PointerEvent | WheelEvent | MouseEvent) {
    const r = canvas!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function hitTest(sp: { x: number; y: number }): Hit | null {
    const items = store.items;
    const sel = items.find((i) => store.isSelected(i.key));
    if (sel?.influence) {
      const ra = reachAnchor(sel.geometry, sel.influence.reach);
      const r = ra && toScreen(ra);
      if (r && Math.abs(sp.x - r.x) <= 7 && Math.abs(sp.y - r.y) <= 7) return { key: sel.key, part: 'reach' };
    }
    const order = sel ? [sel, ...items.filter((i) => i !== sel).reverse()] : [...items].reverse();
    for (const it of order) {
      for (const h of visibleHandles(it.geometry, it === sel)) {
        const p = toScreen(h.pos);
        if (Math.hypot(sp.x - p.x, sp.y - p.y) <= HIT) return { key: it.key, part: h.part };
      }
    }
    const mp = toModel(sp.x, sp.y);
    for (const it of order) {
      const g = it.geometry;
      if (g.type === 'point') continue;
      if (g.type === 'image' ? insideImage(g, mp) : distanceTo(g, mp) * view.scale <= 6) return { key: it.key, part: 'body' };
    }
    return null;
  }

  function geometryOf(k: ItemKey): CtrlGeometry | null {
    return store.items.find((i) => same(i.key, k))?.geometry ?? null;
  }

  function startPan(sp: { x: number; y: number }) {
    drag = { kind: 'pan', x: sp.x, y: sp.y, ox: view.ox, oy: view.oy };
    panning = true;
  }

  const minNodes = (g: CtrlGeometry) => (g.type === 'curve' && g.closed ? 3 : 2);

  function removeNode(k: ItemKey, i: number) {
    const g = geometryOf(k);
    if (g?.type !== 'curve' || g.points.length <= minNodes(g)) return;
    store.setGeometry(k, {
      ...g,
      points: g.points.filter((_, j) => j !== i),
      fillets: g.fillets?.filter((_, j) => j !== i),
    });
    store.node = null;
  }

  /** Insère un nœud sur le segment du tracé le plus proche de `mp`. */
  function insertNode(k: ItemKey, mp: Vec2) {
    const g = geometryOf(k);
    if (g?.type !== 'curve') return;
    const n = g.points.length;
    const edges = g.closed ? n : n - 1;
    let best = 0;
    let bd = Infinity;
    for (let i = 0; i < edges; i++) {
      const d = distSeg(mp, g.points[i], g.points[(i + 1) % n]);
      if (d < bd) {
        bd = d;
        best = i;
      }
    }
    const points = [...g.points];
    points.splice(best + 1, 0, round2(mp));
    const fillets = g.fillets?.length ? [...g.fillets] : undefined;
    fillets?.splice(best + 1, 0, 0);
    store.setGeometry(k, { ...g, points, fillets });
    store.select(k, best + 1);
  }

  function finishDraft(closed: boolean) {
    const pts = draft ?? [];
    draft = null;
    // Un double-clic pose deux fois le même nœud : on ne garde que les nœuds distincts.
    const clean = pts.filter((p, i) => i === 0 || dist(p, pts[i - 1]) * view.scale > 2);
    if (clean.length >= (closed ? 3 : 2)) store.addItem({ type: 'curve', points: clean, closed });
    store.tool = 'select';
  }

  /** Point au curseur, aligné par pas de 15° depuis `from` si demandé. */
  function snap(from: Vec2 | undefined, p: Vec2, on: boolean): Vec2 {
    if (!on || !from) return round2(p);
    const ang = (Math.round(angleOf(sub(p, from)) / 15) * 15 * Math.PI) / 180;
    const l = dist(from, p);
    return round2(add(from, { x: Math.cos(ang) * l, y: Math.sin(ang) * l }));
  }

  function down(e: PointerEvent) {
    if (!store.project) return;
    menu = null;
    if (e.button === 2) return;
    const sp = local(e);
    const mp = toModel(sp.x, sp.y);
    canvas!.setPointerCapture(e.pointerId);
    canvas!.focus();
    if (e.button === 1 || store.spacePan || store.tool === 'hand') {
      e.preventDefault();
      startPan(sp);
      return;
    }
    if (e.button !== 0) return;
    if (store.tool === 'ruler') {
      ruler = { a: round2(mp), b: round2(mp) };
      drag = { kind: 'ruler' };
      return;
    }
    if (store.tool === 'curve') {
      if (draft && draft.length >= 3) {
        const first = toScreen(draft[0]);
        if (Math.hypot(sp.x - first.x, sp.y - first.y) <= HIT) {
          finishDraft(true);
          return;
        }
      }
      const p = snap(draft?.at(-1), mp, e.shiftKey);
      draft = draft ? [...draft, p] : [p];
      return;
    }
    if (!store.channel && (store.tool === 'line' || (store.tool === 'point' && store.tab !== 'plate'))) {
      store.toast(t(store.tab === 'plate' ? 'features.closedOnly' : 'canvas.pathOnlyCurves'));
      store.tool = 'curve';
      return;
    }
    if (store.tool === 'point') {
      store.addItem({ type: 'point', at: round2(mp) });
      store.tool = 'select';
      return;
    }
    if (store.tool === 'line') {
      ghost = { a: round2(mp), b: round2(mp) };
      drag = { kind: 'line', a: round2(mp) };
      return;
    }
    const hit = hitTest(sp);
    if (!hit) {
      store.select(null);
      startPan(sp);
      return;
    }
    const node = hit.part.startsWith('n:') ? Number(hit.part.slice(2)) : null;
    if (node != null && e.altKey) {
      removeNode(hit.key, node);
      return;
    }
    store.select(hit.key, node);
    const g = geometryOf(hit.key);
    if (!g) return;
    store.begin();
    drag = { kind: 'item', hit, start: mp, geom: $state.snapshot(g) as CtrlGeometry };
  }

  function move(e: PointerEvent) {
    if (!store.project) return;
    const sp = local(e);
    const mp = toModel(sp.x, sp.y);
    store.cursor = mp;
    if (store.tool === 'curve') pointer = snap(draft?.at(-1), mp, e.shiftKey);
    if (!drag) {
      hover = store.tool === 'select' && !store.spacePan ? hitTest(sp) : null;
      return;
    }
    if (drag.kind === 'pan') {
      store.pan(drag.ox + sp.x - drag.x, drag.oy + sp.y - drag.y);
    } else if (drag.kind === 'ruler' && ruler) {
      ruler = { a: ruler.a, b: snap(ruler.a, mp, e.shiftKey) };
    } else if (drag.kind === 'line') {
      ghost = { a: drag.a, b: snap(drag.a, mp, e.shiftKey) };
    } else if (drag.kind === 'item') {
      const { hit, start, geom } = drag;
      let d = { x: mp.x - start.x, y: mp.y - start.y };
      if (e.shiftKey && hit.part !== 'reach') d = Math.abs(d.x) > Math.abs(d.y) ? { x: d.x, y: 0 } : { x: 0, y: d.y };
      if (hit.part === 'reach') {
        const ch = store.channel;
        const id = hit.key.kind === 'ctrl' ? hit.key.id : -1;
        const reach = Math.max(0.5, Math.round(distanceTo(geom, mp) * 10) / 10);
        store.edit((p) => {
          const inf = ch ? p.controllers.find((c) => c.id === id)?.[ch] : undefined;
          if (inf) inf.reach = reach;
        });
      } else {
        store.setGeometry(hit.key, dragPart(geom, hit.part, d, mp));
      }
    }
  }

  function up(e: PointerEvent) {
    if (canvas!.hasPointerCapture(e.pointerId)) canvas!.releasePointerCapture(e.pointerId);
    const d = drag;
    drag = null;
    panning = false;
    if (!d) return;
    if (d.kind === 'item') store.end();
    if (d.kind === 'line' && ghost) {
      let { a, b } = ghost;
      if (dist(a, b) * view.scale < 6) {
        // Simple clic : ligne horizontale par défaut, centrée sur le clic.
        const ext = store.plateExtent();
        const half = Math.round(0.2 * Math.min(ext.x, ext.y));
        b = { x: a.x + half, y: a.y };
        a = { x: a.x - half, y: a.y };
      }
      ghost = null;
      store.addItem({ type: 'line', a, b });
      store.tool = 'select';
    }
  }

  function dblclick(e: MouseEvent) {
    if (store.tool === 'curve') {
      finishDraft(false);
      return;
    }
    if (store.tool !== 'select') return;
    const sp = local(e);
    const hit = hitTest(sp);
    if (hit && hit.part === 'body' && geometryOf(hit.key)?.type === 'curve') insertNode(hit.key, toModel(sp.x, sp.y));
  }

  function contextmenu(e: MouseEvent) {
    e.preventDefault();
    if (store.tool !== 'select') return;
    const sp = local(e);
    const hit = hitTest(sp);
    if (!hit) {
      menu = null;
      return;
    }
    const node = hit.part.startsWith('n:') ? Number(hit.part.slice(2)) : null;
    store.select(hit.key, node);
    const g = geometryOf(hit.key)!;
    const mp = toModel(sp.x, sp.y);
    const entries: MenuEntry[] = [];
    if (g.type === 'curve') {
      if (node != null) entries.push({ label: t('ctrl.removeNode'), icon: Minus, action: () => removeNode(hit.key, node), disabled: g.points.length <= minNodes(g) });
      else entries.push({ label: t('ctrl.addNode'), icon: Plus, action: () => insertNode(hit.key, mp) });
      entries.push({
        label: g.closed ? t('ctrl.openCurve') : t('ctrl.closeCurve'),
        icon: g.closed ? Unlock : Lock,
        action: () => store.setGeometry(hit.key, { ...g, closed: !g.closed }),
        disabled: !g.closed && g.points.length < 3,
      });
      if (g.closed && hit.key.kind === 'ctrl') {
        entries.push({ label: t('ctrl.toCircle'), icon: Circle, action: () => store.convertSelected('circle'), separator: true });
        entries.push({ label: t('ctrl.toEllipse'), icon: Spline, action: () => store.convertSelected('ellipse') });
        entries.push({ label: t('ctrl.toPolygon'), icon: Hexagon, action: () => store.convertSelected('polygon') });
      }
    }
    entries.push({ label: t('ctrl.duplicate'), icon: Copy, action: () => store.duplicateSelected(), separator: entries.length > 0 });
    entries.push({ label: t('ctrl.delete'), icon: Trash2, action: () => store.removeSelected() });
    menu = { x: e.clientX, y: e.clientY, entries };
  }

  // Outil Courbe : Entrée termine le tracé ouvert, Retour arrière retire le dernier nœud.
  function keydown(e: KeyboardEvent) {
    if (!draft) return;
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) return;
    if (e.key === 'Enter') {
      e.preventDefault();
      finishDraft(false);
    } else if (e.key === 'Backspace') {
      e.preventDefault();
      draft = draft.length > 1 ? draft.slice(0, -1) : null;
    }
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    menu = null;
    const sp = local(e);
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
    store.zoomBy(Math.exp(-dy * 0.0015), sp.x, sp.y);
  }

  const cursor = $derived(
    panning
      ? 'grabbing'
      : store.spacePan || store.tool === 'hand'
        ? 'grab'
        : store.tool !== 'select'
          ? 'crosshair'
          : hover
            ? hover.part === 'reach'
              ? 'ew-resize'
              : 'move'
            : 'default',
  );

  const errors = $derived(store.frame?.report.issues.filter((i) => i.severity === 'error') ?? []);
  const empty = $derived(store.project != null && store.items.length === 0);
  const hint = $derived.by(() => {
    switch (store.tool) {
      case 'point':
        return t('canvas.placePoint');
      case 'line':
        return t('canvas.placeLine');
      case 'curve':
        return t('canvas.placeCurve');
      case 'ruler':
        return t('canvas.useRuler');
      default:
        return empty ? t(`canvas.empty.${store.tab}`) : '';
    }
  });
</script>

<svelte:window onkeydown={keydown} />

<div class="host" bind:this={host}>
  <canvas
    bind:this={canvas}
    tabindex="0"
    aria-label={t('canvas.label')}
    style:cursor
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onpointerleave={() => {
      store.cursor = null;
      pointer = null;
      if (!drag) hover = null;
    }}
    ondblclick={dblclick}
    onwheel={wheel}
    oncontextmenu={contextmenu}
  ></canvas>
  {#if errors.length}
    <div class="banner" role="status">{t('canvas.notComputed')}</div>
  {/if}
  {#if hint}
    <div class="hint">{hint}</div>
  {/if}
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} entries={menu.entries} onclose={() => (menu = null)} />
{/if}

<style>
  .host {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--canvas-bg);
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    outline: none;
    touch-action: none;
  }

  .banner {
    position: absolute;
    top: var(--s3);
    left: 50%;
    transform: translateX(-50%);
    padding: 5px 10px;
    border-radius: var(--radius);
    background: var(--danger-tint);
    border: 1px solid var(--danger);
    color: var(--danger-text);
    font-size: var(--fs-sm);
    pointer-events: none;
  }

  .hint {
    position: absolute;
    left: var(--s3);
    bottom: var(--s3);
    max-width: 60%;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    pointer-events: none;
  }
</style>
