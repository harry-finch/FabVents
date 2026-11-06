<script lang="ts">
  // Pictogramme d'une disposition (trait, couleur du texte), même graisse
  // que les icônes Lucide.
  import type { LayoutMode } from '../lib/types';

  let { mode, size = 16 }: { mode: LayoutMode; size?: number } = $props();

  const fib = Array.from({ length: 18 }, (_, k) => {
    const a = k * 2.39996;
    const r = 1.9 * Math.sqrt(k + 0.5);
    return { x: 12 + r * Math.cos(a), y: 12 + r * Math.sin(a) };
  });
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if mode === 'grid'}
    {#each [5, 12, 19] as y (y)}{#each [5, 12, 19] as x (x)}<circle cx={x} cy={y} r="1.2" fill="currentColor" stroke="none" />{/each}{/each}
  {:else if mode === 'random'}
    {#each [[5, 6], [13, 4], [19, 9], [8, 12], [15, 14], [4, 19], [11, 20], [20, 19]] as [x, y] (`${x}-${y}`)}<circle cx={x} cy={y} r="1.2" fill="currentColor" stroke="none" />{/each}
  {:else if mode === 'spiral'}
    <path d="M12 12c0-1 1.5-1.5 2.2-.6 1 1.2 0 3.4-2 3.6-2.6.3-4.4-2.4-3.6-4.8.9-2.8 4.3-4 7-2.8 3.2 1.4 4.3 5.4 2.6 8.4-1.9 3.4-6.4 4.5-9.7 2.6" />
  {:else if mode === 'concentric'}
    <circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" />
    <circle cx="12" cy="12" r="5" />
    <circle cx="12" cy="12" r="9.5" />
  {:else if mode === 'fibonacci'}
    {#each fib as p, i (i)}<circle cx={p.x} cy={p.y} r="1" fill="currentColor" stroke="none" />{/each}
  {:else if mode === 'cross_hatch'}
    <path d="M4 7l3-3M10 13l3-3M16 19l3-3M4 19l3-3M16 7l3-3M7 10l3 3M13 4l3 3M13 16l3 3M7 16l-3-3" />
  {:else if mode === 'voronoi'}
    <path d="M4 4l6 3 1 6-6 3M10 7l8-3M11 13l7 1 2 6M5 16l2 5M11 13l-2 8M18 14l2-6" />
  {:else if mode === 'flow'}
    <path d="M3 7c5-4 9 4 18 0M3 12c5-4 9 4 18 0M3 17c5-4 9 4 18 0" />
  {:else}
    <path d="M3 18c4-9 9 2 18-9" stroke-dasharray="0.1 4.6" stroke-width="2.6" />
  {/if}
</svg>
