<script lang="ts">
  // Menu contextuel (clic droit sur le canevas), en position fixe au curseur.
  // Fermeture : clic ailleurs, Échap, défilement, ou après une action.
  import type { Component } from 'svelte';

  export interface MenuEntry {
    label: string;
    icon?: Component<{ size?: number }>;
    action: () => void;
    disabled?: boolean;
    separator?: boolean;
  }

  let { x, y, entries, onclose }: { x: number; y: number; entries: MenuEntry[]; onclose: () => void } = $props();

  let menu = $state<HTMLDivElement>();
  const left = $derived(Math.min(x, (typeof window !== 'undefined' ? window.innerWidth : 1e4) - 228));
  const top = $derived(Math.min(y, (typeof window !== 'undefined' ? window.innerHeight : 1e4) - 32 * entries.length - 16));

  $effect(() => {
    menu?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  });

  function outside(e: PointerEvent) {
    if (menu && !menu.contains(e.target as Node)) onclose();
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={(e) => e.key === 'Escape' && onclose()} onwheel={onclose} onblur={onclose} />

<div class="menu" role="menu" bind:this={menu} style:left={`${left}px`} style:top={`${top}px`}>
  {#each entries as e, i (i)}
    {#if e.separator}<div class="sep"></div>{/if}
    <button
      type="button"
      role="menuitem"
      disabled={e.disabled}
      onclick={() => {
        onclose();
        e.action();
      }}
    >
      <span class="ic">{#if e.icon}<e.icon size={14} />{/if}</span>
      {e.label}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 600;
    min-width: 210px;
    padding: var(--s1);
    background: var(--bg-elevated);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-pop);
  }

  button {
    display: flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    height: 28px;
    padding: 0 var(--s2);
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: var(--fs-sm);
    text-align: left;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--bg-hover);
    outline: none;
  }

  button:disabled {
    color: var(--text-disabled);
    cursor: default;
  }

  .ic {
    display: inline-grid;
    width: 14px;
    color: var(--text-secondary);
  }

  .sep {
    height: 1px;
    margin: var(--s1) 0;
    background: var(--divider);
  }
</style>
