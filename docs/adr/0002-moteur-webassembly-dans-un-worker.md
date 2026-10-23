# ADR-0002 : Moteur en WebAssembly dans un Web Worker

Status: Accepted
Date: 2026-10-23

## Context

Le motif est recalculé à chaque mouvement d'une poignée. FabSpeed passe par un serveur HTTP local (axum) embarqué dans l'application de bureau ; pour FabVents, un aller-retour réseau à chaque trame et un serveur à lancer n'apportent rien : il n'y a ni base partagée ni droits à gérer. SolidVents calcule dans des Web Workers.

## Decision

- `fabvents-core` est compilé en WebAssembly (`fabvents-wasm`, `wasm-pack --target web`) et exécuté dans un **Web Worker module**.
- Protocole : le fil principal envoie le projet en JSON ; le worker renvoie la géométrie en `Float64Array` / `Uint32Array` **transférés** et un rapport JSON.
- **Le dernier gagne** : un seul calcul en cours ; un projet arrivé pendant le calcul remplace le précédent en attente.
- Le `Pipeline` reste dans le worker : son cache par étage évite de recalculer la disposition quand seul un contrôleur bouge.
- L'interface est un site statique (`vite build`) ; la future application Tauri l'embarque telle quelle.

## Alternatives considered

- Serveur local comme FabSpeed : latence à chaque trame, processus à lancer, et pas de version web sans installation.
- WebAssembly sur le fil principal : simple, mais un grand motif (30 000 trous, ~20 ms) figerait les poignées.
- Commandes Tauri (IPC) : liées au bureau, pas de version navigateur.

## Consequences

Pas de `rayon` (pas de threads en WebAssembly sans configuration spéciale) : le calcul est mono-thread, mesuré à 17 ms pour 29 000 trous. Le moteur doit être reconstruit (`npm run wasm`) après chaque modification Rust ; `ui/src/wasm/` est généré et hors Git.

## Validation

Mesure dans le navigateur : 29 121 trous calculés en 17 ms dans le worker, fil principal à 120 images/s pendant le zoom. À revoir si un motif dépasse ~50 ms (calcul différé au relâchement, UX-09, ou parallélisme).
