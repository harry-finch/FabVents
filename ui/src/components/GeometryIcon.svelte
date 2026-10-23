<script lang="ts">
  // Icône d'un type de contrôleur (Lucide).
  import { Crosshair, Slash, Spline, Circle, Ellipse, Hexagon, Image } from '@lucide/svelte';
  import type { GeometryType } from '../lib/types';

  let { type, size = 14 }: { type: GeometryType; size?: number } = $props();
  const ICONS = { point: Crosshair, line: Slash, curve: Spline, circle: Circle, ellipse: Ellipse, polygon: Hexagon, image: Image };
  const Icon = $derived(ICONS[type]);
</script>

<Icon {size} />
