<script lang="ts">
  // Panneau gauche : le motif (plaque, disposition, forme, bords,
  // fabrication). Chaque valeur modifie le projet en une étape d'annulation.
  import Section from './Section.svelte';
  import NumField from './NumField.svelte';
  import Field from './Field.svelte';
  import Segmented from './Segmented.svelte';
  import Disclosure from './Disclosure.svelte';
  import ShapeGlyph from './ShapeGlyph.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import type { EdgeMode, GridPattern, Outline, ShapeKind, ShapeType } from '../lib/types';

  const p = $derived(store.p);
  const grid = $derived(p.layout);
  const kind = $derived(p.shape.kind);

  const SHAPES: ShapeType[] = ['circle', 'rectangle', 'polygon', 'star', 'slot'];
  const DEFAULT_KIND: Record<ShapeType, ShapeKind> = {
    circle: { type: 'circle' },
    rectangle: { type: 'rectangle', aspect: 0.6, corner: 0.5 },
    polygon: { type: 'polygon', sides: 6 },
    star: { type: 'star', points: 5, inner: 0.5 },
    slot: { type: 'slot', aspect: 0.35 },
  };

  function setShape(type: ShapeType) {
    if (type === kind.type) return;
    store.edit((q) => {
      q.shape.kind = structuredClone(DEFAULT_KIND[type]);
    });
  }

  const sizeLabel = $derived(
    kind.type === 'rectangle' ? t('shape.width') : kind.type === 'slot' ? t('shape.length') : t('shape.diameter'),
  );
</script>

