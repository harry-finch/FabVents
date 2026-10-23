# Feuille de route

**But de la v1.0** : une application de bureau (et une version web sans installation) qui reprend les fonctions utiles de SolidVents pour les makers et les fablabs : motifs de perforation paramétriques pilotés par des contrôleurs posés sur le canevas, contrôles de fabrication, exports laser / CNC (SVG, DXF) et CAO (STEP). Sans compte ni abonnement : les fonctions « Pro » de SolidVents sont incluses.

**Principes** (détail dans [AGENTS.md](AGENTS.md)) :

- moteur Rust pur et déterministe (`fabvents-core`), testé, sans IO ni interface ;
- même écosystème que FabSpeed : Svelte 5, jetons *Professional Workshop UI*, français et anglais, thème sombre de référence et thème clair, Tauri pour le bureau ;
- les valeurs saisies ne sont jamais corrigées en silence : le moteur signale, l'interface montre ;
- millimètres en interne, pouces à l'affichage ;
- un jalon = une version testable, livrée avec sa liste de vérifications.

La source fonctionnelle est l'inventaire de [docs/FEATURES.md](docs/FEATURES.md) (chaque fonction y a son identifiant et son jalon).

---

## v0.1 — Tranche verticale (jalon 1) · *en cours*

**But** : dessiner une grille de ventilation à trous de taille variable et l'envoyer à la découpeuse laser, de bout en bout.

Inclus :

- Workspace Rust (`fabvents-core`, `fabvents-wasm`), interface Svelte, moteur en WebAssembly dans un Web Worker (ADR-0002).
- Plaque rectangulaire (coins arrondis) ou ronde, épaisseur, marge de bord.
- Disposition **Grille** : carrée, hexagonale (60°), quinconce ; pas X/Y, angle, décalage, rotation par rangée / colonne.
- Formes : cercle, rectangle (coins arrondis), polygone régulier, étoile, fente ; taille de référence et rotation.
- Contrôleurs **Point** et **Ligne** sur les canaux **Taille** et **Angle** : portée, valeur cible, bord net ; un contrôleur peut agir sur plusieurs canaux (synchronisation, ADR-0004).
- Bords : par le centre, trous entiers.
- Mesures : nombre de trous, taux d'ouverture, tailles min / max, nervure minimale, chevauchements ; avertissements de fabrication (nervure sous le minimum, trous supprimés car trop petits).
- Canevas : zoom à la molette centré sur le curseur, déplacement (Espace + glisser, bouton du milieu, outil Main), cadrage, poignées déplaçables, création de points et de lignes, suppression, annuler / rétablir.
- Exports : **SVG** (projet embarqué, rouvrable), **DXF** R12 en mm (calques `CUT_INNER` / `CUT_OUTER`), projet `.fabvents.json`. Ouverture d'un projet ou d'un SVG FabVents (glisser-déposer).
- Reprise automatique du dernier projet ; FR / EN ; mm / pouces ; thème.

Critères d'acceptation :

- `cargo test --workspace`, `cargo clippy --workspace --all-targets` et `npm run check` sans erreur.
- Une grille 200 × 150 mm de ~2 000 trous se recalcule pendant le déplacement d'un contrôleur sans saccade visible.
- Le DXF s'ouvre à la bonne échelle (mm) dans LightBurn, Inkscape et FreeCAD ; le SVG exporté puis rouvert redonne exactement le projet.

## v0.2 — Dispositions et contrôleurs (jalon 2)

