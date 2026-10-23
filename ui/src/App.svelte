<script lang="ts">
  // Mise en page : barre d'application, motif (gauche), canevas (centre),
  // inspecteur (droite), barre d'état. Raccourcis clavier et dépôt de
  // fichiers au niveau de la fenêtre.
  import AppBar from './components/AppBar.svelte';
  import PatternPanel from './components/PatternPanel.svelte';
  import CanvasToolbar from './components/CanvasToolbar.svelte';
  import Canvas from './components/Canvas.svelte';
  import Inspector from './components/Inspector.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import HelpDialog from './components/HelpDialog.svelte';
  import { store } from './lib/store.svelte';
  import { t } from './lib/i18n.svelte';
  import { exportAs, isImageFile, openFile, pickFile, pickImage } from './lib/files';
  import { translate } from './lib/geometry';
  import type { Tab, Tool } from './lib/types';

  // Recalcul à chaque changement du projet (le moteur garde en cache les
  // étages inchangés ; un seul calcul en cours, le plus récent gagne).
  // Motif lourd : pendant un geste, le calcul attend le relâchement.
  $effect(() => {
    if (!store.project) return;
    const json = JSON.stringify(store.project);
    if (store.deferred) return;
    store.engine.compute(json);
  });

  // Sauvegarde automatique (reprise au prochain lancement).
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (!store.project) return;
    void JSON.stringify(store.project);
    void store.units;
    void store.tab;
    void store.scalePattern;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => store.persist(), 400);
  });

  const TOOL_KEYS: Record<string, Tool> = { v: 'select', h: 'hand', p: 'point', l: 'line', c: 'curve', r: 'ruler' };
  const TAB_KEYS: Record<string, Tab> = { '1': 'size', '2': 'spacing', '3': 'angle', '4': 'path', '5': 'plate' };

  function typing(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
  }

  function nudge(dx: number, dy: number) {
    const k = store.selection;
    const g = store.selectedGeometry;
    if (!k || !g) return;
    store.setGeometry(k, translate(g, { x: dx, y: dy }), 'nudge');
  }

  function keydown(e: KeyboardEvent) {
    if (!store.project || store.help) return;
    const mod = e.metaKey || e.ctrlKey;
    const k = e.key.toLowerCase();
    if (mod && !typing(e)) {
      if (k === 'z') {
        e.preventDefault();
        if (e.shiftKey) store.redo();
        else store.undo();
      } else if (k === 'y') {
        e.preventDefault();
        store.redo();
      }
    }
    if (mod && k === 'o') {
      e.preventDefault();
      pickFile();
    } else if (mod && k === 's') {
      e.preventDefault();
      void exportAs('project');
    }
    if (mod || e.altKey || typing(e)) return;
    if (e.key === ' ') {
      if ((e.target as HTMLElement)?.tagName === 'BUTTON') return;
      e.preventDefault();
      store.spacePan = true;
    } else if (TOOL_KEYS[k]) {
      store.tool = TOOL_KEYS[k];
    } else if (k === 'i' && store.channel) {
      pickImage();
    } else if (k === 'f') {
      store.fit();
    } else if (TAB_KEYS[k] && store.tabs.includes(TAB_KEYS[k])) {
      store.setTab(TAB_KEYS[k]);
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && store.selection && store.tool !== 'curve') {
      e.preventDefault();
      store.removeSelected();
    } else if (e.key === 'Escape') {
      if (store.tool !== 'select') store.tool = 'select';
      else store.select(null);
    } else if (e.key.startsWith('Arrow') && store.selection) {
      e.preventDefault();
      const step = e.shiftKey ? 10 : 1;
      const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[e.key];
      if (d) nudge(d[0], d[1]);
    } else if (e.key === '?') {
      store.help = true;
    }
  }

  function keyup(e: KeyboardEvent) {
    if (e.key === ' ') store.spacePan = false;
  }

  // Glisser-déposer d'un projet (.fabvents.json ou SVG exporté par FabVents).
  let dropping = $state(false);
  function dragover(e: DragEvent) {
    if (!e.dataTransfer?.types.includes('Files')) return;
    e.preventDefault();
    dropping = true;
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    dropping = false;
    const f = e.dataTransfer?.files?.[0];
    if (!f) return;
    if (isImageFile(f)) void store.addImage(f);
    else void openFile(f);
  }
</script>

<svelte:window onkeydown={keydown} onkeyup={keyup} onblur={() => (store.spacePan = false)} ondragover={dragover} ondrop={drop} />

<div class="app">
  {#if store.project}
    <AppBar />
    <div class="main">
      <PatternPanel />
      <div class="center">
        <CanvasToolbar />
        <Canvas />
      </div>
      <Inspector />
    </div>
    <StatusBar />
  {:else}
    <div class="splash">
      <img src="/fabvents-logo.svg" alt="" width="48" height="48" />
      {#if store.engineState === 'failed'}
        <p class="err">{t('status.failedLong')}</p>
      {:else}
        <p>{t('status.loading')}</p>
      {/if}
    </div>
  {/if}
</div>

{#if dropping}
  <div class="drop" role="presentation" ondragleave={() => (dropping = false)}>
    <p>{t('file.dropHere')}</p>
  </div>
{/if}

{#if store.help}<HelpDialog />{/if}

<div class="toasts" aria-live="polite">
  {#each store.toasts as tt (tt.id)}
    <div class="toast">{tt.text}</div>
  {/each}
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    min-width: 0;
  }

  .main {
    display: grid;
    grid-template-columns: 296px minmax(0, 1fr) 296px;
    min-height: 0;
  }

  @media (max-width: 1100px) {
    .main {
      grid-template-columns: 264px minmax(0, 1fr) 264px;
    }
  }

  .center {
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
    min-height: 0;
  }

  .splash {
    grid-row: 1 / -1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s3);
    color: var(--text-muted);
  }

  .err {
    color: var(--danger-text);
    max-width: 420px;
    text-align: center;
  }

  .drop {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    background: var(--backdrop);
    border: 2px dashed var(--accent);
    color: var(--text-primary);
    font-size: var(--fs-lg);
  }

  .toasts {
    position: fixed;
    bottom: 36px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 900;
    pointer-events: none;
  }

  .toast {
    padding: 8px 14px;
    border-radius: var(--radius-lg);
    background: var(--tooltip-bg);
    color: var(--tooltip-text);
    font-size: var(--fs-sm);
    box-shadow: var(--shadow-pop);
    animation: rise 160ms ease-out;
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
