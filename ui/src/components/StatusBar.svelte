<script lang="ts">
  // Barre d'état : moteur, temps de calcul, perforations, position du curseur.
  import { store } from '../lib/store.svelte';
  import { fmtLength, lengthUnit, num } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  const f = $derived(store.frame);
</script>

<footer class="status">
  <span class="dot" class:off={store.engineState === 'failed'} class:wait={store.engineState === 'loading'}></span>
  <span>{t(`status.${store.engineState}`)}</span>
  {#if store.engineVersion}
    <span class="sep"></span>
    <span>{t('status.engine', { v: store.engineVersion })}</span>
  {/if}
  {#if f}
    <span class="sep"></span>
    <span class="tabular">{t('status.holes', { n: num(f.report.metrics.holes, 0) })}</span>
    <span class="sep"></span>
    <span class="tabular">{t('status.compute', { ms: num(f.ms, 1) })}</span>
    {#if store.deferred}
      <span class="sep"></span>
      <span class="deferred">{t('status.deferred')}</span>
    {/if}
  {/if}
  <span class="grow"></span>
  {#if store.cursor}
    <span class="tabular">X {fmtLength(store.cursor.x, store.units, 1)} · Y {fmtLength(store.cursor.y, store.units, 1)} {lengthUnit(store.units)}</span>
  {/if}
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--s2);
    height: 24px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    font-size: var(--fs-xs);
    white-space: nowrap;
    overflow: hidden;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--success);
  }

  .dot.wait {
    background: var(--warning);
  }

  .dot.off {
    background: var(--danger);
  }

  .sep {
    width: 1px;
    height: 12px;
    background: var(--border-strong);
  }

  .grow {
    flex: 1;
  }

  .deferred {
    color: var(--warning);
  }
</style>
