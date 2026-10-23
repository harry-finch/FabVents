<script lang="ts">
  // Panneau droit : l'élément sélectionné (contrôleur ou tracé de chemin),
  // ou le canal affiché ; puis les mesures du motif et les messages du moteur.
  import { Copy, Trash2, TriangleAlert, CircleX, Info, Dices, Equal, Circle, Hexagon, Ellipse } from '@lucide/svelte';
  import Section from './Section.svelte';
  import NumField from './NumField.svelte';
  import Field from './Field.svelte';
  import Segmented from './Segmented.svelte';
  import GeometryIcon from './GeometryIcon.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { fmtArea, fmtLen, num, pct } from '../lib/format';
  import { issueText } from '../lib/issues';
  import { isClosed, isOpenPath } from '../lib/geometry';
  import { baseSpacing, type Channel, type Controller, type CtrlGeometry, type CullMode, type Influence, type Side } from '../lib/types';

  const key = $derived(store.selection);
  const geom = $derived(store.selectedGeometry);
  const ctrl = $derived(store.selectedController);
  const report = $derived(store.frame?.report ?? null);
  const m = $derived(report?.metrics ?? null);
  const issues = $derived(report?.issues ?? []);
  const thin = $derived(m != null && m.bridge_min != null && m.bridge_min < store.p.fabrication.min_bridge);
  const channels = $derived(store.tabs.filter((x): x is Channel => x !== 'path'));
  const editKey = $derived(key ? (key.kind === 'ctrl' ? `c${key.id}` : `p${key.index}`) : '');

  /** Modifie la géométrie de l'élément sélectionné. */
  function setGeom(fn: (g: CtrlGeometry) => CtrlGeometry, field: string) {
    if (!key || !geom) return;
    store.setGeometry(key, fn($state.snapshot(geom) as CtrlGeometry), `${editKey}.${field}`);
  }

  function editCtrl(fn: (c: Controller) => void, field: string) {
    const id = ctrl?.id;
    if (id == null) return;
    store.edit((p) => {
      const c = p.controllers.find((x) => x.id === id);
      if (c) fn(c);
    }, `c${id}.${field}`);
  }

  function editInf(ch: Channel, fn: (i: Influence) => void, field: string) {
    editCtrl((c) => {
      const i = c[ch];
      if (i) fn(i);
    }, `${ch}.${field}`);
  }

  function toggleChannel(ch: Channel, on: boolean) {
    editCtrl((c) => {
      if (on) {
        c[ch] = store.defaultInfluence(ch);
        if (c.geometry.type === 'image') c[ch]!.reach = 1;
      } else delete c[ch];
    }, `${ch}.toggle`);
    if (!on && ch === store.channel) store.select(null);
  }

  const baseOf = (ch: Channel) =>
    ch === 'size' ? store.p.shape.size : ch === 'angle' ? store.p.shape.rotation : (baseSpacing(store.p.layout) ?? 0);
  function setBase(ch: Channel, v: number) {
    store.edit((q) => {
      if (ch === 'size') q.shape.size = v;
      else if (ch === 'angle') q.shape.rotation = v;
      else if (q.layout.mode !== 'grid' && q.layout.mode !== 'cross_hatch') q.layout.spacing = v;
    }, `base.${ch}`);
  }

  const fillets = $derived(geom?.type === 'curve' ? (geom.fillets ?? []) : []);
  const allFillet = $derived(fillets.length && fillets.every((r) => r === fillets[0]) ? fillets[0] : 0);
  const node = $derived(geom?.type === 'curve' && store.node != null && store.node < geom.points.length ? store.node : null);
</script>

