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

Bibliothèque pure et déterministe (dépendances : `serde`, `serde_json`). Millimètres et degrés ; origine en bas à gauche de la plaque, Y vers le haut.

| Module | Rôle |
| --- | --- |
| `project` | `Project` sérialisable (format `fabvents.project`, version 1) : plaque, disposition, forme, bords, fabrication, contrôleurs. Lecture avec contrôle du format et de la version. |
| `validate` | Contrôle des valeurs : erreurs (motif non calculé) et avertissements, par code stable et chemin de champ. |
| `geom` | `Vec2`, aires, distances point / segment / âme, arcs en *bulge*. |
| `contour` | Contour exact : cercle, ou sommets avec arcs (*bulge*) ; contour d'une âme arrondie (somme de Minkowski) ; aire exacte ; facettisation à flèche donnée (ADR-0005). |
| `plate` | Contour de la plaque : distance signée (zone perforable), contour exact, aire. |
| `layout` | Dispositions → germes (position, angle propre). v0.1 : grille centrée sur la plaque. |
| `shape` | Forme d'un trou à une taille et un angle donnés : âme, arrondi, rayon englobant, aire. |
| `field` | Champs d'influence des contrôleurs, mélange pondéré par canal (ADR-0004). |
| `pipeline` | Calcul par étages avec cache (germes → trous → mesures) ; messages du calcul. |
| `metrics` | Taux d'ouverture, tailles, nervures exactes et chevauchements (grille de répartition). |
| `export` | `svg` (projet embarqué dans `<metadata>`), `dxf` (R12), lecture du projet d'un SVG (ADR-0003). |
| `issue` | `Issue` : code, gravité, champ, valeur, nombre. |

Cycle d'un calcul :

```text
Project ─► validate ─► erreurs ? ─oui─► aucun trou, messages
                          │ non
                          ▼
        germes (plaque, disposition)                    cache par égalité
          ─► trous : zone (centre), taille (canal Taille), taille mini,
                     angle (canal Angle + disposition), forme, zone (trou entier)
          ─► mesures (trous, nervure mini)
          ─► Frame : trous, mesures, statistiques, messages
```

### `fabvents-wasm`

Pont `wasm-bindgen` : `Engine.compute(json)` renvoie la géométrie à dessiner en tableaux plats (cercles `x, y, r` ; autres contours facettisés à 0,02 mm ; contour de la plaque) et un rapport JSON ; `export_svg`, `export_dxf`, `read_project` (JSON ou SVG FabVents), `example_project`, `default_project`. Les erreurs sortent par leur code stable, traduit par l'interface. Le `Pipeline` reste en mémoire entre deux appels : seul l'étage modifié est recalculé.

## Interface (`ui/`)

Svelte 5 (runes) + Vite + TypeScript, icônes `@lucide/svelte`, police Inter : même pile que FabSpeed et FabCAM. Les jetons de [app.css](../ui/src/app.css), [Section](../ui/src/components/Section.svelte), [Segmented](../ui/src/components/Segmented.svelte), [Popover](../ui/src/components/Popover.svelte), les infobulles, la langue et le thème sont repris de FabSpeed ; seule la couleur d'identité change.

```text
┌ Barre d'application : logo · Fichier · nom du projet · mm/pouces · FR/EN · thème · aide · Exporter ┐
├ Motif (gauche) ──┬─ Outils · canal Taille / Angle · annuler · zoom ─┬─ Inspecteur (droite) ────┤
│ 1 Plaque         │                                                  │ Contrôleur sélectionné   │
│ 2 Disposition    │   Canevas : plaque, perforations, contrôleurs    │   ou canal affiché       │
│ 3 Forme          │   du canal, poignées et portée                   │ Mesures                  │
│ 4 Bords          │                                                  │ Messages                 │
│ 5 Fabrication    │                                                  │                          │
├──────────────────┴──────────────────────────────────────────────────┴──────────────────────────┤
│ Barre d'état : moteur · perforations · temps de calcul · position du curseur                   │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **État** : [store.svelte.ts](../ui/src/lib/store.svelte.ts). Toute modification passe par `edit()` (une étape d'annulation, fusionnée quand la même valeur change de suite) ou par un geste `begin()` … `end()` (glisser une poignée, faire défiler un libellé). Le projet est sauvegardé dans le navigateur 400 ms après chaque changement et repris au lancement.
- **Moteur** : [engine.ts](../ui/src/lib/engine.ts) et [engine.worker.ts](../ui/src/lib/engine.worker.ts). Un seul calcul en cours ; si le projet change entre-temps, seul le plus récent est envoyé. Les tableaux de géométrie sont transférés sans copie.
- **Canevas** : [Canvas.svelte](../ui/src/components/Canvas.svelte), Canvas 2D, un chemin par famille (cercles, polygones), redessiné à la trame suivante. Couleurs lues dans les jetons du thème. Recadrage automatique tant que l'utilisateur n'a pas touché à la vue.
- **Champs numériques** : [NumField.svelte](../ui/src/components/NumField.svelte), unités converties à l'affichage seulement, flèches, défilement par le libellé, soulignés en rouge quand le moteur signale une erreur sur leur champ.
- **Langues** : `t('clé')`, [fr.ts](../ui/src/lib/locales/fr.ts) de référence, [en.ts](../ui/src/lib/locales/en.ts) vérifié par TypeScript ; messages du moteur rendus par [issues.ts](../ui/src/lib/issues.ts).

## Décisions

Voir [adr/](adr) : workspace Rust et interface Svelte de l'écosystème FabSpeed (0001), moteur en WebAssembly dans un Web Worker (0002), formats de fichiers (0003), contrôleurs multi-canaux et mélange pondéré (0004), trous en âme + arrondi (0005).
