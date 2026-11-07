<script lang="ts">
  // Panneau gauche : le motif (plaque, disposition, forme, bords,
  // fabrication). Chaque valeur modifie le projet en une étape d'annulation.
  import Section from './Section.svelte';
  import NumField from './NumField.svelte';
  import Field from './Field.svelte';
  import Segmented from './Segmented.svelte';
  import Disclosure from './Disclosure.svelte';
  import ShapeGlyph from './ShapeGlyph.svelte';
  import LayoutPicker from './LayoutPicker.svelte';
  import { Dices, PenTool, Spline, Upload, Ban } from '@lucide/svelte';
  import { pickOutline } from '../lib/files';
  import { toCustom } from '../lib/shapes';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { shapesHoles, type EdgeMode, type FlowEnds, type GridPattern, type Outline, type ShapeKind, type ShapeType, type Zone } from '../lib/types';

  const p = $derived(store.p);
  const l = $derived(p.layout);
  const kind = $derived(p.shape.kind);

  /** Champ de la disposition courante (le champ n'est montré que pour les modes qui l'ont). */
  function setL(field: string, v: number | boolean) {
    store.edit((q) => {
      (q.layout as unknown as Record<string, unknown>)[field] = v;
    }, `layout.${field}`);
  }

  function setOutline(v: Outline) {
    if (v === 'custom' && !(p.plate.custom?.length ?? 0)) {
      pickOutline();
      return;
    }
    store.edit((q) => (q.plate.outline = v));
  }

  const zone = $derived<Zone>(p.plate.zone ?? { type: 'plate' });
  function setZone(type: Zone['type']) {
    const ext = store.plateExtent();
    const center = { x: ext.x / 2, y: ext.y / 2 };
    const r = (v: number) => Math.round(v * 10) / 10;
    store.edit((q) => {
      if (type === 'plate') delete q.plate.zone;
      else if (type === 'circle') q.plate.zone = { type, center, diameter: r(0.8 * Math.min(ext.x, ext.y)) };
      else q.plate.zone = { type, center, width: r(0.8 * ext.x), height: r(0.8 * ext.y), corner: 4 };
    });
  }
  function setZoneField(field: string, v: number) {
    store.edit((q) => {
      const z = q.plate.zone as unknown as Record<string, unknown> | undefined;
      if (!z) return;
      if (field === 'cx' || field === 'cy') {
        const c = z.center as { x: number; y: number };
        z.center = field === 'cx' ? { ...c, x: v } : { ...c, y: v };
      } else z[field] = v;
    }, `zone.${field}`);
  }

  function drawPath() {
    store.setTab('path');
    store.tool = 'curve';
  }

  const SHAPES: ShapeType[] = ['circle', 'rectangle', 'polygon', 'star', 'slot', 'custom'];
  const DEFAULT_KIND: Record<ShapeType, ShapeKind> = {
    circle: { type: 'circle' },
    rectangle: { type: 'rectangle', aspect: 0.6, corner: 0.5 },
    polygon: { type: 'polygon', sides: 6 },
    star: { type: 'star', points: 5, inner: 0.5 },
    slot: { type: 'slot', aspect: 0.35 },
    custom: { type: 'custom', layers: [], pivot: { x: 0, y: 0 } },
  };

  /** Forme choisie ; « dessinée » part de la forme actuelle et ouvre l'atelier. */
  function setShape(type: ShapeType) {
    if (type === kind.type) {
      if (type === 'custom') store.workshop = 'kind';
      return;
    }
    store.edit((q) => {
      q.shape.kind = type === 'custom' ? { type: 'custom', ...toCustom(q.shape.kind, q.shape.size) } : structuredClone(DEFAULT_KIND[type]);
    });
    if (type === 'custom') store.workshop = 'kind';
  }

  const morphType = $derived(p.shape.morph?.type ?? 'none');
  function setMorph(type: ShapeType | 'none') {
    store.edit((q) => {
      if (type === 'none') {
        q.shape.morph = null;
        for (const c of q.controllers) delete c.morph;
      } else if (type !== q.shape.morph?.type) {
        q.shape.morph = type === 'custom' ? { type: 'custom', ...toCustom(q.shape.kind, q.shape.size) } : structuredClone(DEFAULT_KIND[type]);
      }
    });
    if (type === 'custom') store.workshop = 'morph';
  }

  const sizeLabel = $derived(
    kind.type === 'rectangle' || kind.type === 'custom' ? t('shape.width') : kind.type === 'slot' ? t('shape.length') : t('shape.diameter'),
  );
</script>

