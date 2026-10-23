<script lang="ts">
  // Aide : prise en main en quelques lignes et raccourcis.
  import { X } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';

  const SHORTCUTS: [string, string][] = [
    ['V', 'help.keys.select'],
    ['H', 'help.keys.hand'],
    ['P', 'help.keys.point'],
    ['L', 'help.keys.line'],
    ['C', 'help.keys.curve'],
    ['I', 'help.keys.image'],
    ['R', 'help.keys.ruler'],
    ['1 – 4', 'help.keys.channel'],
    ['F', 'help.keys.fit'],
    ['⌫', 'help.keys.delete'],
    ['← ↑ → ↓', 'help.keys.nudge'],
    [t('help.nodes'), 'help.keys.nodes'],
    [t('help.context'), 'help.keys.context'],
    ['⌘Z / ⇧⌘Z', 'help.keys.undo'],
    ['⌘O / ⌘S', 'help.keys.file'],
    [t('help.wheel'), 'help.keys.zoom'],
    [t('help.space'), 'help.keys.pan'],
    [t('help.scrub'), 'help.keys.scrub'],
  ];
  let dialog = $state<HTMLDialogElement>();
  $effect(() => dialog?.showModal());
</script>

<dialog bind:this={dialog} class="dlg" aria-label={t('help.title')} onclose={() => (store.help = false)}>
  <header>
    <h2>{t('help.title')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('help.close')} onclick={() => dialog?.close()}><X size={16} /></button>
  </header>
  <div class="body">
    <ol class="steps">
      <li>{t('help.steps.pattern')}</li>
      <li>{t('help.steps.controllers')}</li>
      <li>{t('help.steps.check')}</li>
      <li>{t('help.steps.export')}</li>
    </ol>
    <h3>{t('help.shortcuts')}</h3>
    <dl class="keys">
      {#each SHORTCUTS as [k, label] (label)}
        <dt><kbd>{k}</kbd></dt>
        <dd>{t(label as never)}</dd>
      {/each}
    </dl>
  </div>
</dialog>

<style>
  .dlg {
    width: min(560px, calc(100vw - 32px));
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
    padding: var(--s3) var(--s3) 0 var(--s4);
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  h3 {
    margin: var(--s4) 0 var(--s2);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .body {
    padding: var(--s2) var(--s4) var(--s4);
  }

  .steps {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--text-secondary);
  }

  .keys {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px var(--s3);
    margin: 0;
  }

  dd {
    margin: 0;
    color: var(--text-secondary);
  }

  kbd {
    display: inline-block;
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg-control);
    font: 500 var(--fs-sm) var(--font);
    color: var(--text-primary);
  }
</style>
