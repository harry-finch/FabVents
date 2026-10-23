<script lang="ts">
  // Choix de la disposition : menu des sept modes, pictogramme et usage.
  import { Check, ChevronDown } from '@lucide/svelte';
  import Popover from './Popover.svelte';
  import LayoutGlyph from './LayoutGlyph.svelte';
  import { store } from '../lib/store.svelte';
  import { t } from '../lib/i18n.svelte';
  import { LAYOUT_MODES } from '../lib/types';

  const mode = $derived(store.p.layout.mode);
</script>

<Popover label={t('layout.mode')} width={300}>
  {#snippet trigger({ open, toggle })}
    <button class="pick input" class:open type="button" onclick={toggle} aria-haspopup="menu" aria-expanded={open}>
      <LayoutGlyph {mode} size={15} />
      <span class="name">{t(`layout.modes.${mode}`)}</span>
      <ChevronDown size={13} />
    </button>
  {/snippet}
  {#snippet content({ close })}
    {#each LAYOUT_MODES as m (m)}
      <button
        class="row"
        class:selected={m === mode}
        type="button"
        onclick={() => {
          store.setLayoutMode(m);
          close();
        }}
      >
        <span class="glyph"><LayoutGlyph mode={m} /></span>
        <span class="txt">
          <span class="rname">{t(`layout.modes.${m}`)}</span>
          <span class="desc">{t(`layout.modeHints.${m}`)}</span>
        </span>
        {#if m === mode}<Check size={14} />{/if}
      </button>
    {/each}
  {/snippet}
</Popover>

<style>
  .pick {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    color: var(--text-primary);
    text-align: left;
  }

  .pick.open {
    border-color: var(--accent);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    color: var(--accent-text);
  }

  .row:hover {
    background: var(--bg-hover);
  }

  .row.selected {
    background: var(--bg-selected);
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 22px;
    color: var(--text-secondary);
  }

  .txt {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .rname {
    color: var(--text-primary);
    font-weight: 500;
  }

  .desc {
    color: var(--text-muted);
    font-size: var(--fs-sm);
  }
</style>
