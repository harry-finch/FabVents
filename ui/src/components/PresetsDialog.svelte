<script lang="ts">
  // Préréglages : enregistrer le motif en cours sous un nom, l'appliquer à
  // une autre plaque (adapté à ses proportions), renommer, supprimer,
  // exporter la bibliothèque ou importer celle d'un autre poste.
  import { Bookmark, Download, Trash2, Upload, X as Close } from '@lucide/svelte';
  import { store } from '../lib/store.svelte';
  import { presets } from '../lib/presets.svelte';
  import { deliver } from '../lib/files';
  import { intlLocale, t } from '../lib/i18n.svelte';
  import { tip } from '../lib/tip';
  import type { Project } from '../lib/types';

  let dialog = $state<HTMLDialogElement>();
  $effect(() => dialog?.showModal());

  let name = $state(store.p.name);
  const date = (iso: string) => new Date(iso).toLocaleDateString(intlLocale(), { day: '2-digit', month: '2-digit', year: 'numeric' });
  const summary = (p: Project) =>
    `${t(`layout.modes.${p.layout.mode}`)} · ${p.shape.kind.type === 'custom' ? t('shape.types.custom') : t(`shape.types.${p.shape.kind.type}`)}${p.controllers.length ? ` · ${t('presets.controllers', { n: p.controllers.length })}` : ''}`;

  function save() {
    presets.add(name, $state.snapshot(store.p) as Project);
    store.toast(t('presets.saved', { name: name.trim() || store.p.name }));
  }

  function exportAll() {
    void deliver(presets.exportJson(), 'fabvents-presets.json', 'application/json');
  }

  function importFile() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return;
      const n = presets.importJson(await f.text());
      store.toast(n == null ? t('presets.notPresets', { name: f.name }) : t('presets.imported', { n }));
    };
    input.click();
  }
</script>

<dialog bind:this={dialog} class="dlg" aria-labelledby="presets-title" onclose={() => (store.presetsOpen = false)}>
  <header>
    <h2 id="presets-title">{t('presets.title')}</h2>
    <button class="btn quiet icon" type="button" aria-label={t('help.close')} onclick={() => dialog?.close()}><Close size={16} /></button>
  </header>
  <div class="body">
    <p class="lead">{t('presets.intro')}</p>
    <div class="save">
      <input class="input" type="text" aria-label={t('presets.name')} placeholder={t('presets.name')} bind:value={name} onkeydown={(e) => e.key === 'Enter' && save()} />
      <button class="btn primary" type="button" onclick={save}><Bookmark size={14} /> {t('presets.save')}</button>
    </div>

    <h3>{t('presets.library')}</h3>
    {#if presets.list.length}
      <ul class="list">
        {#each presets.list as pr (pr.id)}
          <li>
            <span class="txt">
              <input class="rname" type="text" aria-label={t('presets.name')} value={pr.name} onchange={(e) => presets.rename(pr.id, (e.currentTarget as HTMLInputElement).value)} />
              <span class="desc">{summary(pr.project)} · {date(pr.saved)}</span>
            </span>
            <button
              class="btn"
              type="button"
              use:tip={t('presets.applyHint')}
              onclick={() => {
                store.applyPreset(pr.project, pr.name);
                dialog?.close();
              }}>{t('presets.apply')}</button
            >
            <button class="btn quiet icon" type="button" aria-label={t('presets.delete')} use:tip={t('presets.delete')} onclick={() => presets.remove(pr.id)}><Trash2 size={14} /></button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">{t('presets.empty')}</p>
    {/if}
  </div>
  <footer>
    <button class="btn" type="button" use:tip={t('presets.importHint')} onclick={importFile}><Upload size={14} /> {t('presets.import')}</button>
    <button class="btn" type="button" disabled={!presets.list.length} onclick={exportAll}><Download size={14} /> {t('presets.export')}</button>
  </footer>
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

  header,
  footer {
    display: flex;
    align-items: center;
    padding: var(--s3) var(--s4);
  }

  header {
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
  }

  footer {
    justify-content: flex-end;
    gap: var(--s2);
    border-top: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: var(--fs-lg);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4) var(--s4);
    max-height: 70vh;
    overflow: auto;
  }

  .lead,
  .muted {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--fs-sm);
    line-height: 1.5;
  }

  .muted {
    color: var(--text-muted);
  }

  .save {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--s2);
  }

  h3 {
    margin: var(--s3) 0 0;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--s1) var(--s1) var(--s1) 0;
    border-bottom: 1px solid var(--divider);
  }

  .txt {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .rname {
    height: 22px;
    padding: 0 var(--s1);
    margin-left: calc(-1 * var(--s1));
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--text-primary);
    font-weight: 500;
  }

  .rname:hover {
    border-color: var(--border);
  }

  .rname:focus {
    border-color: var(--accent);
    outline: none;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
