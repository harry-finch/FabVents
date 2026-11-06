<script lang="ts">
  // Atelier de formes : dessin d'une forme de perforation par calques.
  // - Vue : forme obtenue (calques combinés, symétries comprises) en vide sur
  //   la matière, contour de chaque calque, nœuds et poignées du calque choisi,
  //   pivot. Glisser un nœud, une poignée ou le pivot (Maj : pas de 0,05),
  //   double-clic sur un bord pour ajouter un nœud, Suppr pour retirer le nœud.
  // - Panneau : calques (ajout de primitives cotées, import SVG / DXF, ordre,
  //   rôle union / soustraction, symétrie, réponse au canal Taille), nœud
  //   choisi (position, congé, courbe de Bézier).
  // Les coordonnées sont en unités de taille (1 = la taille de la forme) et
  // affichées en mm à la taille de base. Chaque geste est une étape d'annulation.
  import { ArrowDown, ArrowUp, Copy, Trash2, Upload, X as Close, Plus, Minus } from '@lucide/svelte';
  import NumField from './NumField.svelte';
  import Field from './Field.svelte';
  import Segmented from './Segmented.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { nearestSegment, PRIMITIVES, primitiveLayer, smoothHandles, type Primitive } from '../lib/shapes';
  import type { CustomShape, LayerRole, ShapePreview, ShapeLayer, Vec2 } from '../lib/types';

  const target = $derived(store.workshop ?? 'kind');
  const shape = $derived.by((): CustomShape | null => {
    const k = target === 'kind' ? store.p.shape.kind : store.p.shape.morph;
    return k && k.type === 'custom' ? k : null;
  });
  const size = $derived(store.p.shape.size);

  let dialog = $state<HTMLDialogElement>();
  $effect(() => dialog?.showModal());

  let layerIndex = $state(0);
  let nodeIndex = $state<number | null>(null);
  const layer = $derived(shape?.layers[Math.min(layerIndex, (shape?.layers.length ?? 1) - 1)] ?? null);
  const node = $derived(layer && nodeIndex != null ? (layer.nodes[nodeIndex] ?? null) : null);

  /** Modifie la forme éditée (étape d'annulation, fusionnée sur `key`). */
  function edit(fn: (c: CustomShape) => void, key = '') {
    store.edit((q) => {
      const k = target === 'kind' ? q.shape.kind : q.shape.morph;
      if (k && k.type === 'custom') fn(k);
    }, key ? `workshop.${key}` : '');
  }
  const editLayer = (fn: (l: ShapeLayer) => void, key = '') =>
    edit((c) => {
      const l = c.layers[layerIndex];
      if (l) fn(l);
    }, key);

  // ---- Aperçu calculé par le moteur (le dernier demandé gagne).
  let preview = $state<ShapePreview | null>(null);
  let asked = 0;
  $effect(() => {
    if (!shape) return;
    const json = JSON.stringify({ kind: { type: 'custom', ...$state.snapshot(shape) }, size, rotation: 0 });
    const n = ++asked;
    void store.engine.shapePreview(json).then(
      (r) => {
        if (n === asked) preview = JSON.parse(r) as ShapePreview;
      },
      () => {
        if (n === asked) preview = null;
      },
    );
  });

  // ---- Vue : repère de la forme → pixels (Y vers le haut).
  const W = 460;
  const SPAN = 1.6;
  const X = (x: number) => ((x + SPAN / 2) / SPAN) * W;
  const Y = (y: number) => ((SPAN / 2 - y) / SPAN) * W;
  const toShape = (px: number, py: number): Vec2 => ({ x: (px / W) * SPAN - SPAN / 2, y: SPAN / 2 - (py / W) * SPAN });
  const path = (pts: [number, number][], dx = 0, dy = 0) =>
    pts.length ? 'M' + pts.map(([x, y]) => `${X(x + dx).toFixed(2)},${Y(y + dy).toFixed(2)}`).join('L') + 'Z' : '';
  const pivot = $derived(shape?.pivot ?? { x: 0, y: 0 });
  const grid = Array.from({ length: 17 }, (_, i) => -0.8 + i * 0.1);

  let svg = $state<SVGSVGElement>();
  type Drag = { kind: 'node' | 'in' | 'out' | 'pivot'; index: number };
  let drag: Drag | null = null;

  function pointer(e: PointerEvent): Vec2 {
    const r = svg!.getBoundingClientRect();
    const p = toShape(((e.clientX - r.left) / r.width) * W, ((e.clientY - r.top) / r.height) * W);
    const step = e.shiftKey ? 0.05 : 0.005;
    return { x: Math.round(p.x / step) * step, y: Math.round(p.y / step) * step };
  }

  function start(e: PointerEvent, d: Drag) {
    e.stopPropagation();
    e.preventDefault();
    if (d.kind === 'node') nodeIndex = d.index;
    drag = d;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    store.begin();
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    const p = pointer(e);
    const d = drag;
    if (d.kind === 'pivot') {
      edit((c) => (c.pivot = p));
      return;
    }
    editLayer((l) => {
      const n = l.nodes[d.index];
      if (!n) return;
      if (d.kind === 'node') n.at = p;
      else if (d.kind === 'in') n.handle_in = { x: p.x - n.at.x, y: p.y - n.at.y };
      else n.handle_out = { x: p.x - n.at.x, y: p.y - n.at.y };
    });
  }

  function stop() {
    if (!drag) return;
    drag = null;
    store.end();
  }

  /** Double-clic sur un bord du calque choisi : nouveau nœud. */
  function insert(e: MouseEvent) {
    if (!layer) return;
    const p = pointer(e as PointerEvent);
    const s = nearestSegment(layer, p);
    if (s.dist > 0.05) return;
    editLayer((l) => l.nodes.splice(s.index + 1, 0, { at: s.at }));
    nodeIndex = s.index + 1;
  }

  function removeNode() {
    if (!layer || nodeIndex == null || layer.nodes.length <= 3) return;
    const i = nodeIndex;
    editLayer((l) => l.nodes.splice(i, 1));
    nodeIndex = null;
  }

  function key(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (el.tagName === 'INPUT') return;
    if ((e.key === 'Delete' || e.key === 'Backspace') && nodeIndex != null) {
      e.preventDefault();
      removeNode();
    }
  }

  // ---- Calques.
  function addLayer(p: Primitive) {
    edit((c) => c.layers.push(primitiveLayer(p)));
    layerIndex = (shape?.layers.length ?? 1) - 1;
    nodeIndex = null;
  }

  function moveLayer(d: -1 | 1) {
    const i = layerIndex;
    const j = i + d;
    if (!shape || j < 0 || j >= shape.layers.length) return;
    edit((c) => ([c.layers[i], c.layers[j]] = [c.layers[j], c.layers[i]]));
    layerIndex = j;
  }

  function duplicateLayer() {
    const i = layerIndex;
    edit((c) => c.layers.splice(i + 1, 0, structuredClone($state.snapshot(c.layers[i]))));
    layerIndex = i + 1;
  }

  function deleteLayer() {
    if (!shape || shape.layers.length <= 1) return;
    const i = layerIndex;
    edit((c) => c.layers.splice(i, 1));
    layerIndex = Math.max(0, i - 1);
    nodeIndex = null;
  }

  function importShape() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.svg,.dxf,image/svg+xml';
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return;
      try {
        const c = JSON.parse(await store.engine.importShape(await f.text())) as CustomShape;
        edit((s) => s.layers.push(...c.layers));
        layerIndex = (shape?.layers.length ?? 1) - 1;
        store.toast(t('workshop.imported', { name: f.name }));
      } catch (err) {
        const code = err instanceof Error ? err.message : '';
        store.toast(t(`errors.${code}` as never));
      }
    };
    input.click();
  }

  const symmetryType = $derived(layer?.symmetry?.type ?? 'none');
  function setSymmetry(v: 'none' | 'mirror' | 'radial') {
    editLayer((l) => {
      l.symmetry = v === 'none' ? { type: 'none' } : v === 'mirror' ? { type: 'mirror', vertical: true, horizontal: false } : { type: 'radial', count: 4 };
    });
  }

  function toggleCurve(on: boolean) {
    const i = nodeIndex;
    if (i == null) return;
    editLayer((l) => {
      const n = l.nodes[i];
      if (on) Object.assign(n, smoothHandles(l, i));
      else {
        n.handle_in = null;
        n.handle_out = null;
      }
    });
  }

  const mm = (u: number) => u * size;
  const unit = (v: number) => (size > 0 ? v / size : v);