<aside class="panel" aria-label={t('panel.pattern')}>
  <Section id="plate" title={t('plate.title')} step={1}>
    <Field label={t('plate.outline')}>
      <Segmented
        label={t('plate.outline')}
        value={p.plate.outline}
        options={[
          { id: 'rectangle', label: t('plate.rectangle') },
          { id: 'circle', label: t('plate.circle') },
        ]}
        onchange={(v: Outline) => store.edit((q) => (q.plate.outline = v))}
      />
    </Field>
    {#if p.plate.outline === 'rectangle'}
      <NumField label={t('plate.width')} field="plate.width" min={0} value={p.plate.width} onchange={(v) => store.edit((q) => (q.plate.width = v), 'plate.width')} />
      <NumField label={t('plate.height')} field="plate.height" min={0} value={p.plate.height} onchange={(v) => store.edit((q) => (q.plate.height = v), 'plate.height')} />
      <NumField label={t('plate.corner')} field="plate.corner_radius" min={0} value={p.plate.corner_radius} onchange={(v) => store.edit((q) => (q.plate.corner_radius = v), 'plate.corner')} />
    {:else}
      <NumField label={t('plate.diameter')} field="plate.width" min={0} value={p.plate.width} onchange={(v) => store.edit((q) => (q.plate.width = v), 'plate.width')} />
    {/if}
    <NumField label={t('plate.margin')} hint={t('plate.marginHint')} field="plate.margin" min={0} value={p.plate.margin} onchange={(v) => store.edit((q) => (q.plate.margin = v), 'plate.margin')} />
    <NumField label={t('plate.thickness')} hint={t('plate.thicknessHint')} field="plate.thickness" min={0} value={p.plate.thickness} onchange={(v) => store.edit((q) => (q.plate.thickness = v), 'plate.thickness')} />
  </Section>

  <Section id="layout" title={t('layout.title')} step={2}>
    <Field label={t('layout.mode')}>
      <select class="input select" aria-label={t('layout.mode')} value="grid">
        <option value="grid">{t('layout.grid')}</option>
      </select>
    </Field>
    <Segmented
      label={t('layout.pattern')}
      value={grid.pattern}
      options={[
        { id: 'square', label: t('layout.square'), hint: t('layout.squareHint') },
        { id: 'hexagonal', label: t('layout.hexagonal'), hint: t('layout.hexagonalHint') },
        { id: 'staggered', label: t('layout.staggered'), hint: t('layout.staggeredHint') },
      ]}
      onchange={(v: GridPattern) => store.edit((q) => (q.layout.pattern = v))}
    />
    {#if grid.pattern === 'hexagonal'}
      <NumField label={t('layout.pitch')} hint={t('layout.pitchHint')} field="layout.pitch_x" min={0} value={grid.pitch_x} onchange={(v) => store.edit((q) => (q.layout.pitch_x = v), 'layout.pitch_x')} />
    {:else}
      <NumField label={t('layout.pitchX')} hint={t('layout.pitchHint')} field="layout.pitch_x" min={0} value={grid.pitch_x} onchange={(v) => store.edit((q) => (q.layout.pitch_x = v), 'layout.pitch_x')} />
      <NumField label={t('layout.pitchY')} field="layout.pitch_y" min={0} value={grid.pitch_y} onchange={(v) => store.edit((q) => (q.layout.pitch_y = v), 'layout.pitch_y')} />
    {/if}
    <NumField label={t('layout.angle')} kind="angle" field="layout.angle" value={grid.angle} onchange={(v) => store.edit((q) => (q.layout.angle = v), 'layout.angle')} />
    <Disclosure id="layout.advanced" label={t('layout.advanced')}>
      <NumField label={t('layout.offsetX')} field="layout.offset_x" value={grid.offset_x} onchange={(v) => store.edit((q) => (q.layout.offset_x = v), 'layout.offset_x')} />
      <NumField label={t('layout.offsetY')} field="layout.offset_y" value={grid.offset_y} onchange={(v) => store.edit((q) => (q.layout.offset_y = v), 'layout.offset_y')} />
      <NumField label={t('layout.rowTwist')} hint={t('layout.twistHint')} kind="angle" field="layout.row_twist" value={grid.row_twist} onchange={(v) => store.edit((q) => (q.layout.row_twist = v), 'layout.row_twist')} />
      <NumField label={t('layout.colTwist')} kind="angle" field="layout.col_twist" value={grid.col_twist} onchange={(v) => store.edit((q) => (q.layout.col_twist = v), 'layout.col_twist')} />
    </Disclosure>
  </Section>

  <Section id="shape" title={t('shape.title')} step={3}>
    <div class="shapes" role="radiogroup" aria-label={t('shape.type')}>
      {#each SHAPES as s (s)}
        <button
          type="button"
          role="radio"
          aria-checked={kind.type === s}
          aria-label={t(`shape.types.${s}`)}
          class:on={kind.type === s}
          use:tip={t(`shape.types.${s}`)}
          onclick={() => setShape(s)}><ShapeGlyph type={s} /></button
        >
      {/each}
    </div>
    <NumField label={sizeLabel} hint={t('shape.sizeHint')} field="shape.size" min={0} value={p.shape.size} onchange={(v) => store.edit((q) => (q.shape.size = v), 'shape.size')} />
    {#if kind.type === 'rectangle'}
      {@const k = kind}
      <NumField
        label={t('shape.height')}
        field="shape.aspect"
        min={0}
        value={p.shape.size * k.aspect}
        onchange={(v) => store.edit((q) => q.shape.kind.type === 'rectangle' && (q.shape.kind.aspect = q.shape.size > 0 ? v / q.shape.size : 1), 'shape.aspect')}
      />
      <NumField label={t('shape.corner')} field="shape.corner" min={0} value={k.corner} onchange={(v) => store.edit((q) => q.shape.kind.type === 'rectangle' && (q.shape.kind.corner = v), 'shape.corner')} />
    {:else if kind.type === 'slot'}
      {@const k = kind}
      <NumField
        label={t('shape.slotWidth')}
        field="shape.aspect"
        min={0}
        value={p.shape.size * k.aspect}
        onchange={(v) => store.edit((q) => q.shape.kind.type === 'slot' && (q.shape.kind.aspect = q.shape.size > 0 ? v / q.shape.size : 1), 'shape.aspect')}
      />
    {:else if kind.type === 'polygon'}
      {@const k = kind}
      <NumField label={t('shape.sides')} kind="count" field="shape.sides" min={3} max={64} value={k.sides} onchange={(v) => store.edit((q) => q.shape.kind.type === 'polygon' && (q.shape.kind.sides = Math.round(v)), 'shape.sides')} />
    {:else if kind.type === 'star'}
      {@const k = kind}
      <NumField label={t('shape.points')} kind="count" field="shape.points" min={3} max={64} value={k.points} onchange={(v) => store.edit((q) => q.shape.kind.type === 'star' && (q.shape.kind.points = Math.round(v)), 'shape.points')} />
      <NumField label={t('shape.inner')} hint={t('shape.innerHint')} kind="percent" field="shape.inner" min={0.01} max={0.99} value={k.inner} onchange={(v) => store.edit((q) => q.shape.kind.type === 'star' && (q.shape.kind.inner = v), 'shape.inner')} />
    {/if}
    {#if kind.type !== 'circle'}
      <NumField label={t('shape.rotation')} kind="angle" field="shape.rotation" value={p.shape.rotation} onchange={(v) => store.edit((q) => (q.shape.rotation = v), 'shape.rotation')} />
    {/if}
  </Section>

  <Section id="edge" title={t('edge.title')} step={4}>
    <Segmented
      label={t('edge.title')}
      value={p.edge}
      options={[
        { id: 'center', label: t('edge.center'), hint: t('edge.centerHint') },
        { id: 'whole', label: t('edge.whole'), hint: t('edge.wholeHint') },
      ]}
      onchange={(v: EdgeMode) => store.edit((q) => (q.edge = v))}
    />
  </Section>

  <Section id="fabrication" title={t('fab.title')} step={5}>
    <NumField label={t('fab.minSize')} hint={t('fab.minSizeHint')} field="fabrication.min_size" min={0} value={p.fabrication.min_size} onchange={(v) => store.edit((q) => (q.fabrication.min_size = v), 'fab.min_size')} />
    <NumField label={t('fab.minBridge')} hint={t('fab.minBridgeHint')} field="fabrication.min_bridge" min={0} value={p.fabrication.min_bridge} onchange={(v) => store.edit((q) => (q.fabrication.min_bridge = v), 'fab.min_bridge')} />
  </Section>
</aside>

<style>
  .panel {
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    min-height: 0;
  }

  .select {
    width: 100%;
    appearance: none;
    background-image: linear-gradient(45deg, transparent 50%, var(--text-muted) 50%),
      linear-gradient(135deg, var(--text-muted) 50%, transparent 50%);
    background-position:
      calc(100% - 13px) 12px,
      calc(100% - 9px) 12px;
    background-size:
      4px 4px,
      4px 4px;
    background-repeat: no-repeat;
  }

  .shapes {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 2px;
    height: 32px;
    padding: 2px;
    background: var(--segment-track);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .shapes button {
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--text-secondary);
  }

  .shapes button:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .shapes button.on {
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
</style>
