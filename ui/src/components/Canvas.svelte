<script lang="ts">
  // Canevas : plaque, perforations calculées par le moteur et contrôleurs du
  // canal affiché, avec leurs poignées. Repère de la plaque : Y vers le haut,
  // retourné à l'écran. Dessin en Canvas 2D, regroupé en un chemin par
  // famille (cercles, polygones) ; redessiné à la trame suivante seulement.
  //
  // Souris : molette = zoom centré sur le curseur ; glisser le fond, bouton
  // du milieu, Espace + glisser ou outil Main = déplacer la vue ; glisser une
  // poignée = déplacer le contrôleur (Maj : horizontal ou vertical) ; carré
  // du contrôleur sélectionné = portée.
  import { untrack } from 'svelte';
  import { store } from '../lib/store.svelte';
  import { theme } from '../lib/theme.svelte';
  import { t } from '../lib/i18n.svelte';
  import { fmtLen, numTrim } from '../lib/format';
  import type { Controller, CtrlGeometry, FrameData, Plate, Vec2 } from '../lib/types';

  type Part = 'at' | 'a' | 'b' | 'body' | 'reach';
  type Hit = { id: number; part: Part };
  type Drag =
    | { kind: 'pan'; x: number; y: number; ox: number; oy: number }
    | { kind: 'ctrl'; hit: Hit; start: Vec2; geom: CtrlGeometry }
    | { kind: 'line'; a: Vec2 };

  const HANDLE = 6;
  const HIT = 9;

  let host = $state<HTMLDivElement>();
  let canvas = $state<HTMLCanvasElement>();
  let size = $state({ w: 0, h: 0 });
  let hover = $state<Hit | null>(null);
  let ghost = $state<{ a: Vec2; b: Vec2 } | null>(null);
  let panning = $state(false);
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
    'text-secondary',
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
    plate: Plate;
    ctrls: Controller[];
    channel: 'size' | 'angle';
    selected: number | null;
    hover: Hit | null;
    ghost: { a: Vec2; b: Vec2 } | null;
    units: 'mm' | 'in';
  }

  let raf = 0;
  let pending: Scene | null = null;

  $effect(() => {
    void theme.applied;
    readColors();
    const scene: Scene = {
      frame: store.frame,
      plate: $state.snapshot(store.p.plate),
      ctrls: $state.snapshot(store.channelControllers) as Controller[],
      channel: store.channel,
      selected: store.selected,
      hover,
      ghost: ghost ? { ...ghost } : null,
      units: store.units,
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

    // Zone de marge (pointillés).
    marginPath(ctx, scene.plate, X, Y, s);
    ctx.setLineDash([4, 4]);
    ctx.strokeStyle = colors['plate-margin'];
    ctx.stroke();
    ctx.setLineDash([]);

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
      ctx.fill();
    }

    // Contrôleurs du canal affiché.
    for (const c of scene.ctrls) drawController(ctx, c, scene);

    // Ligne en cours de tracé.
    if (scene.ghost) {
      const a = toScreen(scene.ghost.a);
      const b = toScreen(scene.ghost.b);
      ctx.strokeStyle = colors['canvas-ghost'];
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
  }

  function marginPath(ctx: CanvasRenderingContext2D, pl: Plate, X: (x: number) => number, Y: (y: number) => number, s: number) {
    ctx.beginPath();
    const m = pl.margin;
    if (pl.outline === 'circle') {
      const r = pl.width / 2 - m;
      if (r > 0) ctx.arc(X(pl.width / 2), Y(pl.width / 2), r * s, 0, Math.PI * 2);
      return;
    }
    const w = pl.width - 2 * m;
    const h = pl.height - 2 * m;
    if (w <= 0 || h <= 0) return;
    const r = Math.max(0, Math.min(pl.corner_radius, pl.width / 2, pl.height / 2) - m);
    ctx.roundRect(X(m), Y(pl.height - m), w * s, h * s, Math.min(r, w / 2, h / 2) * s);
  }

  function drawController(ctx: CanvasRenderingContext2D, c: Controller, scene: Scene) {
    const inf = c[scene.channel];
    if (!inf) return;
    const sel = c.id === scene.selected;
    const hov = scene.hover?.id === c.id;
    const s = view.scale;
    const reachPx = inf.reach * s;
    const stroke = sel ? colors['ctrl-active'] : colors['ctrl-reach'];

    // Portée.
    ctx.beginPath();
    if (c.geometry.type === 'point') {
      const p = toScreen(c.geometry.at);
      ctx.arc(p.x, p.y, reachPx, 0, Math.PI * 2);
    } else {
      capsulePath(ctx, toScreen(c.geometry.a), toScreen(c.geometry.b), reachPx);
    }
    ctx.fillStyle = sel ? colors['ctrl-active-fill'] : colors['ctrl-reach-fill'];
    ctx.fill();
    ctx.setLineDash(inf.hard_edge ? [] : [5, 4]);
    ctx.lineWidth = sel || hov ? 1.5 : 1;
    ctx.strokeStyle = stroke;
    ctx.stroke();
    ctx.setLineDash([]);

    // Géométrie et poignées.
    const fill = sel ? colors['ctrl-active'] : colors['ctrl'];
    const handle = (p: { x: number; y: number }, r: number) => {
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.fillStyle = fill;
      ctx.fill();
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = colors['ctrl-edge'];
      ctx.stroke();
    };
    let anchor: { x: number; y: number };
    if (c.geometry.type === 'point') {
      anchor = toScreen(c.geometry.at);
      handle(anchor, hov && !sel ? HANDLE + 1 : HANDLE);
    } else {
      const a = toScreen(c.geometry.a);
      const b = toScreen(c.geometry.b);
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.lineWidth = 4;
      ctx.strokeStyle = colors['ctrl-edge'];
      ctx.stroke();
      ctx.lineWidth = 2;
      ctx.strokeStyle = fill;
      ctx.stroke();
      handle(a, HANDLE - 1);
      handle(b, HANDLE - 1);
      anchor = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    }

    if (sel) {
      // Poignée de portée et valeur cible.
      const r = reachHandle(c, scene.channel);
      if (r) {
        ctx.fillStyle = colors['ctrl'];
        ctx.strokeStyle = colors['ctrl-active'];
        ctx.lineWidth = 1.5;
        ctx.fillRect(r.x - 4, r.y - 4, 8, 8);
        ctx.strokeRect(r.x - 4, r.y - 4, 8, 8);
      }
      const label = scene.channel === 'size' ? fmtLen(inf.target, scene.units, 2) : `${numTrim(inf.target, 1)}°`;
      ctx.font = `600 11px ${colors['font']}`;
      const tw = ctx.measureText(label).width;
      const lx = anchor.x + 10;
      const ly = anchor.y - 22;
      ctx.fillStyle = colors['tooltip-bg'];
      ctx.beginPath();
      ctx.roundRect(lx, ly, tw + 12, 18, 3);
      ctx.fill();
      ctx.fillStyle = colors['tooltip-text'];
      ctx.textBaseline = 'middle';
      ctx.fillText(label, lx + 6, ly + 9.5);
    }
  }

  function capsulePath(ctx: CanvasRenderingContext2D, a: { x: number; y: number }, b: { x: number; y: number }, r: number) {
    const ang = Math.atan2(b.y - a.y, b.x - a.x);
    ctx.arc(b.x, b.y, r, ang - Math.PI / 2, ang + Math.PI / 2);
    ctx.arc(a.x, a.y, r, ang + Math.PI / 2, ang + (3 * Math.PI) / 2);
    ctx.closePath();
  }

  /** Position à l'écran de la poignée de portée d'un contrôleur. */
  function reachHandle(c: Controller, ch: 'size' | 'angle') {
    const inf = c[ch];
    if (!inf) return null;
    if (c.geometry.type === 'point') return toScreen({ x: c.geometry.at.x + inf.reach, y: c.geometry.at.y });
    const { a, b } = c.geometry;
    const d = { x: b.x - a.x, y: b.y - a.y };
    const l = Math.hypot(d.x, d.y) || 1;
    const n = { x: -d.y / l, y: d.x / l };
    return toScreen({ x: (a.x + b.x) / 2 + n.x * inf.reach, y: (a.y + b.y) / 2 + n.y * inf.reach });
  }

  // ---- Interaction

  function local(e: PointerEvent | WheelEvent) {
    const r = canvas!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function distSeg(p: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const l2 = dx * dx + dy * dy;
    const k = l2 > 0 ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2)) : 0;
    return Math.hypot(p.x - (a.x + dx * k), p.y - (a.y + dy * k));
  }

  function hitTest(sp: { x: number; y: number }): Hit | null {
    const ctrls = store.channelControllers;
    const sel = ctrls.find((c) => c.id === store.selected);
    if (sel) {
      const r = reachHandle(sel, store.channel);
      if (r && Math.abs(sp.x - r.x) <= 7 && Math.abs(sp.y - r.y) <= 7) return { id: sel.id, part: 'reach' };
    }
    for (let i = ctrls.length - 1; i >= 0; i--) {
      const c = ctrls[i];
      const g = c.geometry;
      if (g.type === 'point') {
        const p = toScreen(g.at);
        if (Math.hypot(sp.x - p.x, sp.y - p.y) <= HIT) return { id: c.id, part: 'at' };
      } else {
        const a = toScreen(g.a);
        const b = toScreen(g.b);
        if (Math.hypot(sp.x - a.x, sp.y - a.y) <= HIT) return { id: c.id, part: 'a' };
        if (Math.hypot(sp.x - b.x, sp.y - b.y) <= HIT) return { id: c.id, part: 'b' };
        if (distSeg(sp, a, b) <= 6) return { id: c.id, part: 'body' };
      }
    }
    return null;
  }

  function startPan(sp: { x: number; y: number }) {
    drag = { kind: 'pan', x: sp.x, y: sp.y, ox: view.ox, oy: view.oy };
    panning = true;
  }

  function down(e: PointerEvent) {
    if (!store.project) return;
    canvas!.setPointerCapture(e.pointerId);
    canvas!.focus();
    const sp = local(e);
    const mp = toModel(sp.x, sp.y);
    if (e.button === 1 || store.spacePan || store.tool === 'hand') {
      e.preventDefault();
      startPan(sp);
      return;
    }
    if (e.button !== 0) return;
    if (store.tool === 'point') {
      store.addController({ type: 'point', at: round(mp) });
      store.tool = 'select';
      return;
    }
    if (store.tool === 'line') {
      ghost = { a: round(mp), b: round(mp) };
      drag = { kind: 'line', a: round(mp) };
      return;
    }
    const hit = hitTest(sp);
    if (hit) {
      store.selected = hit.id;
      const c = store.p.controllers.find((x) => x.id === hit.id)!;
      store.begin();
      drag = { kind: 'ctrl', hit, start: mp, geom: $state.snapshot(c.geometry) as CtrlGeometry };
    } else {
      store.selected = null;
      startPan(sp);
    }
  }

  const round = (p: Vec2): Vec2 => ({ x: Math.round(p.x * 100) / 100, y: Math.round(p.y * 100) / 100 });
  const add = (p: Vec2, d: Vec2): Vec2 => round({ x: p.x + d.x, y: p.y + d.y });

  function move(e: PointerEvent) {
    if (!store.project) return;
    const sp = local(e);
    const mp = toModel(sp.x, sp.y);
    store.cursor = mp;
    if (!drag) {
      hover = store.tool === 'select' && !store.spacePan ? hitTest(sp) : null;
      return;
    }
    if (drag.kind === 'pan') {
      store.pan(drag.ox + sp.x - drag.x, drag.oy + sp.y - drag.y);
    } else if (drag.kind === 'line') {
      ghost = { a: drag.a, b: e.shiftKey ? snapAngle(drag.a, mp) : round(mp) };
    } else {
      const { hit, start, geom } = drag;
      let d = { x: mp.x - start.x, y: mp.y - start.y };
      if (e.shiftKey && hit.part !== 'reach') d = Math.abs(d.x) > Math.abs(d.y) ? { x: d.x, y: 0 } : { x: 0, y: d.y };
      const ch = store.channel;
      store.edit((p) => {
        const c = p.controllers.find((x) => x.id === hit.id);
        if (!c) return;
        if (hit.part === 'reach') {
          const inf = c[ch];
          const dist = geom.type === 'point' ? Math.hypot(mp.x - geom.at.x, mp.y - geom.at.y) : segDist(mp, geom.a, geom.b);
          if (inf) inf.reach = Math.max(0.5, Math.round(dist * 10) / 10);
        } else if (geom.type === 'point') {
          c.geometry = { type: 'point', at: add(geom.at, d) };
        } else {
          c.geometry = {
            type: 'line',
            a: hit.part === 'b' ? geom.a : add(geom.a, d),
            b: hit.part === 'a' ? geom.b : add(geom.b, d),
          };
        }
      });
    }
  }

  function segDist(p: Vec2, a: Vec2, b: Vec2) {
    return distSeg(p, a, b);
  }

  function snapAngle(a: Vec2, p: Vec2): Vec2 {
    const step = Math.PI / 12;
    const ang = Math.round(Math.atan2(p.y - a.y, p.x - a.x) / step) * step;
    const l = Math.hypot(p.x - a.x, p.y - a.y);
    return round({ x: a.x + Math.cos(ang) * l, y: a.y + Math.sin(ang) * l });
  }

  function up(e: PointerEvent) {
    if (canvas!.hasPointerCapture(e.pointerId)) canvas!.releasePointerCapture(e.pointerId);
    const d = drag;
    drag = null;
    panning = false;
    if (!d) return;
    if (d.kind === 'ctrl') store.end();
    if (d.kind === 'line' && ghost) {
      let { a, b } = ghost;
      const px = Math.hypot(b.x - a.x, b.y - a.y) * view.scale;
      if (px < 6) {
        // Simple clic : ligne horizontale par défaut, centrée sur le clic.
        const ext = store.plateExtent();
        const half = Math.round(0.2 * Math.min(ext.x, ext.y));
        b = { x: a.x + half, y: a.y };
        a = { x: a.x - half, y: a.y };
      }
      ghost = null;
      store.addController({ type: 'line', a, b });
      store.tool = 'select';
    }
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const sp = local(e);
    const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
    store.zoomBy(Math.exp(-dy * 0.0015), sp.x, sp.y);
  }

  const cursor = $derived(
    panning
      ? 'grabbing'
      : store.spacePan || store.tool === 'hand'
        ? 'grab'
        : store.tool === 'point' || store.tool === 'line'
          ? 'crosshair'
          : hover
            ? hover.part === 'reach'
              ? 'ew-resize'
              : 'move'
            : 'default',
  );

  const errors = $derived(store.frame?.report.issues.filter((i) => i.severity === 'error') ?? []);
  const emptyChannel = $derived(store.project != null && store.channelControllers.length === 0);
</script>

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
      if (!drag) hover = null;
    }}
    onwheel={wheel}
    oncontextmenu={(e) => e.preventDefault()}
  ></canvas>
  {#if errors.length}
    <div class="banner" role="status">{t('canvas.notComputed')}</div>
  {/if}
  {#if emptyChannel && store.tool === 'select'}
    <div class="hint">{t(`canvas.empty.${store.channel}`)}</div>
  {:else if store.tool === 'point'}
    <div class="hint">{t('canvas.placePoint')}</div>
  {:else if store.tool === 'line'}
    <div class="hint">{t('canvas.placeLine')}</div>
  {/if}
</div>

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