<aside class="panel" aria-label={t('panel.pattern')}>
  <Section id="plate" title={t('plate.title')} step={1}>
    <Segmented
      label={t('plate.outline')}
      value={p.plate.outline}
      options={[
        { id: 'rectangle', label: t('plate.rectangle') },
        { id: 'circle', label: t('plate.circle') },
        { id: 'ellipse', label: t('plate.ellipse') },
        { id: 'custom', label: t('plate.custom'), hint: t('plate.customHint') },
      ]}
      onchange={setOutline}
    />
    {#if p.plate.outline === 'circle'}
      <NumField label={t('plate.diameter')} field="plate.width" min={0} value={p.plate.width} onchange={(v) => store.resizePlate(v, p.plate.height)} />
    {:else}
      <NumField label={t('plate.width')} field="plate.width" min={0} value={p.plate.width} onchange={(v) => store.resizePlate(v, p.plate.height)} />
      <NumField label={t('plate.height')} field="plate.height" min={0} value={p.plate.height} onchange={(v) => store.resizePlate(p.plate.width, v)} />
    {/if}
    {#if p.plate.outline === 'rectangle'}
      <NumField label={t('plate.corner')} field="plate.corner_radius" min={0} value={p.plate.corner_radius} onchange={(v) => store.edit((q) => (q.plate.corner_radius = v), 'plate.corner')} />
    {/if}
    <button class="btn quiet import" type="button" use:tip={t('plate.importHint')} onclick={pickOutline}><Upload size={14} /> {t('plate.import')}</button>
    <label class="check" use:tip={t('plate.scalePatternHint')}>
      <input type="checkbox" bind:checked={store.scalePattern} />
      {t('plate.scalePattern')}
    </label>
    <NumField label={t('plate.margin')} hint={t('plate.marginHint')} field="plate.margin" min={0} value={p.plate.margin} onchange={(v) => store.edit((q) => (q.plate.margin = v), 'plate.margin')} />
    <NumField label={t('plate.thickness')} hint={t('plate.thicknessHint')} field="plate.thickness" min={0} value={p.plate.thickness} onchange={(v) => store.edit((q) => (q.plate.thickness = v), 'plate.thickness')} />
    <p class="sub" use:tip={t('zone.hint')}>{t('zone.title')}</p>
    <Segmented
      label={t('zone.title')}
      value={zone.type}
      options={[
        { id: 'plate', label: t('zone.plate'), hint: t('zone.hint') },
        { id: 'circle', label: t('zone.circle'), hint: t('zone.hint') },
        { id: 'rectangle', label: t('zone.rectangle'), hint: t('zone.hint') },
      ]}
      onchange={setZone}
    />
    {#if zone.type === 'circle'}
      <NumField label={t('plate.diameter')} field="plate.zone.diameter" min={0} value={zone.diameter} onchange={(v) => setZoneField('diameter', v)} />
    {:else if zone.type === 'rectangle'}
      <NumField label={t('plate.width')} field="plate.zone.width" min={0} value={zone.width} onchange={(v) => setZoneField('width', v)} />
      <NumField label={t('plate.height')} field="plate.zone.height" min={0} value={zone.height} onchange={(v) => setZoneField('height', v)} />
      <NumField label={t('plate.corner')} field="plate.zone.corner" min={0} value={zone.corner} onchange={(v) => setZoneField('corner', v)} />
    {/if}
    {#if zone.type !== 'plate'}
      <Disclosure id="zone.center" label={t('zone.center')}>
        <NumField label={t('ctrl.cx')} value={zone.center.x} onchange={(v) => setZoneField('cx', v)} />
        <NumField label={t('ctrl.cy')} value={zone.center.y} onchange={(v) => setZoneField('cy', v)} />
      </Disclosure>
    {/if}
  </Section>

  <Section id="layout" title={t('layout.title')} step={2}>
    <Field label={t('layout.mode')}>
      <LayoutPicker />
    </Field>
    {#if l.mode === 'grid'}
      <Segmented
        label={t('layout.pattern')}
        value={l.pattern}
        options={[
          { id: 'square', label: t('layout.square'), hint: t('layout.squareHint') },
          { id: 'hexagonal', label: t('layout.hexagonal'), hint: t('layout.hexagonalHint') },
          { id: 'staggered', label: t('layout.staggered'), hint: t('layout.staggeredHint') },
        ]}
        onchange={(v: GridPattern) => setL('pattern', v as never)}
      />
      {#if l.pattern === 'hexagonal'}
        <NumField label={t('layout.pitch')} hint={t('layout.pitchHint')} field="layout.pitch_x" min={0} value={l.pitch_x} onchange={(v) => setL('pitch_x', v)} />
      {:else}
        <NumField label={t('layout.pitchX')} hint={t('layout.pitchHint')} field="layout.pitch_x" min={0} value={l.pitch_x} onchange={(v) => setL('pitch_x', v)} />
        <NumField label={t('layout.pitchY')} field="layout.pitch_y" min={0} value={l.pitch_y} onchange={(v) => setL('pitch_y', v)} />
      {/if}
      <NumField label={t('layout.angle')} kind="angle" field="layout.angle" value={l.angle} onchange={(v) => setL('angle', v)} />
      <Disclosure id="layout.advanced" label={t('layout.advanced')}>
        <NumField label={t('layout.offsetX')} field="layout.offset_x" value={l.offset_x} onchange={(v) => setL('offset_x', v)} />
        <NumField label={t('layout.offsetY')} field="layout.offset_y" value={l.offset_y} onchange={(v) => setL('offset_y', v)} />
        <NumField label={t('layout.rowTwist')} hint={t('layout.twistHint')} kind="angle" field="layout.row_twist" value={l.row_twist} onchange={(v) => setL('row_twist', v)} />
        <NumField label={t('layout.colTwist')} kind="angle" field="layout.col_twist" value={l.col_twist} onchange={(v) => setL('col_twist', v)} />
      </Disclosure>
    {:else if l.mode === 'random'}
      <NumField label={t('layout.spacing')} hint={t('layout.spacingHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <Field label={t('layout.seed')} hint={t('layout.seedHint')}>
        <span class="val tabular">{l.seed}</span>
        <button class="btn quiet grow-right" type="button" use:tip={t('layout.reroll')} onclick={() => setL('seed', (l.seed % 99999) + 1)}><Dices size={14} /> {t('ctrl.rerollShort')}</button>
      </Field>
    {:else if l.mode === 'spiral'}
      <NumField label={t('layout.turnPitch')} hint={t('layout.turnPitchHint')} field="layout.pitch" min={0} value={l.pitch} onchange={(v) => setL('pitch', v)} />
      <NumField label={t('layout.spacingAlong')} hint={t('layout.spacingHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.start')} field="layout.start" min={0} value={l.start} onchange={(v) => setL('start', v)} />
      <Segmented
        label={t('layout.direction')}
        value={l.clockwise ? 'cw' : 'ccw'}
        options={[
          { id: 'ccw', label: t('layout.ccw') },
          { id: 'cw', label: t('layout.cw') },
        ]}
        onchange={(v: 'cw' | 'ccw') => setL('clockwise', v === 'cw')}
      />
    {:else if l.mode === 'concentric'}
      <NumField label={t('layout.ringPitch')} field="layout.pitch" min={0} value={l.pitch} onchange={(v) => setL('pitch', v)} />
      <NumField label={t('layout.spacingAlong')} hint={t('layout.spacingHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.start')} hint={t('layout.startHint')} field="layout.start" min={0} value={l.start} onchange={(v) => setL('start', v)} />
      <NumField label={t('layout.phase')} hint={t('layout.phaseHint')} kind="angle" field="layout.phase" value={l.phase} onchange={(v) => setL('phase', v)} />
    {:else if l.mode === 'fibonacci'}
      <NumField label={t('layout.spacing')} hint={t('layout.spacingHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.divergence')} hint={t('layout.divergenceHint')} kind="angle" decimals={3} field="layout.divergence" value={l.divergence} onchange={(v) => setL('divergence', v)} />
    {:else if l.mode === 'cross_hatch'}
      <NumField label={t('layout.step')} hint={t('layout.stepHint')} field="layout.step" min={0} value={l.step} onchange={(v) => setL('step', v)} />
      <NumField label={t('layout.lineSpacing')} field="layout.line_spacing" min={0} value={l.line_spacing} onchange={(v) => setL('line_spacing', v)} />
      <NumField label={t('layout.angle')} kind="angle" field="layout.angle" value={l.angle} onchange={(v) => setL('angle', v)} />
      <NumField label={t('layout.cross')} hint={t('layout.crossHint')} kind="angle" field="layout.cross" value={l.cross} onchange={(v) => setL('cross', v)} />
    {:else if l.mode === 'path'}
      <NumField label={t('layout.spacingAlong')} hint={t('layout.spacingHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.rows')} kind="count" field="layout.rows" min={1} max={50} value={l.rows} onchange={(v) => setL('rows', Math.round(v))} />
      {#if l.rows > 1}
        <NumField label={t('layout.rowGap')} field="layout.row_gap" min={0} value={l.row_gap} onchange={(v) => setL('row_gap', v)} />
      {/if}
      <button class="btn" type="button" onclick={drawPath}><Spline size={14} /> {l.paths.length ? t('layout.drawAnother') : t('layout.drawPath')}</button>
    {:else if l.mode === 'voronoi'}
      <NumField label={t('layout.cellSize')} hint={t('layout.cellSizeHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.rib')} hint={t('layout.ribHint')} field="layout.rib" min={0} value={l.rib} onchange={(v) => setL('rib', v)} />
      <NumField label={t('layout.fillet')} hint={t('layout.filletHint')} field="layout.fillet" min={0} value={l.fillet} onchange={(v) => setL('fillet', v)} />
      <NumField label={t('layout.relax')} hint={t('layout.relaxHint')} kind="count" field="layout.relax" min={0} max={20} value={l.relax} onchange={(v) => setL('relax', Math.round(v))} />
      <Field label={t('layout.seed')} hint={t('layout.seedHint')}>
        <span class="val tabular">{l.seed}</span>
        <button class="btn quiet grow-right" type="button" use:tip={t('layout.reroll')} onclick={() => setL('seed', (l.seed % 99999) + 1)}><Dices size={14} /> {t('ctrl.rerollShort')}</button>
      </Field>
    {:else if l.mode === 'script'}
      <p class="sub" use:tip={t('layout.scriptHint')}>{t('layout.script')}</p>
      <textarea
        class="input code"
        rows="14"
        spellcheck="false"
        aria-label={t('layout.script')}
        value={l.source}
        oninput={(e) => {
          const v = (e.currentTarget as HTMLTextAreaElement).value;
          store.edit((q) => q.layout.mode === 'script' && (q.layout.source = v), 'layout.source');
        }}
      ></textarea>
      {#each store.frame?.report.sliders ?? [] as sl (sl.name)}
        <NumField
          label={sl.name}
          hint={t('layout.sliderHint', { min: sl.min, max: sl.max, value: sl.default })}
          kind="plain"
          min={sl.min}
          max={sl.max}
          decimals={2}
          step={(sl.max - sl.min) / 100}
          value={sl.value}
          onchange={(v) =>
            store.edit((q) => {
              if (q.layout.mode !== 'script') return;
              q.layout.params = { ...(q.layout.params ?? {}), [sl.name]: v };
            }, `layout.params.${sl.name}`)}
        />
      {/each}
      <Field label={t('layout.seed')} hint={t('layout.scriptSeedHint')}>
        <span class="val tabular">{l.seed}</span>
        <button class="btn quiet grow-right" type="button" use:tip={t('layout.reroll')} onclick={() => setL('seed', (l.seed % 99999) + 1)}><Dices size={14} /> {t('ctrl.rerollShort')}</button>
      </Field>
    {:else if l.mode === 'flow'}
      <NumField label={t('layout.lineGap')} hint={t('layout.lineGapHint')} field="layout.spacing" min={0} value={l.spacing} onchange={(v) => setL('spacing', v)} />
      <NumField label={t('layout.ribbon')} hint={t('layout.ribbonHint')} field="layout.width" min={0} value={l.width} onchange={(v) => setL('width', v)} />
      <NumField label={t('layout.flowAngle')} hint={t('layout.flowAngleHint')} kind="angle" field="layout.angle" value={l.angle} onchange={(v) => setL('angle', v)} />
      <Field label={t('layout.ends')}>
        <Segmented
          label={t('layout.ends')}
          value={l.ends}
          options={[
            { id: 'round', label: t('layout.endsRound') },
            { id: 'flat', label: t('layout.endsFlat') },
          ]}
          onchange={(v: FlowEnds) => setL('ends', v as never)}
        />
      </Field>
    {/if}
    {#if l.mode === 'spiral' || l.mode === 'concentric' || l.mode === 'path' || l.mode === 'fibonacci'}
      <label class="check" use:tip={t(l.mode === 'fibonacci' ? 'layout.orientRadialHint' : 'layout.orientHint')}>
        <input type="checkbox" checked={l.orient} onchange={(e) => setL('orient', (e.currentTarget as HTMLInputElement).checked)} />
        {t(l.mode === 'fibonacci' ? 'layout.orientRadial' : 'layout.orient')}
      </label>
    {/if}
    {#if l.mode === 'spiral' || l.mode === 'concentric' || l.mode === 'fibonacci' || l.mode === 'cross_hatch'}
      <Disclosure id="layout.advanced" label={t('layout.advanced')}>
        <NumField label={t('layout.offsetX')} hint={t('layout.offsetHint')} field="layout.offset_x" value={l.offset_x} onchange={(v) => setL('offset_x', v)} />
        <NumField label={t('layout.offsetY')} field="layout.offset_y" value={l.offset_y} onchange={(v) => setL('offset_y', v)} />
      </Disclosure>
    {/if}
  </Section>

  <Section id="shape" title={t('shape.title')} step={3}>
    {#if shapesHoles(l)}
      <p class="note">{t(l.mode === 'voronoi' ? 'shape.byVoronoi' : 'shape.byFlow')}</p>
    {:else}
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
    {#if kind.type === 'custom'}
      <button class="btn import" type="button" use:tip={t('workshop.openHint')} onclick={() => (store.workshop = 'kind')}><PenTool size={14} /> {t('workshop.open')}</button>
    {/if}
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
    <p class="sub" use:tip={t('shape.targetHint')}>{t('shape.target')}</p>
    <div class="shapes" role="radiogroup" aria-label={t('shape.target')}>
      <button type="button" role="radio" aria-checked={morphType === 'none'} aria-label={t('shape.noTarget')} class:on={morphType === 'none'} use:tip={t('shape.noTarget')} onclick={() => setMorph('none')}><Ban size={16} /></button>
      {#each SHAPES as s (s)}
        <button
          type="button"
          role="radio"
          aria-checked={morphType === s}
          aria-label={t(`shape.types.${s}`)}
          class:on={morphType === s}
          use:tip={t(`shape.types.${s}`)}
          onclick={() => setMorph(s)}><ShapeGlyph type={s} /></button
        >
      {/each}
    </div>
    {#if morphType === 'custom'}
      <button class="btn import" type="button" onclick={() => (store.workshop = 'morph')}><PenTool size={14} /> {t('shape.editTarget')}</button>
    {/if}
    {/if}
  </Section>

  <Section id="edge" title={t('edge.title')} step={4}>
    <Segmented
      label={t('edge.title')}
      value={p.edge}
      options={[
        { id: 'center', label: t('edge.center'), hint: t('edge.centerHint') },
        { id: 'whole', label: t('edge.whole'), hint: t('edge.wholeHint') },
        { id: 'clip', label: t('edge.clip'), hint: t('edge.clipHint') },
      ]}
      onchange={(v: EdgeMode) => store.edit((q) => (q.edge = v))}
    />
  </Section>

  <Section id="fabrication" title={t('fab.title')} step={5}>
    <NumField label={t('fab.minSize')} hint={t('fab.minSizeHint')} field="fabrication.min_size" min={0} value={p.fabrication.min_size} onchange={(v) => store.edit((q) => (q.fabrication.min_size = v), 'fab.min_size')} />
    <NumField label={t('fab.minBridge')} hint={t('fab.minBridgeHint')} field="fabrication.min_bridge" min={0} value={p.fabrication.min_bridge} onchange={(v) => store.edit((q) => (q.fabrication.min_bridge = v), 'fab.min_bridge')} />
    <p class="sub">{t('fab.step')}</p>
    <NumField label={t('fab.draft')} hint={t('fab.draftHint')} field="fabrication.draft" kind="angle" step={0.5} value={p.fabrication.draft ?? 0} onchange={(v) => store.edit((q) => (q.fabrication.draft = v), 'fab.draft')} />
    {#each [0, 1] as axis (axis)}
      <NumField
        label={t(axis === 0 ? 'fab.tilesX' : 'fab.tilesY')}
        hint={t('fab.tilesHint')}
        field={axis === 0 ? 'fabrication.tiles_x' : 'fabrication.tiles_y'}
        kind="count"
        min={1}
        value={(p.fabrication.tiles ?? [1, 1])[axis]}
        onchange={(v) =>
          store.edit((q) => {
            const tiles: [number, number] = [...(q.fabrication.tiles ?? [1, 1])];
            tiles[axis] = Math.round(v);
            q.fabrication.tiles = tiles;
          }, `fab.tiles${axis}`)}
      />
    {/each}
  </Section>
</aside>

<style>
  .panel {
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    min-height: 0;
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

  .val {
    color: var(--text-primary);
  }

  .grow-right {
    margin-left: auto;
  }

  .import {
    align-self: flex-start;
  }

  .sub {
    margin: var(--s1) 0 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  .code {
    width: 100%;
    height: auto;
    min-height: 200px;
    padding: var(--s2);
    font: 11.5px/1.45 var(--mono);
    resize: vertical;
    white-space: pre;
    tab-size: 4;
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
    line-height: 1.45;
  }

  .shapes {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
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
