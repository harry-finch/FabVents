<script lang="ts">
  // Panneau droit : le contrôleur sélectionné (ou le canal affiché), puis les
  // mesures du motif et les messages du moteur.
  import { Copy, Trash2, Crosshair, Slash, TriangleAlert, CircleX, Info } from '@lucide/svelte';
  import Section from './Section.svelte';
  import NumField from './NumField.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { fmtArea, fmtLen, num, pct } from '../lib/format';
  import { issueText } from '../lib/issues';
  import { CHANNELS, type Channel, type Controller, type Influence } from '../lib/types';

  const sel = $derived(store.selectedController);
  const report = $derived(store.frame?.report ?? null);
  const m = $derived(report?.metrics ?? null);
  const issues = $derived(report?.issues ?? []);
  const thin = $derived(m != null && m.bridge_min != null && m.bridge_min < store.p.fabrication.min_bridge);

  function edit(fn: (c: Controller) => void, key: string) {
    const id = sel?.id;
    if (id == null) return;
    store.edit((p) => {
      const c = p.controllers.find((x) => x.id === id);
      if (c) fn(c);
    }, `ctrl.${id}.${key}`);
  }

  /** Modifie l'influence du contrôleur sélectionné sur un canal. */
  function editInf(ch: Channel, fn: (i: Influence) => void, key: string) {
    edit((c) => {
      const i = c[ch];
      if (i) fn(i);
    }, `${ch}.${key}`);
  }

  function toggleChannel(ch: Channel, on: boolean) {
    edit((c) => {
      if (on) c[ch] = store.defaultInfluence(ch);
      else delete c[ch];
    }, `${ch}.toggle`);
    if (!on && ch === store.channel) store.selected = null;
  }

  const baseOf = (ch: Channel) => (ch === 'size' ? store.p.shape.size : store.p.shape.rotation);
</script>

<aside class="panel" aria-label={t('panel.inspector')}>
  {#if sel}
    {@const g = sel.geometry}
    <Section id="controller" title={t('ctrl.title')}>
      {#snippet actions()}
        <button class="btn quiet icon" type="button" aria-label={t('ctrl.duplicate')} use:tip={t('ctrl.duplicate')} onclick={() => store.duplicateController(sel.id)}><Copy size={14} /></button>
        <button class="btn quiet icon" type="button" aria-label={t('ctrl.delete')} use:tip={`${t('ctrl.delete')} (⌫)`} onclick={() => store.removeController(sel.id)}><Trash2 size={14} /></button>
      {/snippet}
      <div class="kind">
        {#if g.type === 'point'}<Crosshair size={14} />{:else}<Slash size={14} />{/if}
        <span>{t(`ctrl.types.${g.type}`)} {sel.id}</span>
      </div>
      {#if g.type === 'point'}
        <NumField label="X" value={g.at.x} onchange={(v) => edit((c) => c.geometry.type === 'point' && (c.geometry.at.x = v), 'x')} />
        <NumField label="Y" value={g.at.y} onchange={(v) => edit((c) => c.geometry.type === 'point' && (c.geometry.at.y = v), 'y')} />
      {:else}
        <NumField label={t('ctrl.ax')} value={g.a.x} onchange={(v) => edit((c) => c.geometry.type === 'line' && (c.geometry.a.x = v), 'ax')} />
        <NumField label={t('ctrl.ay')} value={g.a.y} onchange={(v) => edit((c) => c.geometry.type === 'line' && (c.geometry.a.y = v), 'ay')} />
        <NumField label={t('ctrl.bx')} value={g.b.x} onchange={(v) => edit((c) => c.geometry.type === 'line' && (c.geometry.b.x = v), 'bx')} />
        <NumField label={t('ctrl.by')} value={g.b.y} onchange={(v) => edit((c) => c.geometry.type === 'line' && (c.geometry.b.y = v), 'by')} />
      {/if}
    </Section>

    {#each CHANNELS as ch (ch)}
      {@const inf = sel[ch]}
      <Section id={`controller.${ch}`} title={t(`channels.${ch}`)}>
        {#snippet actions()}
          <label class="switch" use:tip={t('ctrl.actsOnHint')}>
            <input type="checkbox" checked={!!inf} onchange={(e) => toggleChannel(ch, (e.currentTarget as HTMLInputElement).checked)} />
            <span>{t('ctrl.actsOn')}</span>
          </label>
        {/snippet}
        {#if inf}
          <NumField label={t('ctrl.reach')} hint={t('ctrl.reachHint')} field={`controllers.${sel.id}.${ch}.reach`} min={0} value={inf.reach} onchange={(v) => editInf(ch, (i) => (i.reach = v), 'reach')} />
          {#if ch === 'size'}
            <NumField label={t('ctrl.targetSize')} hint={t('ctrl.targetSizeHint')} field={`controllers.${sel.id}.size.target`} min={0} value={inf.target} onchange={(v) => editInf('size', (i) => (i.target = v), 'target')} />
          {:else}
            <NumField label={t('ctrl.targetAngle')} kind="angle" field={`controllers.${sel.id}.angle.target`} value={inf.target} onchange={(v) => editInf('angle', (i) => (i.target = v), 'target')} />
          {/if}
          <label class="check" use:tip={t('ctrl.hardEdgeHint')}>
            <input type="checkbox" checked={inf.hard_edge} onchange={(e) => { const on = (e.currentTarget as HTMLInputElement).checked; editInf(ch, (i) => (i.hard_edge = on), 'hard'); }} />
            {t('ctrl.hardEdge')}
          </label>
        {:else}
          <p class="muted">{t('ctrl.inactive')}</p>
        {/if}
      </Section>
    {/each}
  {:else}
    {@const ch = store.channel}
    <Section id={`channel.${ch}`} title={t(`channels.${ch}`)}>
      {#if ch === 'size'}
        <NumField label={t('channels.baseSize')} hint={t('channels.baseHint')} field="shape.size" min={0} value={store.p.shape.size} onchange={(v) => store.edit((q) => (q.shape.size = v), 'shape.size')} />
      {:else}
        <NumField label={t('channels.baseAngle')} hint={t('channels.baseHint')} kind="angle" field="shape.rotation" value={store.p.shape.rotation} onchange={(v) => store.edit((q) => (q.shape.rotation = v), 'shape.rotation')} />
      {/if}
      {#if store.channelControllers.length}
        <ul class="list">
          {#each store.channelControllers as c (c.id)}
            {@const inf = c[ch]!}
            <li>
              <button type="button" class="row" onclick={() => (store.selected = c.id)}>
                {#if c.geometry.type === 'point'}<Crosshair size={13} />{:else}<Slash size={13} />{/if}
                <span class="rname">{t(`ctrl.types.${c.geometry.type}`)} {c.id}</span>
                <span class="rval tabular">{ch === 'size' ? fmtLen(inf.target, store.units) : `${num(inf.target, 0)}°`}</span>
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

  .switch,
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    cursor: pointer;
  }

  .switch {
    font-size: var(--fs-xs);
  }

  input[type='checkbox'] {
    accent-color: var(--accent);
    margin: 0;
  }

  .muted {
    margin: 0;
    color: var(--text-muted);
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
