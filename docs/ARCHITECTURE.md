# Architecture

## Vue d'ensemble

```text
┌──────────────────────── navigateur, ou vue web Tauri (v0.7) ────────────────────────┐
│                                                                                      │
│  Interface Svelte (fil principal)                 Web Worker                         │
│  ┌──────────────────────────────┐   projet JSON  ┌────────────────────────────────┐  │
│  │ store : projet + historique  │ ─────────────► │ fabvents-wasm (WebAssembly)    │  │
│  │ panneaux, canevas, exports   │ ◄───────────── │  └─ fabvents-core (Rust pur)   │  │
│  └──────────────────────────────┘  géométrie en  │     Pipeline · mesures · SVG / │  │
│        │ reprise automatique        tableaux     │     DXF                        │  │
│        ▼ (localStorage)             transférés   └────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

Aucun serveur : l'interface est un site statique, le moteur tourne dans le navigateur (ADR-0002).

## Crates

### `fabvents-core`

Bibliothèque pure et déterministe (dépendances : `serde`, `serde_json`, `i_overlay` pour les booléens, `roxmltree` et `svgtypes` pour l'import SVG). Millimètres et degrés ; origine en bas à gauche de la plaque, Y vers le haut.

| Module | Rôle |
| --- | --- |
| `project` | `Project` sérialisable (format `fabvents.project`, version 5) : plaque (contour, zone perforée), disposition, forme, bords, fabrication (dont dépouille et dalles de l'export STEP), contrôleurs, éléments de plaque (trous de fixation, zones réservées). Lecture avec contrôle du format et de la version. |
| `validate` | Contrôle des valeurs : erreurs (motif non calculé) et avertissements, par code stable et chemin de champ. |
| `geom` | `Vec2`, aires, distances point / segment / âme, arcs en *bulge*. |
| `contour` | Contour exact : cercle, ou sommets avec arcs (*bulge*) ; contour d'une âme arrondie (somme de Minkowski) ; aire exacte ; facettisation à flèche donnée (ADR-0005). |
| `region` | Région plane en âme + arrondi (disque, rectangle arrondi, polygone) : distance signée, contenance d'un trou, écart, contour, élargissement. |
| `plate` | Plaque et zone perforée en régions : distance signée, contour exact, aire, contour de la zone perforable (plaque réduite de la marge ∩ zone). |
| `boolean` | Booléens `i_overlay` : décalage d'un contour, union des perforations coupée par la zone et privée des zones réservées, îlots, combinaison des calques d'une forme. |
| `import` | Contour importé d'un SVG (chemins, formes, transformations, unités) ou d'un DXF (polylignes à *bulge*, cercles, ellipses, lignes et arcs chaînés) : contour extérieur et découpes intérieures. |
| `templates` | Gabarits de grilles de ventilateur 40 à 140 mm (entraxe, Ø des vis, ouverture). |
| `layout` | Dispositions → germes (position, angle propre, contour imposé) : grille, aléatoire (Bridson à rayon variable), spirale, concentrique, Fibonacci, hachures croisées, chemin, Voronoï (cellules), lignes fluides (polylignes), script (Rhai). Les dispositions à espacement libre lisent le canal Espacement. |
| `voronoi` | Cellules par découpe de demi-plans, relaxation de Lloyd pondérée par la densité voulue, réduction exacte d'une cellule (nervure). |
| `flow` | Lignes fluides : lignes de courant (Runge-Kutta 2) semées sur une transverse, écart minimal entre lignes, arrêt au bord et aux trous de fixation ; rubans de largeur variable nettoyés par `i_overlay`. |
| `solve` | Solveur de taux d'ouverture : dichotomie sur la taille de base ou l'espacement, nervure mini respectée, meilleur réglage si la cible est hors d'atteinte. |
| `script` | Dispositions scriptées (Rhai en bac à sable) : germes, curseurs déclarés, aléatoire de la graine, erreurs situées. |
| `curve` | Tracés : aplatissement à congés, subdivision, décalage parallèle, distance et côté (avec prolongement), placement à espacement variable le long d'un tracé. |
| `raster` | Image d'un contrôleur : décodage base64, luminance bilinéaire en un point de la plaque. |
| `rng` | Aléatoire reproductible (SplitMix64) et tirage attaché à une position. |
| `shape` | Forme d'un trou à une taille et un angle donnés : âme, arrondi, rayon englobant, aire. `PreparedShape`, préparée une fois par calcul : forme dessinée (calques facettisés, symétries, union / soustraction, réponse au canal Taille, pivot ; contour mis en cache par taille), morphing (contours rééchantillonnés et appariés, exacts aux deux bouts) ; forme importée d'un SVG / DXF. |
| `field` | Contrôleurs préparés (tracés aplatis, image décodée) ; poids avec options (côté, prolongement, remplissage, image) ; mélange pondéré par canal, symétrie, paliers, bornes du canal, disparition aléatoire (ADR-0004, ADR-0006). |
| `pipeline` | Calcul par étages avec cache (germes → trous → mesures → découpe → pièces STEP) ; messages du calcul. |
| `fit` | Contours exacts des découpes : arcs retrouvés sur les cercles connus (trous, gardes, zone, plaque), raccords recalés sur les intersections exactes, segments alignés fusionnés (RDP). |
| `brep` | Boucles exactes (segments et arcs) orientées matière à gauche : aire, boîte, décalage du fond (dépouille), coupe de plusieurs boucles par une droite (dalles, encoches à travers les trous). |
| `solid` | Pièces de l'export STEP : plaque ou dalles, îlots, parois en dépouille ; contrôles `draft.collapse`, `draft.pierce`, `tiles.draft` et perforations en cause pour le canevas. |
| `metrics` | Taux d'ouverture, tailles, nervures exactes et chevauchements (grille de répartition), trous à fusionner. |
| `export` | `Drawing` (contours intérieurs : trous simples, découpes exactes, éléments découpés), `svg` (projet embarqué dans `<metadata>`), `dxf` (R12), `step` (AP214, B-Rep explicite, ADR-0008), `zip` (archive stockée, une dalle par fichier), lecture du projet d'un SVG (ADR-0003). |
| `issue` | `Issue` : code, gravité, champ, valeur, nombre. |

Cycle d'un calcul :

```text
Project ─► validate ─► erreurs ? ─oui─► aucun trou, messages
                          │ non
                          ▼
        germes (plaque, disposition, contrôleurs d'espacement)   cache par égalité
          ─► trous : zone (centre), taille (canal Taille), taille mini, disparition,
                     angle (canal Angle + disposition), forme, zone (trou entier),
                     garde des éléments de plaque
          ─► mesures (trous, nervure mini, chevauchements)
          ─► découpe : union des trous qui se chevauchent ou dépassent (Découper),
                       arcs retrouvés (`fit`)
          ─► pièces STEP (seulement avec dépouille ou dalles) : contrôles, coupes
          ─► Frame : trous, découpes, mesures, statistiques, messages, pièces
```

### `fabvents-cli`

Binaire `fabvents` : `info`, `export` (selon l'extension), `solve`, `template`, `example`, et `mcp` (serveur Model Context Protocol, JSON-RPC 2.0 ligne à ligne sur stdio : guide, nouveau projet, calcul, solveur, aperçu PNG, export). Aperçu PNG rendu sans fenêtre par `tiny-skia`.

### `fabvents-desktop`

Application Tauri 2 : la même interface et le même moteur WebAssembly dans une fenêtre native ; commandes Rust pour les fenêtres d'ouverture et d'enregistrement (greffon `dialog`), l'enregistrement sur place et les fichiers récents (`recent.json` dans le dossier de données). L'interface les appelle par `window.__TAURI__` ([desktop.ts](../ui/src/lib/desktop.ts)) ; dans un navigateur, elle télécharge comme avant.

### `fabvents-wasm`

Pont `wasm-bindgen` : `Engine.compute(json)` renvoie la géométrie à dessiner en tableaux plats (cercles `x, y, r` ; autres contours et découpes facettisés à 0,02 mm ; contours de la plaque et de la zone perforée ; perforations en cause et coupes des dalles de l'export STEP) et un rapport JSON ; `export_svg`, `export_dxf`, `export_step` et `export_step_zip` (date de l'en-tête fournie par l'interface : le moteur n'a pas d'horloge), `read_project` (JSON ou SVG FabVents), `import_outline` (SVG ou DXF quelconque), `import_shape` et `shape_preview` (atelier de formes), `solve_open_ratio`, `script_example`, `fan_template`, `example_project`, `default_project`. Les erreurs sortent par leur code stable, traduit par l'interface. Le `Pipeline` reste en mémoire entre deux appels : seul l'étage modifié est recalculé.

## Interface (`ui/`)

Svelte 5 (runes) + Vite + TypeScript, icônes `@lucide/svelte`, police Inter : même pile que FabSpeed et FabCAM. Les jetons de [app.css](../ui/src/app.css), [Section](../ui/src/components/Section.svelte), [Segmented](../ui/src/components/Segmented.svelte), [Popover](../ui/src/components/Popover.svelte), les infobulles, la langue et le thème sont repris de FabSpeed ; seule la couleur d'identité change.

```text
┌ Barre d'application : logo · Fichier · nom du projet · mm/pouces · FR/EN · thème · aide · Exporter ┐
├ Motif (gauche) ──┬─ Canevas ─────────────────────────────────────────┬─ Inspecteur (droite) ────┤
│ 1 Plaque         │   Outils : sélection, main, point, ligne,       │ Élément sélectionné      │
│ 2 Disposition    │   courbe, image, règle ; onglets Taille,         │   ou canal affiché       │
│ 3 Forme          │   Espacement, Angle, Chemin                      │ Mesures                  │
│ 4 Bords          │                                                  │ Messages                 │
│ 5 Fabrication    │                                                  │                          │
├──────────────────┴──────────────────────────────────────────────────┴──────────────────────────┤
│ Barre d'état : moteur · perforations · temps de calcul · position du curseur                   │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **État** : [store.svelte.ts](../ui/src/lib/store.svelte.ts). Toute modification passe par `edit()` (une étape d'annulation, fusionnée quand la même valeur change de suite) ou par un geste `begin()` … `end()` (glisser une poignée, faire défiler un libellé). Le projet est sauvegardé dans le navigateur 400 ms après chaque changement et repris au lancement.
- **Moteur** : [engine.ts](../ui/src/lib/engine.ts) et [engine.worker.ts](../ui/src/lib/engine.worker.ts). Un seul calcul en cours ; si le projet change entre-temps, seul le plus récent est envoyé. Les tableaux de géométrie sont transférés sans copie.
- **Canevas** : [Canvas.svelte](../ui/src/components/Canvas.svelte), Canvas 2D, un chemin par famille (cercles, polygones), redessiné à la trame suivante. Couleurs lues dans les jetons du thème. Recadrage automatique tant que l'utilisateur n'a pas touché à la vue. Il édite les **éléments** de l'onglet affiché (`store.items`) : contrôleurs du canal, ou tracés de la disposition Chemin. Poignées, distances, déplacements, conversions et mise à l'échelle : [geometry.ts](../ui/src/lib/geometry.ts) ; images : [image.ts](../ui/src/lib/image.ts) (réduction à 200 px, luminance en base64, rendu mis en cache) ; menu contextuel : [ContextMenu.svelte](../ui/src/components/ContextMenu.svelte).
- **Atelier de formes** : [ShapeWorkshop.svelte](../ui/src/components/ShapeWorkshop.svelte), fenêtre de dessin d'une forme par calques (vue SVG, aperçu du moteur par `shape_preview`, primitives de [shapes.ts](../ui/src/lib/shapes.ts)) ; ouverte sur la forme ou sur la cible du morphing (`store.workshop`).
- **Préréglages** : [presets.svelte.ts](../ui/src/lib/presets.svelte.ts) (bibliothèque du poste, fichier `fabvents.presets`) et `store.applyPreset` (motif mis à l'échelle de la plaque en cours, une étape d'annulation).
- **Calcul différé** : si le premier calcul d'un geste dépasse 60 ms, les suivants attendent le relâchement (`store.deferred`) ; la barre d'état le signale.
- **Champs numériques** : [NumField.svelte](../ui/src/components/NumField.svelte), unités converties à l'affichage seulement, flèches, défilement par le libellé, soulignés en rouge quand le moteur signale une erreur sur leur champ.
- **Langues** : `t('clé')`, [fr.ts](../ui/src/lib/locales/fr.ts) de référence, [en.ts](../ui/src/lib/locales/en.ts) vérifié par TypeScript ; messages du moteur rendus par [issues.ts](../ui/src/lib/issues.ts).

## Décisions

Voir [adr/](adr) : workspace Rust et interface Svelte de l'écosystème FabSpeed (0001), moteur en WebAssembly dans un Web Worker (0002), formats de fichiers (0003), contrôleurs multi-canaux et mélange pondéré (0004), trous en âme + arrondi (0005), options des contrôleurs (0006), découpe et éléments de plaque (0007), export STEP en B-Rep explicite (0008), Voronoï et lignes fluides (0009), atelier de formes et morphing (0010), bureau et automatisation (0011).
