import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Le moteur (WebAssembly, `npm run wasm`) tourne dans un Web Worker module :
// aucune API à lancer, l'application est un site statique.
export default defineConfig({
  plugins: [svelte()],
  server: { port: 5174 },
  worker: { format: 'es' },
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
});