- Dispositions : **Aléatoire** (Poisson-disc à densité variable, graine), **Spirale**, **Concentrique**, **Fibonacci** (phyllotaxie), **Hachures croisées**, **Chemin** (répétition le long d'un tracé).
- Orientation selon le chemin (concentrique, spirale, chemin).
- Canal **Espacement** (densité locale des dispositions aléatoire, Fibonacci, chemin).
- Contrôleur **Courbe / polyligne** ouverte ou fermée, congés aux nœuds, conversion en géométrie exacte (cercle, ellipse, polygone régulier), saisie aux cotes.
- Contrôleur **Image** (halftone) sur n'importe quel canal : inverser, contraste.
- Options : côté unique et extension infinie, remplir l'intérieur d'une courbe fermée, disparition aléatoire (*cull*) avec graine, angle symétrique et par paliers, unifier portée / valeur d'un canal.
- Outil **Règle** ; redimensionnement proportionnel de tout le motif quand la plaque change de taille.

Acceptation : chaque disposition et chaque option a un test moteur (déterminisme compris : même graine, même résultat).

## v0.3 — Contours et découpe (jalon 3)

- Contours : ellipse, contour importé (SVG, DXF) ou dessiné.
- **Découpe au contour** (booléens 2D, `i_overlay`) ; fusion des trous qui se chevauchent ; taux d'ouverture exact.
- *Ajout FabVents* : **trous de fixation et zones réservées** (aucune perforation autour des vis, d'un logo, d'un passage de câble).
- *Ajout FabVents* : **gabarits** de grilles de ventilateur 40 / 60 / 80 / 92 / 120 / 140 mm avec leurs trous de fixation.
- Export **PNG** haute résolution, fond transparent.

## v0.4 — Export STEP (jalon 4)

- Plaque extrudée en **B-Rep explicite** (`MANIFOLD_SOLID_BREP`) : trous ronds en vraies faces cylindriques, polygones en faces planes. L'`EXTRUDED_AREA_SOLID` suggéré dans la recherche est écarté (mal lu par les importeurs AP214, voir `docs/FEATURES.md`).
- **Découpage en dalles** (STP Split) N × M : archive ZIP ou assemblage STEP unique.
- **Angle de dépouille** : cônes analytiques, validation avant export (trou qui se referme, paroi percée) surlignée sur le canevas.
- DXF : arcs exacts (*bulge*) pour les congés ; simplification Ramer-Douglas-Peucker.
- Fiches de validation : import FreeCAD, Fusion 360, PrusaSlicer.

## v0.5 — Voronoï et lignes fluides (jalon 5)

- **Voronoï** : relaxation de Lloyd, nervures d'épaisseur variable (canal Taille), congés aux sommets, densité par le canal Espacement.
- **Lignes fluides** : rubans d'épaisseur variable, canaux remappés (épaisseur, espacement, courbure, direction), extrémités plates ou rondes, nettoyage des auto-intersections.

## v0.6 — Atelier de formes et morphing (jalon 6)

- Éditeur de formes : plume, Bézier, primitives cotées, sélection directe, congés non destructifs, import SVG / DXF, symétrie miroir et radiale (2 à 12), calques avec rôles booléens (union, soustraction), réponse au gradient par calque, point de pivot.
- Canal **Forme** : morphing vers une primitive cible ; morphing personnalisé départ ↔ cible.

## v0.7 — Bureau et automatisation (jalon 7)

- Application **Tauri** (macOS, Windows, Linux) : fenêtres natives d'ouverture / d'enregistrement, fichiers récents.
- Bibliothèque locale de préréglages (import / export JSON, adaptation aux proportions de la plaque).
- **CLI** et **serveur MCP** (stdio) : générer, exporter, solveur de taux d'ouverture cible (dichotomie), aperçu PNG pour l'agent.
- Dispositions scriptées (« IA sur mesure ») : script `rhai` en bac à sable, curseurs déclarés par le script.

## v1.0 — Publication

- Installeurs signés, documentation utilisateur FR / EN, tests de bout en bout de l'interface.
- Pièces d'exemple validées à l'atelier : laser (contreplaqué 3 mm, acrylique), CNC (alu, MDF), impression 3D.

## Hors périmètre (à réévaluer après la v1.0)

- **Wrap** (projection d'un motif sur une face 3D courbe d'un STEP importé) : demande un noyau CAO complet (OpenCascade), un dépliage LSCM / ARAP et des booléens 3D.
- Comptes, nuage, place de marché.