<aside class="panel" aria-label={t('panel.inspector')}>
  {#if key && geom}
    {@const g = geom}
    <Section id="item" title={key.kind === 'path' ? t('ctrl.pathTitle') : t('ctrl.title')}>
      {#snippet actions()}
        <button class="btn quiet icon" type="button" aria-label={t('ctrl.duplicate')} use:tip={t('ctrl.duplicate')} onclick={() => store.duplicateSelected()}><Copy size={14} /></button>
        <button class="btn quiet icon" type="button" aria-label={t('ctrl.delete')} use:tip={`${t('ctrl.delete')} (⌫)`} onclick={() => store.removeSelected()}><Trash2 size={14} /></button>
      {/snippet}
      <div class="kind">
        <GeometryIcon type={g.type} />
        <span>{key.kind === 'path' ? t('ctrl.pathName', { n: key.index + 1 }) : `${t(`ctrl.types.${g.type}`)} ${key.id}`}</span>
      </div>

      {#if g.type === 'point'}
        <NumField label="X" value={g.at.x} onchange={(v) => setGeom((x) => (x.type === 'point' ? { ...x, at: { ...x.at, x: v } } : x), 'x')} />
        <NumField label="Y" value={g.at.y} onchange={(v) => setGeom((x) => (x.type === 'point' ? { ...x, at: { ...x.at, y: v } } : x), 'y')} />
      {:else if g.type === 'line'}
        <NumField label={t('ctrl.ax')} value={g.a.x} onchange={(v) => setGeom((x) => (x.type === 'line' ? { ...x, a: { ...x.a, x: v } } : x), 'ax')} />
        <NumField label={t('ctrl.ay')} value={g.a.y} onchange={(v) => setGeom((x) => (x.type === 'line' ? { ...x, a: { ...x.a, y: v } } : x), 'ay')} />
        <NumField label={t('ctrl.bx')} value={g.b.x} onchange={(v) => setGeom((x) => (x.type === 'line' ? { ...x, b: { ...x.b, x: v } } : x), 'bx')} />
        <NumField label={t('ctrl.by')} value={g.b.y} onchange={(v) => setGeom((x) => (x.type === 'line' ? { ...x, b: { ...x.b, y: v } } : x), 'by')} />
      {:else if g.type === 'curve'}
        <Field label={t('ctrl.nodes')}>
          <span class="val tabular">{g.points.length}</span>
          <label class="check grow-right" use:tip={t('ctrl.closedHint')}>
            <input
              type="checkbox"
              checked={g.closed}
              disabled={!g.closed && g.points.length < 3}
              onchange={(e) => {
                const on = (e.currentTarget as HTMLInputElement).checked;
                setGeom((x) => (x.type === 'curve' ? { ...x, closed: on } : x), 'closed');
              }}
            />
            {t('ctrl.closed')}
          </label>
        </Field>
        <NumField
          label={t('ctrl.filletAll')}
          hint={t('ctrl.filletHint')}
          min={0}
          value={allFillet}
          onchange={(v) => setGeom((x) => (x.type === 'curve' ? { ...x, fillets: x.points.map(() => v) } : x), 'fillets')}
        />
        {#if node != null}
          {@const i = node}
          <p class="sub">{t('ctrl.node', { n: i + 1 })}</p>
          <NumField label="X" value={g.points[i].x} onchange={(v) => setGeom((x) => (x.type === 'curve' ? { ...x, points: x.points.map((p, k) => (k === i ? { ...p, x: v } : p)) } : x), `n${i}x`)} />
          <NumField label="Y" value={g.points[i].y} onchange={(v) => setGeom((x) => (x.type === 'curve' ? { ...x, points: x.points.map((p, k) => (k === i ? { ...p, y: v } : p)) } : x), `n${i}y`)} />
          <NumField
            label={t('ctrl.fillet')}
            min={0}
            value={fillets[i] ?? 0}
            onchange={(v) =>
              setGeom((x) => (x.type === 'curve' ? { ...x, fillets: x.points.map((_, k) => (k === i ? v : (x.fillets?.[k] ?? 0))) } : x), `n${i}r`)}
          />
        {:else}
          <p class="muted">{t('ctrl.nodeHint')}</p>
        {/if}
        {#if g.closed && key.kind === 'ctrl'}
          <Field label={t('ctrl.convert')}>
            <div class="convert">
              <button class="btn" type="button" use:tip={t('ctrl.toCircle')} aria-label={t('ctrl.toCircle')} onclick={() => store.convertSelected('circle')}><Circle size={14} /></button>
              <button class="btn" type="button" use:tip={t('ctrl.toEllipse')} aria-label={t('ctrl.toEllipse')} onclick={() => store.convertSelected('ellipse')}><Ellipse size={14} /></button>
              <button class="btn" type="button" use:tip={t('ctrl.toPolygon')} aria-label={t('ctrl.toPolygon')} onclick={() => store.convertSelected('polygon')}><Hexagon size={14} /></button>
            </div>
          </Field>
        {/if}
      {:else if g.type === 'circle'}
        <NumField label={t('ctrl.cx')} value={g.center.x} onchange={(v) => setGeom((x) => (x.type === 'circle' ? { ...x, center: { ...x.center, x: v } } : x), 'cx')} />
        <NumField label={t('ctrl.cy')} value={g.center.y} onchange={(v) => setGeom((x) => (x.type === 'circle' ? { ...x, center: { ...x.center, y: v } } : x), 'cy')} />
        <NumField label={t('ctrl.radius')} field={`controllers.${ctrl?.id}.geometry.radius`} min={0} value={g.radius} onchange={(v) => setGeom((x) => (x.type === 'circle' ? { ...x, radius: v } : x), 'r')} />
      {:else if g.type === 'ellipse'}
        <NumField label={t('ctrl.cx')} value={g.center.x} onchange={(v) => setGeom((x) => (x.type === 'ellipse' ? { ...x, center: { ...x.center, x: v } } : x), 'cx')} />
        <NumField label={t('ctrl.cy')} value={g.center.y} onchange={(v) => setGeom((x) => (x.type === 'ellipse' ? { ...x, center: { ...x.center, y: v } } : x), 'cy')} />
        <NumField label={t('ctrl.rx')} min={0} value={g.rx} onchange={(v) => setGeom((x) => (x.type === 'ellipse' ? { ...x, rx: v } : x), 'rx')} />
        <NumField label={t('ctrl.ry')} min={0} value={g.ry} onchange={(v) => setGeom((x) => (x.type === 'ellipse' ? { ...x, ry: v } : x), 'ry')} />
        <NumField label={t('ctrl.angle')} kind="angle" value={g.angle} onchange={(v) => setGeom((x) => (x.type === 'ellipse' ? { ...x, angle: v } : x), 'a')} />
      {:else if g.type === 'polygon'}
        <NumField label={t('ctrl.cx')} value={g.center.x} onchange={(v) => setGeom((x) => (x.type === 'polygon' ? { ...x, center: { ...x.center, x: v } } : x), 'cx')} />
        <NumField label={t('ctrl.cy')} value={g.center.y} onchange={(v) => setGeom((x) => (x.type === 'polygon' ? { ...x, center: { ...x.center, y: v } } : x), 'cy')} />
        <NumField label={t('ctrl.radius')} min={0} value={g.radius} onchange={(v) => setGeom((x) => (x.type === 'polygon' ? { ...x, radius: v } : x), 'r')} />
        <NumField label={t('shape.sides')} kind="count" min={3} max={64} value={g.sides} onchange={(v) => setGeom((x) => (x.type === 'polygon' ? { ...x, sides: Math.round(v) } : x), 's')} />
        <NumField label={t('ctrl.angle')} kind="angle" value={g.angle} onchange={(v) => setGeom((x) => (x.type === 'polygon' ? { ...x, angle: v } : x), 'a')} />
      {:else if g.type === 'image'}
        <NumField label="X" value={g.at.x} onchange={(v) => setGeom((x) => (x.type === 'image' ? { ...x, at: { ...x.at, x: v } } : x), 'x')} />
        <NumField label="Y" value={g.at.y} onchange={(v) => setGeom((x) => (x.type === 'image' ? { ...x, at: { ...x.at, y: v } } : x), 'y')} />
        <NumField
          label={t('plate.width')}
          min={0.1}
          value={g.width}
          onchange={(v) => setGeom((x) => (x.type === 'image' ? { ...x, width: v, height: (x.height * v) / x.width } : x), 'w')}
        />
        <NumField
          label={t('plate.height')}
          min={0.1}
          value={g.height}
          onchange={(v) => setGeom((x) => (x.type === 'image' ? { ...x, height: v, width: (x.width * v) / x.height } : x), 'h')}
        />
        <p class="muted">{t('ctrl.imageInfo', { w: g.raster.width, h: g.raster.height })}</p>
      {/if}

      {#if ctrl && isOpenPath(g)}
        <Segmented
          label={t('ctrl.side')}
          value={ctrl.side ?? 'both'}
          options={[
            { id: 'both', label: t('ctrl.sides.both'), hint: t('ctrl.sideHint') },
            { id: 'left', label: t('ctrl.sides.left'), hint: t('ctrl.sideHint') },
            { id: 'right', label: t('ctrl.sides.right'), hint: t('ctrl.sideHint') },
          ]}
          onchange={(v: Side) => editCtrl((c) => (c.side = v === 'both' ? undefined : v), 'side')}
        />
        <label class="check" use:tip={t('ctrl.extendHint')}>
          <input
            type="checkbox"
            checked={!!ctrl.extend}
            onchange={(e) => {
              const on = (e.currentTarget as HTMLInputElement).checked;
              editCtrl((c) => (c.extend = on || undefined), 'extend');
            }}
          />
          {t('ctrl.extend')}
        </label>
      {:else if ctrl && isClosed(g)}
        <label class="check" use:tip={t('ctrl.fillHint')}>
          <input
            type="checkbox"
            checked={!!ctrl.fill}
            onchange={(e) => {
              const on = (e.currentTarget as HTMLInputElement).checked;
              editCtrl((c) => (c.fill = on || undefined), 'fill');
            }}
          />
          {t('ctrl.fill')}
        </label>
      {/if}
    </Section>

    {#if ctrl}
      {#each channels as ch (ch)}
        {@const inf = ctrl[ch]}
        {@const image = ctrl.geometry.type === 'image'}
        <Section id={`controller.${ch}`} title={t(`channels.${ch}`)}>
          {#snippet actions()}
            <label class="check small" use:tip={t('ctrl.actsOnHint')}>
              <input type="checkbox" checked={!!inf} onchange={(e) => toggleChannel(ch, (e.currentTarget as HTMLInputElement).checked)} />
              {t('ctrl.actsOn')}
            </label>
          {/snippet}
          {#if inf}
            {#if !image}
              <div class="with-action">
                <NumField label={t('ctrl.reach')} hint={t('ctrl.reachHint')} field={`controllers.${ctrl.id}.${ch}.reach`} min={0} value={inf.reach} onchange={(v) => editInf(ch, (i) => (i.reach = v), 'reach')} />
                <button class="btn quiet icon" type="button" aria-label={t('ctrl.unifyReach')} use:tip={t('ctrl.unifyReach')} onclick={() => store.unify(ch, 'reach', inf.reach)}><Equal size={14} /></button>
              </div>
            {/if}
            <div class="with-action">
              <NumField
                label={t(`ctrl.target.${ch}`)}
                hint={t(`ctrl.targetHint.${ch}`)}
                kind={ch === 'angle' ? 'angle' : 'length'}
                field={`controllers.${ctrl.id}.${ch}.target`}
                min={ch === 'angle' ? -Infinity : 0}
                value={inf.target}
                onchange={(v) => editInf(ch, (i) => (i.target = v), 'target')}
              />
              <button class="btn quiet icon" type="button" aria-label={t('ctrl.unifyTarget')} use:tip={t('ctrl.unifyTarget')} onclick={() => store.unify(ch, 'target', inf.target)}><Equal size={14} /></button>
            </div>
            {#if image}
              <NumField label={t('ctrl.contrast')} hint={t('ctrl.contrastHint')} kind="plain" step={0.1} min={0} max={20} value={inf.contrast ?? 1} onchange={(v) => editInf(ch, (i) => (i.contrast = v), 'contrast')} />
              <label class="check" use:tip={t('ctrl.invertHint')}>
                <input
                  type="checkbox"
                  checked={!!inf.invert}
                  onchange={(e) => {
                    const on = (e.currentTarget as HTMLInputElement).checked;
                    editInf(ch, (i) => (i.invert = on || undefined), 'invert');
                  }}
                />
                {t('ctrl.invert')}
              </label>
            {/if}
            <label class="check" use:tip={image ? t('ctrl.thresholdHint') : t('ctrl.hardEdgeHint')}>
              <input
                type="checkbox"
                checked={inf.hard_edge}
                onchange={(e) => {
                  const on = (e.currentTarget as HTMLInputElement).checked;
                  editInf(ch, (i) => (i.hard_edge = on), 'hard');
                }}
              />
              {image ? t('ctrl.threshold') : t('ctrl.hardEdge')}
            </label>
            {#if ch === 'angle'}
              <label class="check" use:tip={t('ctrl.symmetricHint')}>
                <input
                  type="checkbox"
                  checked={!!inf.symmetric}
                  onchange={(e) => {
                    const on = (e.currentTarget as HTMLInputElement).checked;
                    editInf(ch, (i) => (i.symmetric = on || undefined), 'sym');
                  }}
                />
                {t('ctrl.symmetric')}
              </label>
              <NumField label={t('ctrl.step')} hint={t('ctrl.stepHint')} kind="angle" min={0} max={180} value={inf.step ?? 0} onchange={(v) => editInf(ch, (i) => (i.step = v || undefined), 'step')} />
            {/if}
            {#if ch === 'size'}
              <label class="check" use:tip={t('ctrl.cullHint')}>
                <input
                  type="checkbox"
                  checked={!!inf.cull}
                  onchange={(e) => {
                    const on = (e.currentTarget as HTMLInputElement).checked;
                    editInf(ch, (i) => (i.cull = on ? { amount: 0.3, mode: 'center', seed: 1 } : undefined), 'cull');
                  }}
                />
                {t('ctrl.cull')}
              </label>
              {#if inf.cull}
                {@const cull = inf.cull}
                <NumField
                  label={t('ctrl.cullAmount')}
                  kind="percent"
                  min={0}
                  max={1}
                  decimals={0}
                  field={`controllers.${ctrl.id}.size.cull`}
                  value={cull.amount}
                  onchange={(v) =>
                    editInf(
                      ch,
                      (i) => {
                        if (i.cull) i.cull.amount = v;
                      },
                      'cullAmount',
                    )}
                />
                <Segmented
                  label={t('ctrl.cullMode')}
                  value={cull.mode}
                  options={[
                    { id: 'center', label: t('ctrl.cullModes.center'), hint: t('ctrl.cullModes.centerHint') },
                    { id: 'edge', label: t('ctrl.cullModes.edge'), hint: t('ctrl.cullModes.edgeHint') },
                    { id: 'uniform', label: t('ctrl.cullModes.uniform'), hint: t('ctrl.cullModes.uniformHint') },
                  ]}
                  onchange={(v: CullMode) =>
                    editInf(
                      ch,
                      (i) => {
                        if (i.cull) i.cull.mode = v;
                      },
                      'cullMode',
                    )}
                />
                <Field label={t('ctrl.cullSeed')}>
                  <span class="val tabular">{cull.seed}</span>
                  <button
                    class="btn quiet grow-right"
                    type="button"
                    use:tip={t('ctrl.reroll')}
                    onclick={() =>
                      editInf(
                        ch,
                        (i) => {
                          if (i.cull) i.cull.seed = (i.cull.seed % 9999) + 1;
                        },
                        'cullSeed',
                      )}><Dices size={14} /> {t('ctrl.rerollShort')}</button
                  >
                </Field>
              {/if}
            {/if}
          {:else}
            <p class="muted">{t('ctrl.inactive')}</p>
          {/if}
        </Section>
      {/each}
    {/if}
  {:else if store.tab === 'path'}
    <Section id="paths" title={t('channels.path')}>
      {#if store.items.length}
        <ul class="list">
          {#each store.items as it, n (n)}
            <li>
              <button type="button" class="row" onclick={() => store.select(it.key)}>
                <GeometryIcon type="curve" size={13} />
                <span class="rname">{t('ctrl.pathName', { n: n + 1 })}</span>
                <span class="rval tabular">{it.geometry.type === 'curve' ? t('ctrl.nodesCount', { n: it.geometry.points.length }) : ''}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">{t('canvas.empty.path')}</p>
      {/if}
    </Section>
  {:else if store.channel}
    {@const ch = store.channel}
    <Section id={`channel.${ch}`} title={t(`channels.${ch}`)}>
      <NumField
        label={t(`channels.base.${ch}`)}
        hint={t('channels.baseHint')}
        kind={ch === 'angle' ? 'angle' : 'length'}
        min={ch === 'angle' ? -Infinity : 0}
        field={ch === 'size' ? 'shape.size' : ch === 'angle' ? 'shape.rotation' : 'layout.spacing'}
        value={baseOf(ch)}
        onchange={(v) => setBase(ch, v)}
      />
      {#if store.items.length}
        <ul class="list">
          {#each store.items as it, n (n)}
            {@const inf = it.influence!}
            <li>
              <button type="button" class="row" onclick={() => store.select(it.key)}>
                <GeometryIcon type={it.geometry.type} size={13} />
                <span class="rname">{t(`ctrl.types.${it.geometry.type}`)} {it.key.kind === 'ctrl' ? it.key.id : ''}</span>
                <span class="rval tabular">{ch === 'angle' ? `${num(inf.target, 0)}°` : fmtLen(inf.target, store.units)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">{t(`canvas.empty.${ch}`)}</p>
      {/if}
    </Section>
  {/if}

  <Section id="metrics" title={t('metrics.title')}>
    {#if m}
      <dl class="metrics">
        <dt>{t('metrics.holes')}</dt>
        <dd class="tabular">{num(m.holes, 0)}</dd>
        <dt>{t('metrics.openRatio')}</dt>
        <dd class="tabular strong" class:warn={m.overlaps > 0} use:tip={m.overlaps > 0 ? t('metrics.openRatioOverlap') : t('metrics.openRatioHint')}>{pct(m.open_ratio, 1)}</dd>
        <dt>{t('metrics.openArea')}</dt>
        <dd class="tabular">{fmtArea(m.open_area, store.units)}</dd>
        <dt>{t('metrics.size')}</dt>
        <dd class="tabular">
          {#if m.size_min != null && m.size_max != null}
            {m.size_max - m.size_min < 0.005 ? fmtLen(m.size_min, store.units) : `${fmtLen(m.size_min, store.units)} – ${fmtLen(m.size_max, store.units)}`}
          {:else}—{/if}
        </dd>
        <dt>{t('metrics.bridgeMin')}</dt>
        <dd class="tabular" class:warn={thin || m.overlaps > 0}>
          {#if m.overlaps > 0}{t('metrics.overlap')}{:else if m.bridge_min != null}{fmtLen(m.bridge_min, store.units)}{:else}&gt; {fmtLen(m.bridge_search, store.units, 0)}{/if}
        </dd>
        {#if report && report.stats.removed_cull > 0}
          <dt>{t('metrics.culled')}</dt>
          <dd class="tabular">{num(report.stats.removed_cull, 0)}</dd>
        {/if}
      </dl>
    {:else}
      <p class="muted">{t('status.loading')}</p>
    {/if}
  </Section>

  {#if issues.length}
    <Section id="issues" title={t('issues.title')}>
      <ul class="issues">
        {#each issues as i, k (k)}
          <li class={i.severity}>
            {#if i.severity === 'error'}<CircleX size={14} />{:else if i.severity === 'warning'}<TriangleAlert size={14} />{:else}<Info size={14} />{/if}
            <span>{issueText(i)}</span>
          </li>
        {/each}
      </ul>
    </Section>
  {/if}
</aside>

<style>
  .panel {
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    overflow-y: auto;
    min-height: 0;
  }

  .kind {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-primary);
    font-weight: 600;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    cursor: pointer;
  }

  .check.small {
    font-size: var(--fs-xs);
  }

  .grow-right {
    margin-left: auto;
  }

  input[type='checkbox'] {
    accent-color: var(--accent);
    margin: 0;
  }

  .val {
    color: var(--text-primary);
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .sub {
    margin: var(--s1) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .with-action {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 2px;
  }

  .with-action .btn.icon {
    width: 24px;
    height: 24px;
  }

  .convert {
    display: flex;
    gap: var(--s1);
  }

  .convert .btn {
    padding: 0 var(--s2);
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

  .rname {
    flex: 1;
  }

  .rval {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .metrics {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px var(--s2);
    margin: 0;
  }

  dt {
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }

  dd {
    margin: 0;
    text-align: right;
  }

  dd.strong {
    font-weight: 600;
    font-size: var(--fs-lg);
  }

  dd.warn {
    color: var(--warning);
  }

  .issues {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .issues li {
    display: grid;
    grid-template-columns: 14px 1fr;
    gap: 6px;
    align-items: start;
    font-size: var(--fs-sm);
    color: var(--text-secondary);
  }

  .issues li :global(svg) {
    margin-top: 1px;
  }

  .issues li.error :global(svg) {
    color: var(--danger);
  }

  .issues li.error {
    color: var(--danger-text);
  }

  .issues li.warning :global(svg) {
    color: var(--warning);
  }

  .issues li.info :global(svg) {
    color: var(--info);
  }
</style>
