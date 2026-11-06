<script lang="ts">
  // Pictogramme d'une forme de perforation (trait, couleur du texte), même
  // graisse que les icônes Lucide.
  import type { ShapeType } from '../lib/types';

  let { type, size = 16 }: { type: ShapeType; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true">
  {#if type === 'circle'}
    <circle cx="12" cy="12" r="8" />
  {:else if type === 'rectangle'}
    <rect x="4" y="7" width="16" height="10" rx="2.5" />
  {:else if type === 'polygon'}
    <polygon points="7.5,19.8 16.5,19.8 21,12 16.5,4.2 7.5,4.2 3,12" />
  {:else if type === 'star'}
    <polygon points="12,3 14.4,9.3 21,9.6 15.8,13.8 17.6,20.3 12,16.6 6.4,20.3 8.2,13.8 3,9.6 9.6,9.3" />
  {:else if type === 'custom'}
    <path d="M4 20l1.5-5.5L16 4l4 4L9.5 18.5zM13.5 6.5l4 4" />
  {:else}
    <rect x="3" y="8.5" width="18" height="7" rx="3.5" />
  {/if}
</svg>
