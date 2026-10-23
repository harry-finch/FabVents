# FabVents

Générateur de **motifs de perforation paramétriques** pour makers et fablabs : grilles de ventilation, grilles de haut-parleur, panneaux décoratifs. On règle une plaque, une disposition (grille, aléatoire, spirale, concentrique, Fibonacci, hachures croisées, chemin) et une forme de trou, puis on pose des **contrôleurs** sur le canevas (points, lignes, courbes, cercles, ellipses, polygones, images) qui font varier la taille, l'espacement ou l'angle des perforations. FabVents mesure ce qui compte avant la découpe (taux d'ouverture, nervure minimale, chevauchements) et exporte en **SVG** et **DXF** à l'échelle, en millimètres.

Inspiré des fonctions de SolidVents, sans compte ni abonnement, dans le même écosystème que [FabSpeed](../FabSpeed) (même langage visuel, mêmes conventions).

> État : **v0.2** (dispositions et contrôleurs) livrée pour essai. Voir [STATUS.md](STATUS.md) et [ROADMAP.md](ROADMAP.md).

## Lancer

Prérequis : Rust 1.88 (installé automatiquement par `rust-toolchain.toml`) avec la cible `wasm32-unknown-unknown`, [wasm-pack](https://rustwasm.github.io/wasm-pack/), Node 20+.

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cd ui && npm install && npm run wasm && npm run dev
```

Puis ouvrir http://localhost:5174. Après une modification du moteur Rust, relancer `npm run wasm` (le serveur de développement recharge la page).

Version de production (site statique, aucun serveur d'API) :

```bash
cd ui && npm run build && npm run preview
```

## Utiliser

1. **À gauche** : plaque (rectangle à coins arrondis ou disque, marge, mise à l'échelle du motif), disposition et ses réglages, forme (cercle, rectangle, polygone, étoile, fente), bords, seuils de fabrication.
2. **Au centre** : choisir l'onglet (**Taille**, **Espacement**, **Angle**, ou **Chemin** pour la disposition Chemin), poser un point (`P`), une ligne (`L`), une courbe (`C`) ou une image (`I`), glisser les poignées ; le carré du contrôleur sélectionné règle sa portée ; clic droit pour le menu de l'élément, `R` pour la règle.
3. **À droite** : réglages du contrôleur sélectionné, mesures du motif, messages.
4. **Exporter** : SVG (le projet complet est embarqué : glisser le SVG sur la fenêtre le rouvre), DXF R12 (calques `CUT_INNER` / `CUT_OUTER`), projet `.fabvents.json`.

Raccourcis : `?` dans l'application.

## Développer

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cd ui && npm run check
```

Exporter le projet d'exemple sans interface : `cargo run -p fabvents-core --example export -- <dossier>`. Temps de calcul par étage : `cargo run --release -p fabvents-core --example bench -- projet.json`.

| Dossier | Rôle |
| --- | --- |
| `crates/fabvents-core` | Moteur pur : projet, dispositions, formes, contrôleurs, mesures, exports SVG / DXF |
| `crates/fabvents-wasm` | Le moteur pour l'interface (WebAssembly, appelé depuis un Web Worker) |
| `ui/` | Interface Svelte 5 (jetons *Professional Workshop UI* partagés avec FabSpeed) |
| `docs/` | [Architecture](docs/ARCHITECTURE.md), [inventaire des fonctions](docs/FEATURES.md), [décisions](docs/adr) |
| `branding/` | Logo (provisoire) |

## Licence

GPL-3.0-or-later, comme FabSpeed et FabCAM. Voir [LICENSE](LICENSE).
