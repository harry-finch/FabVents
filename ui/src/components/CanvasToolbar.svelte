<script lang="ts">
  // Barre d'outils du canevas : outils (sélection, main, point, ligne,
  // courbe, image, règle), onglet affiché (canaux Taille, Espacement, Angle,
  // et tracés du Chemin), annuler / rétablir, zoom et cadrage.
  import { Hand, Maximize, MousePointer2, Redo2, Slash, Undo2, ZoomIn, ZoomOut, Crosshair, Spline, Image, Ruler } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import { pct } from '../lib/format';
  import { pickImage } from '../lib/files';
  import type { Tab, Tool } from '../lib/types';

  const TOOLS: { id: Tool; icon: typeof Hand; key: string }[] = [
    { id: 'select', icon: MousePointer2, key: 'V' },
    { id: 'hand', icon: Hand, key: 'H' },
    { id: 'point', icon: Crosshair, key: 'P' },
    { id: 'line', icon: Slash, key: 'L' },
    { id: 'curve', icon: Spline, key: 'C' },
  ];

  const count = (tab: Tab) => {
    const p = store.project;
    if (!p) return 0;
    if (tab === 'path') return p.layout.mode === 'path' ? p.layout.paths.length : 0;
    if (tab === 'plate') return p.features?.length ?? 0;
    return p.controllers.filter((c) => c[tab]).length;
  };
  // Zoom : 100 % = taille réelle à l'écran (96 px par pouce CSS).
  const zoomPct = $derived((store.view.scale / (96 / 25.4)) * 100);
  const KEYS: Record<Tab, string> = { size: '1', spacing: '2', angle: '3', path: '4', plate: '5', morph: '6' };

</script>

<div class="bar" role="toolbar" aria-label={t('canvas.toolbar')}>
  <div class="group" role="radiogroup" aria-label={t('canvas.tool')}>
    {#each TOOLS as tl (tl.id)}
      {@const Icon = tl.icon}
      <button
        type="button"
        role="radio"
        class="tool"
        class:on={store.tool === tl.id}
        aria-checked={store.tool === tl.id}
        aria-label={t(`canvas.tools.${tl.id}`)}
        use:tip={`${t(`canvas.tools.${tl.id}`)} (${tl.key})\n${t(`canvas.toolHints.${tl.id}`)}`}
        onclick={() => (store.tool = tl.id)}><Icon size={16} /></button
      >
    {/each}
    <button
      type="button"
      class="tool"
      aria-label={t('canvas.tools.image')}
      disabled={!store.channel}
      use:tip={`${t('canvas.tools.image')} (I)\n${t('canvas.toolHints.image')}`}
      onclick={pickImage}><Image size={16} /></button
    >
    <button
      type="button"
      role="radio"
      class="tool"
      class:on={store.tool === 'ruler'}
      aria-checked={store.tool === 'ruler'}
      aria-label={t('canvas.tools.ruler')}
      use:tip={`${t('canvas.tools.ruler')} (R)\n${t('canvas.toolHints.ruler')}`}
      onclick={() => (store.tool = 'ruler')}><Ruler size={16} /></button
    >
  </div>

  <span class="sep"></span>

  <div class="channels" role="tablist" aria-label={t('canvas.channel')}>
    {#each store.tabs as tab (tab)}
      <button
        type="button"
        role="tab"
        aria-selected={store.tab === tab}
        class:on={store.tab === tab}
        use:tip={`${t(`channels.hint.${tab}`)} (${KEYS[tab]})`}
        onclick={() => store.setTab(tab)}
      >
        {t(`channels.${tab}`)}
        {#if count(tab)}<span class="count tabular">{count(tab)}</span>{/if}
      </button>
    {/each}
  </div>

  <span class="grow"></span>

  <div class="group">
    <button type="button" class="tool" disabled={!store.canUndo} aria-label={t('canvas.undo')} use:tip={`${t('canvas.undo')} (⌘Z)`} onclick={() => store.undo()}><Undo2 size={16} /></button>
    <button type="button" class="tool" disabled={!store.canRedo} aria-label={t('canvas.redo')} use:tip={`${t('canvas.redo')} (⇧⌘Z)`} onclick={() => store.redo()}><Redo2 size={16} /></button>
  </div>
  <span class="sep"></span>
  <div class="group">
    <button type="button" class="tool" aria-label={t('canvas.zoomOut')} use:tip={t('canvas.zoomOut')} onclick={() => store.zoomBy(1 / 1.25)}><ZoomOut size={16} /></button>
    <span class="zoom tabular" use:tip={t('canvas.zoomHint')}>{pct(zoomPct)}</span>
    <button type="button" class="tool" aria-label={t('canvas.zoomIn')} use:tip={t('canvas.zoomIn')} onclick={() => store.zoomBy(1.25)}><ZoomIn size={16} /></button>
    <button type="button" class="tool" aria-label={t('canvas.fit')} use:tip={`${t('canvas.fit')} (F)`} onclick={() => store.fit()}><Maximize size={16} /></button>
  </div>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 38px;
    padding: 0 var(--s2);
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border);
    min-width: 0;
  }

  .group {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .tool {
    display: grid;
    place-items: center;
    width: 30px;
    height: 28px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-secondary);
  }

  .tool:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tool.on {
    background: var(--bg-selected);
    color: var(--accent-text);
  }

  .tool:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .sep {
    width: 1px;
    height: 18px;
    background: var(--border-strong);
  }

  .grow {
    flex: 1;
  }

  /* Onglets compacts, actif souligné. */
  .channels {
    display: flex;
    align-items: stretch;
    gap: var(--s3);
    height: 100%;
    padding-left: var(--s1);
  }

  .channels button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 2px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-md);
    font-weight: 500;
    white-space: nowrap;
  }

  .channels button:hover {
    color: var(--text-primary);
  }

  .channels button.on {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .count {
    min-width: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--neutral-tint);
    color: var(--text-secondary);
    font-size: var(--fs-xs);
    text-align: center;
  }

  .zoom {
    min-width: 48px;
    text-align: center;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
  }
</style>
