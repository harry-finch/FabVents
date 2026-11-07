# FabVents

Générateur de **motifs de perforation paramétriques** pour makers et fablabs : grilles de ventilation, grilles de haut-parleur, panneaux décoratifs. On règle une plaque (rectangle, disque, ellipse, contour importé), une disposition (grille, aléatoire, spirale, concentrique, Fibonacci, hachures croisées, chemin, Voronoï, lignes fluides, script) et une forme de trou (simple ou dessinée dans l'atelier de formes), puis on pose des **contrôleurs** sur le canevas qui font varier la taille, l'espacement, l'angle ou la forme des perforations. FabVents mesure ce qui compte avant la découpe (taux d'ouverture exact, nervure minimale, chevauchements, îlots), vise un taux d'ouverture à la demande, et exporte à l'échelle en millimètres : **SVG**, **DXF**, **STEP** (solide exact, dépouille, dalles) et **PNG**.

Inspiré des fonctions de SolidVents, sans compte ni abonnement, dans le même écosystème que [FabSpeed](../FabSpeed) (même langage visuel, mêmes conventions).

> État : **v0.7** (bureau et automatisation) livrée pour essai. Voir [STATUS.md](STATUS.md) et [ROADMAP.md](ROADMAP.md).

## Lancer

Prérequis : Rust 1.88 (installé automatiquement par `rust-toolchain.toml`) avec la cible `wasm32-unknown-unknown`, [wasm-pack](https://rustwasm.github.io/wasm-pack/), Node 20+.

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cd ui && npm install && npm run wasm && npm run dev
```

Puis ouvrir http://localhost:5174. Après une modification du moteur Rust, relancer `npm run wasm`.

Version de production (site statique, aucun serveur) :

```bash
cd ui && npm run build && npm run preview
```

### Application de bureau

Même interface dans une fenêtre native (Tauri), avec les fenêtres d'ouverture et d'enregistrement du système, l'enregistrement sur place (`⌘S`) et les fichiers récents. Construire l'application (`.app` et `.dmg` sur macOS) :

```bash
cargo install tauri-cli --version 2.11.5 --locked
```

```bash
cd crates/fabvents-desktop && cargo tauri build
```

## Utiliser

1. **À gauche** : plaque, disposition, forme (dont l'**atelier de formes** et la forme cible du morphing), bords, fabrication (nervure mini, dépouille et dalles de l'export STEP).
2. **Au centre** : onglets **Taille**, **Espacement**, **Angle**, **Forme**, **Chemin**, **Plaque** ; poser un point (`P`), une ligne (`L`), une courbe (`C`) ou une image (`I`), glisser les poignées.
3. **À droite** : élément sélectionné, mesures (et « Viser un taux… »), messages du moteur.
4. **Fichier** : nouveau projet, exemple, gabarits de ventilateur, **préréglages** (motifs enregistrés, adaptés à la plaque en cours), ouvrir, enregistrer.
5. **Exporter** : SVG (projet embarqué, rouvrable), DXF R12, STEP, STEP par dalle (ZIP), PNG, projet `.fabvents.json`.

Raccourcis : `?` dans l'application.

## Ligne de commande et agents IA

Le binaire `fabvents` donne le moteur sans interface :

```bash
cargo build --release -p fabvents-cli
```

| Commande | Rôle |
| --- | --- |
| `fabvents template 120 > grille.json` | Gabarit de grille de ventilateur |
| `fabvents info grille.json` | Mesures et messages (JSON) |
| `fabvents solve grille.json --target 35 --param size -o réglé.json` | Viser un taux d'ouverture (taille ou espacement), sans nervure trop fine |
| `fabvents export réglé.json plaque.svg plaque.dxf plaque.step aperçu.png` | Exports, selon l'extension (`.svg`, `.dxf`, `.step`, `.zip`, `.png`, `.json`) |
| `fabvents mcp` | Serveur MCP sur stdio |

Le serveur MCP expose six outils à un agent (Claude Code, Cursor…) : guide du format, nouveau projet, calcul, solveur de taux d'ouverture, aperçu PNG, export. Pour l'ajouter à Claude Code :

```bash
claude mcp add fabvents -- /chemin/vers/FabVents/target/release/fabvents mcp
```

## Développer

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cd ui && npm run check
```

Exporter un projet sans interface (SVG, DXF, STEP) : `cargo run -p fabvents-core --example export -- <dossier> [projet.json]`. Temps de calcul par étage : `cargo run --release -p fabvents-core --example bench -- projet.json`. Contrôle d'un STEP dans FreeCAD : [`scripts/check_step.py`](scripts/check_step.py).

| Dossier | Rôle |
| --- | --- |
| `crates/fabvents-core` | Moteur pur : projet, dispositions, formes, contrôleurs, mesures, découpe, solides, exports |
| `crates/fabvents-wasm` | Le moteur pour l'interface (WebAssembly, appelé depuis un Web Worker) |
| `crates/fabvents-cli` | Ligne de commande, serveur MCP, aperçu PNG |
| `crates/fabvents-desktop` | Application de bureau (Tauri) |
| `ui/` | Interface Svelte 5 (skill `fab-design-system`, partagée avec FabSpeed) |
| `docs/` | [Architecture](docs/ARCHITECTURE.md), [inventaire des fonctions](docs/FEATURES.md), [décisions](docs/adr), [validations](docs/validation) |
| `scripts/` | Outils de vérification (FreeCAD) |

## Licence

GPL-3.0-or-later, comme FabSpeed et FabCAM. Voir [LICENSE](LICENSE).
