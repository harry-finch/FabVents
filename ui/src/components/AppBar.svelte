<script lang="ts">
  // Barre d'application : identité, projet (nom et menu Fichier), unités,
  // langue, thème, aide, et l'action principale : exporter.
  import { ChevronDown, CircleHelp, Download, FilePlus, FolderOpen, Monitor, Moon, Save, Sparkles, Sun, FileImage, FileCode, FileBraces, FileText, Fan, ImageDown } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import { theme, THEMES } from '../lib/theme.svelte';
  import { store } from '../lib/store.svelte';
  import { tip } from '../lib/tip';
  import { i18n, LOCALES, t } from '../lib/i18n.svelte';
  import { exportAs, exportPng, pickFile } from '../lib/files';
  import { FAN_SIZES } from '../lib/types';

  // Texte en cours de saisie : l'encart suit sa longueur (voir `.sizer`).
  let draft = $state<string | null>(null);
  const shown = $derived(draft ?? store.p.name);

  const blocked = $derived(!!store.frame?.report.issues.some((i) => i.severity === 'error'));
</script>

<header class="bar">
  <div class="brand">
    <img class="logo" src="/fabvents-logo.svg" alt="" width="24" height="24" />
    <span class="name">FabVents</span>
    <span class="tag">{t('app.tagline')}</span>
  </div>

  <div class="project">
    <Popover label={t('file.menu')} width={260}>
      {#snippet trigger({ open, toggle })}
        <button class="fbtn" class:open type="button" onclick={toggle} aria-haspopup="menu" aria-expanded={open}>
          {t('file.menu')}
          <ChevronDown size={13} />
        </button>
      {/snippet}
      {#snippet content({ close })}
        <button class="row" type="button" onclick={() => { close(); store.newProject(false); }}><FilePlus size={14} /><span class="rname">{t('file.new')}</span></button>
        <button class="row" type="button" onclick={() => { close(); store.newProject(true); }}><Sparkles size={14} /><span class="rname">{t('file.example')}</span></button>
        <p class="sub">{t('templates.fan')}</p>
        <div class="fans">
          {#each FAN_SIZES as size (size)}
            <button class="chip tabular" type="button" use:tip={t('templates.fanHint', { size })} onclick={() => { close(); void store.applyTemplate(size); }}><Fan size={12} /> {size}</button>
          {/each}
        </div>
        <div class="divider"></div>
        <button class="row" type="button" onclick={() => { close(); pickFile(); }}><FolderOpen size={14} /><span class="rname">{t('file.open')}</span><span class="kbd">⌘O</span></button>
        <button class="row" type="button" onclick={() => { close(); void exportAs('project'); }}><Save size={14} /><span class="rname">{t('file.save')}</span><span class="kbd">⌘S</span></button>
        <p class="note">{t('file.dropHint')}</p>
      {/snippet}
    </Popover>
    <label class="pbox" use:tip={t('file.nameHint')}>
      <FileText size={14} />
      <span class="plabel">{t('file.project')}</span>
      <span class="pfield">
        <span class="sizer" aria-hidden="true">{shown || t('file.untitled')}</span>
        <input
          class="pname"
          type="text"
          size="1"
          placeholder={t('file.untitled')}
          aria-label={t('file.name')}
          value={store.p.name}
          oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)}
          onchange={(e) => {
            const v = (e.currentTarget as HTMLInputElement).value;
            draft = null;
            store.edit((p) => (p.name = v));
          }}
          onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()}
        />
      </span>
    </label>
  </div>

  <div class="right">
    <div class="units" role="radiogroup" aria-label={t('app.units')}>
      {#each [['mm', 'mm'], ['in', t('app.inches')]] as [id, label] (id)}
        <button type="button" role="radio" aria-checked={store.units === id} class:on={store.units === id} onclick={() => (store.units = id as 'mm' | 'in')}>{label}</button>
      {/each}
    </div>
    <div class="units" role="radiogroup" aria-label={t('app.language')}>
      {#each LOCALES as l (l.id)}
        <button type="button" role="radio" aria-checked={i18n.locale === l.id} aria-label={l.label} class:on={i18n.locale === l.id} onclick={() => i18n.set(l.id)}>{l.id.toUpperCase()}</button>
      {/each}
    </div>
    <div class="units theme" role="radiogroup" aria-label={t('app.theme')}>
      {#each THEMES as th (th)}
        {@const Icon = { system: Monitor, light: Sun, dark: Moon }[th]}
        <button type="button" role="radio" aria-checked={theme.choice === th} aria-label={t(`app.themes.${th}`)} class:on={theme.choice === th} use:tip={t(`app.themes.${th}`)} onclick={() => theme.set(th)}><Icon size={12} /></button>
      {/each}
    </div>
    <button class="btn quiet icon" type="button" use:tip={`${t('app.help')} (?)`} aria-label={t('app.help')} onclick={() => (store.help = true)}>
      <CircleHelp size={16} />
    </button>
    <Popover label={t('export.menu')} width={300}>
      {#snippet trigger({ open, toggle })}
        <button class="btn primary" class:open type="button" onclick={toggle} aria-haspopup="menu" aria-expanded={open}>
          <Download size={14} />
          {t('export.button')}
          <ChevronDown size={13} />
        </button>
      {/snippet}
      {#snippet content({ close })}
        {#if blocked}<p class="note warn">{t('export.blocked')}</p>{/if}
        <button class="row" type="button" disabled={blocked} onclick={() => { close(); void exportAs('svg'); }}>
          <FileImage size={14} />
          <span class="txt"><span class="rname">{t('export.svg')}</span><span class="desc">{t('export.svgHint')}</span></span>
        </button>
        <button class="row" type="button" disabled={blocked} onclick={() => { close(); void exportAs('dxf'); }}>
          <FileCode size={14} />
          <span class="txt"><span class="rname">{t('export.dxf')}</span><span class="desc">{t('export.dxfHint')}</span></span>
        </button>
        <button class="row" type="button" disabled={blocked || !store.frame} onclick={() => { close(); exportPng(); }}>
          <ImageDown size={14} />
          <span class="txt"><span class="rname">{t('export.png')}</span><span class="desc">{t('export.pngHint')}</span></span>
        </button>
        <div class="divider"></div>
        <button class="row" type="button" onclick={() => { close(); void exportAs('project'); }}>
          <FileBraces size={14} />
          <span class="txt"><span class="rname">{t('export.project')}</span><span class="desc">{t('export.projectHint')}</span></span>
        </button>
      {/snippet}
    </Popover>
  </div>
</header>

<style>
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s4);
    height: 42px;
    padding: 0 var(--s3);
    background: var(--bg-chrome);
    border-bottom: 1px solid var(--border);
  }

  .bar::before {
    content: '';
    position: absolute;
    inset: 0 0 auto 0;
    height: 2px;
    background: var(--identity);
    opacity: 0.85;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .logo {
    width: 24px;
    height: 24px;
    display: block;
  }

  .name {
    font-weight: 700;
    font-size: var(--fs-lg);
    letter-spacing: -0.01em;
  }

  .tag {
    color: var(--text-muted);
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  @media (max-width: 1100px) {
    .tag {
      display: none;
    }
  }

  .project {
    display: flex;
    align-items: center;
    gap: var(--s1);
    min-width: 0;
  }

  .fbtn {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 var(--s2);
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--text-secondary);
    font-size: var(--fs-md);
    white-space: nowrap;
  }

  .fbtn:hover,
  .fbtn.open {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Encart du projet : même dessin que l'encart de la machine dans FabSpeed. */
  .project :global(.pop) {
    width: auto;
  }

  .pbox {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--s2);
    min-width: 0;
    max-width: 460px;
    height: var(--control-h);
    padding: 0 var(--s2);
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-secondary);
    cursor: text;
  }

  .pbox:hover {
    border-color: var(--border-strong);
  }

  .pbox:focus-within {
    border-color: var(--accent);
  }

  .plabel {
    font-size: var(--fs-sm);
    white-space: nowrap;
  }

  /* Champ à la largeur de son texte : le texte invisible du `.sizer` et le
     champ partagent la même case de grille. */
  .pfield {
    display: inline-grid;
    min-width: 0;
    height: 100%;
    align-items: center;
  }

  .sizer,
  .pname {
    grid-area: 1 / 1;
    font: inherit;
    font-weight: 600;
    white-space: pre;
  }

  .sizer {
    visibility: hidden;
    min-width: 40px;
    padding-right: 2px;
    overflow: hidden;
  }

  .pname {
    width: 100%;
    min-width: 0;
    height: 100%;
    padding: 0;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text-primary);
    font-weight: 600;
    text-overflow: ellipsis;
  }

  @media (max-width: 1100px) {
    .pbox {
      max-width: 280px;
    }
  }

  .pname::placeholder {
    color: var(--text-muted);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    padding: 6px var(--s2);
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    color: var(--text-secondary);
  }

  .row:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .row:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .txt {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .rname {
    flex: 1;
    color: var(--text-primary);
    font-weight: 500;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .kbd {
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .divider {
    height: 1px;
    margin: var(--s1) 0;
    background: var(--divider);
  }

  .sub {
    margin: var(--s2) var(--s2) var(--s1);
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-section);
  }

  .fans {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1);
    padding: 0 var(--s2) var(--s1);
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

  .note {
    margin: var(--s1) var(--s2);
    color: var(--text-muted);
    font-size: var(--fs-xs);
  }

  .note.warn {
    color: var(--warning);
  }

  .right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }

  .right :global(.pop) {
    width: auto;
  }

  .units {
    display: flex;
    height: 24px;
    padding: 2px;
    gap: 2px;
    background: var(--segment-track);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .units button {
    padding: 0 var(--s2);
    border: 0;
    border-radius: 3px;
    background: none;
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }

  .units button.on {
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .theme button {
    display: grid;
    place-items: center;
    padding: 0 6px;
  }
</style>