</script>

<svelte:window onpointermove={move} onpointerup={stop} onpointercancel={stop} />

<dialog bind:this={dialog} class="dlg" aria-labelledby="ws-title" onclose={() => (store.workshop = null)} onkeydown={key}>
  <header>
    <h2 id="ws-title">{t(target === 'kind' ? 'workshop.title' : 'workshop.titleMorph')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('help.close')} onclick={() => dialog?.close()}><Close size={16} /></button>
  </header>
  {#if shape && layer}
    <div class="body">
      <div class="view">
        <svg bind:this={svg} viewBox={`0 0 ${W} ${W}`} width={W} height={W} role="img" aria-label={t('workshop.view')} ondblclick={insert}>
          <rect class="matter" x="0" y="0" width={W} height={W} />
          {#each grid as g (g)}
            <line class="grid" class:axis={Math.abs(g) < 1e-9} x1={X(g)} y1="0" x2={X(g)} y2={W} />
            <line class="grid" class:axis={Math.abs(g) < 1e-9} x1="0" y1={Y(g)} x2={W} y2={Y(g)} />
          {/each}
          <rect class="unit" x={X(-0.5)} y={Y(0.5)} width={X(0.5) - X(-0.5)} height={Y(-0.5) - Y(0.5)} />
          {#if preview}
            <path class="result" d={path(preview.result, pivot.x, pivot.y)} />
            {#each preview.layers as l, i (i)}
              <path class="layer" class:sel={i === layerIndex} class:sub={shape.layers[i]?.role === 'subtract'} d={path(l, pivot.x, pivot.y)} />
            {/each}
          {/if}
          {#each layer.nodes as n, i (i)}
            {#if i === nodeIndex}
              {#each [['in', n.handle_in], ['out', n.handle_out]] as [kind, h] (kind)}
                {#if h && typeof h === 'object'}
                  <line class="arm" x1={X(n.at.x)} y1={Y(n.at.y)} x2={X(n.at.x + h.x)} y2={Y(n.at.y + h.y)} />
                  <rect
                    class="handle"
                    x={X(n.at.x + h.x) - 4}
                    y={Y(n.at.y + h.y) - 4}
                    width="8"
                    height="8"
                    role="presentation"
                    onpointerdown={(e) => start(e, { kind: kind as 'in' | 'out', index: i })}
                  />
                {/if}
              {/each}
            {/if}
            <circle class="node" class:on={i === nodeIndex} cx={X(n.at.x)} cy={Y(n.at.y)} r="5" role="presentation" onpointerdown={(e) => start(e, { kind: 'node', index: i })} />
          {/each}
          <g class="pivot" role="presentation" onpointerdown={(e) => start(e, { kind: 'pivot', index: 0 })}>
            <circle cx={X(pivot.x)} cy={Y(pivot.y)} r="7" />
            <line x1={X(pivot.x) - 11} y1={Y(pivot.y)} x2={X(pivot.x) + 11} y2={Y(pivot.y)} />
            <line x1={X(pivot.x)} y1={Y(pivot.y) - 11} x2={X(pivot.x)} y2={Y(pivot.y) + 11} />
          </g>
        </svg>
        <p class="hint">{t('workshop.hint')}</p>
        {#if preview && preview.dropped > 0}<p class="warn">{t('workshop.dropped', { count: preview.dropped })}</p>{/if}
      </div>

      <div class="side">
        <p class="sub">{t('workshop.layers')}</p>
        <ul class="list">
          {#each shape.layers as l, i (i)}
            <li>
              <button class="row" class:selected={i === layerIndex} type="button" onclick={() => ((layerIndex = i), (nodeIndex = null))}>
                {#if l.role === 'union'}<Plus size={13} />{:else}<Minus size={13} />{/if}
                <span class="rname">{t('workshop.layer', { n: i + 1 })}</span>
                <span class="rval">{t(`workshop.roles.${l.role}`)}</span>
              </button>
            </li>
          {/each}
        </ul>
        <div class="tools">
          <button class="btn quiet icon" type="button" aria-label={t('workshop.up')} use:tip={t('workshop.up')} disabled={layerIndex === 0} onclick={() => moveLayer(-1)}><ArrowUp size={14} /></button>
          <button class="btn quiet icon" type="button" aria-label={t('workshop.down')} use:tip={t('workshop.down')} disabled={layerIndex >= shape.layers.length - 1} onclick={() => moveLayer(1)}><ArrowDown size={14} /></button>
          <button class="btn quiet icon" type="button" aria-label={t('workshop.duplicate')} use:tip={t('workshop.duplicate')} onclick={duplicateLayer}><Copy size={14} /></button>
          <button class="btn quiet icon" type="button" aria-label={t('workshop.delete')} use:tip={t('workshop.delete')} disabled={shape.layers.length <= 1} onclick={deleteLayer}><Trash2 size={14} /></button>
        </div>
        <p class="sub">{t('workshop.add')}</p>
        <div class="adds">
          {#each PRIMITIVES as pr (pr)}
            <button class="chip" type="button" onclick={() => addLayer(pr)}>{t(`workshop.primitives.${pr}`)}</button>
          {/each}
          <button class="chip" type="button" use:tip={t('workshop.importHint')} onclick={importShape}><Upload size={12} /> {t('workshop.import')}</button>
        </div>

        <p class="sub">{t('workshop.layer', { n: layerIndex + 1 })}</p>
        <Field label={t('workshop.role')}>
          <Segmented
            label={t('workshop.role')}
            value={layer.role}
            options={[
              { id: 'union', label: t('workshop.roles.union'), hint: t('workshop.unionHint') },
              { id: 'subtract', label: t('workshop.roles.subtract'), hint: t('workshop.subtractHint') },
            ]}
            onchange={(v: LayerRole) => editLayer((l) => (l.role = v), 'role')}
          />
        </Field>
        <p class="label">{t('workshop.symmetry')}</p>
        <Segmented
          label={t('workshop.symmetry')}
          value={symmetryType}
          options={[
            { id: 'none', label: t('workshop.symmetries.none') },
            { id: 'mirror', label: t('workshop.symmetries.mirror'), hint: t('workshop.mirrorHint') },
            { id: 'radial', label: t('workshop.symmetries.radial'), hint: t('workshop.radialHint') },
          ]}
          onchange={setSymmetry}
        />
        {#if layer.symmetry?.type === 'mirror'}
          {@const s = layer.symmetry}
          <div class="checks">
            <label class="check"><input type="checkbox" checked={s.vertical} onchange={(e) => editLayer((l) => l.symmetry?.type === 'mirror' && (l.symmetry.vertical = (e.currentTarget as HTMLInputElement).checked))} /> {t('workshop.vertical')}</label>
            <label class="check"><input type="checkbox" checked={s.horizontal} onchange={(e) => editLayer((l) => l.symmetry?.type === 'mirror' && (l.symmetry.horizontal = (e.currentTarget as HTMLInputElement).checked))} /> {t('workshop.horizontal')}</label>
          </div>
        {:else if layer.symmetry?.type === 'radial'}
          {@const s = layer.symmetry}
          <NumField label={t('workshop.copies')} kind="count" min={2} max={12} field="shape.layers.symmetry" value={s.count} onchange={(v) => editLayer((l) => l.symmetry?.type === 'radial' && (l.symmetry.count = Math.round(v)), 'count')} />
        {/if}
        <NumField label={t('workshop.response')} hint={t('workshop.responseHint')} kind="percent" min={0} max={2} value={layer.response} onchange={(v) => editLayer((l) => (l.response = v), 'response')} />

        {#if node && nodeIndex != null}
          {@const i = nodeIndex}
          <p class="sub">{t('workshop.node', { n: i + 1 })}</p>
          <NumField label="X" value={mm(node.at.x)} onchange={(v) => editLayer((l) => (l.nodes[i].at.x = unit(v)), `x${i}`)} />
          <NumField label="Y" value={mm(node.at.y)} onchange={(v) => editLayer((l) => (l.nodes[i].at.y = unit(v)), `y${i}`)} />
          <NumField label={t('workshop.fillet')} hint={t('workshop.filletHint')} min={0} value={mm(node.fillet ?? 0)} onchange={(v) => editLayer((l) => (l.nodes[i].fillet = unit(v)), `f${i}`)} />
          <label class="check" use:tip={t('workshop.curveHint')}>
            <input type="checkbox" checked={!!(node.handle_in || node.handle_out)} onchange={(e) => toggleCurve((e.currentTarget as HTMLInputElement).checked)} />
            {t('workshop.curve')}
          </label>
          <button class="btn quiet" type="button" disabled={layer.nodes.length <= 3} onclick={removeNode}><Trash2 size={14} /> {t('workshop.removeNode')}</button>
        {:else}
          <p class="muted">{t('workshop.pickNode')}</p>
        {/if}
      </div>
    </div>
  {/if}
</dialog>

<style>
  .dlg {
    width: min(900px, calc(100vw - 32px));
    padding: 0;
    border: 0;
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-pop);
  }

  .dlg::backdrop {
    background: var(--backdrop);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--s3) var(--s4);
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  .body {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--s4);
    padding: var(--s3) var(--s4) var(--s4);
    max-height: 80vh;
    overflow: auto;
  }

  .view {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }

  svg {
    display: block;
    max-width: 100%;
    height: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    touch-action: none;
    user-select: none;
  }

  .matter {
    fill: var(--plate-fill);
  }

  .grid {
    stroke: var(--plate-margin);
    stroke-width: 0.5;
  }

  .grid.axis {
    stroke-width: 1;
  }

  .unit {
    fill: none;
    stroke: var(--plate-edge);
    stroke-dasharray: 4 4;
  }

  .result {
    fill: var(--hole-fill);
  }

  .layer {
    fill: none;
    stroke: var(--ctrl-dim);
    stroke-width: 1;
  }

  .layer.sub {
    stroke-dasharray: 4 3;
  }

  .layer.sel {
    stroke: var(--ctrl-active);
    stroke-width: 1.5;
  }

  .node {
    fill: var(--ctrl);
    stroke: var(--ctrl-edge);
    stroke-width: 1.5;
    cursor: move;
  }

  .node.on {
    fill: var(--ctrl-active);
  }

  .arm {
    stroke: var(--ctrl-active);
    stroke-width: 1;
  }

  .handle {
    fill: var(--ctrl);
    stroke: var(--ctrl-active);
    stroke-width: 1.5;
    cursor: move;
  }

  .pivot {
    cursor: move;
  }

  .pivot circle {
    fill: var(--ctrl-active-fill);
    stroke: var(--ctrl-active);
    stroke-width: 1.5;
  }

  .pivot line {
    stroke: var(--ctrl-active);
    stroke-width: 1.5;
  }

  .hint,
  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.45;
    max-width: 460px;
  }

  .warn {
    margin: 0;
    color: var(--warning);
    font-size: var(--fs-sm);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }

  .sub {
    margin: var(--s2) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .sub:first-child {
    margin-top: 0;
  }

  .label {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: 26px;
    padding: 0 var(--s1);
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text-secondary);
    text-align: left;
  }

  .row:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .row.selected {
    background: var(--bg-selected);
    color: var(--text-primary);
  }

  .rname {
    flex: 1;
  }

  .rval {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .adds {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 24px;
    padding: 0 var(--s2);
    background: var(--bg-control);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: var(--fs-sm);
  }

  .chip:hover {
    border-color: var(--accent);
  }

  .checks {
    display: flex;
    gap: var(--s3);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    cursor: pointer;
  }

  input[type='checkbox'] {
    accent-color: var(--accent);
    margin: 0;
  }

  @media (max-width: 860px) {
    .body {
      grid-template-columns: 1fr;
    }
  }
</style>
