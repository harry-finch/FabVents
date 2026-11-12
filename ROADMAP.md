<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->

**But de la v1.0** : une application de bureau (et une version web sans installation) qui reprend les fonctions utiles de SolidVents pour les makers et les fablabs : motifs de perforation paramétriques pilotés par des contrôleurs posés sur le canevas, contrôles de fabrication, exports laser / CNC (SVG, DXF) et CAO (STEP). Sans compte ni abonnement : les fonctions « Pro » de SolidVents sont incluses.

**Principes** (détail dans [AGENTS.md](AGENTS.md)) :

- moteur Rust pur et déterministe (`fabvents-core`), testé, sans IO ni interface ;
- même écosystème que FabSpeed : Svelte 5, jetons *Professional Workshop UI*, français et anglais, thème sombre de référence et thème clair, Tauri pour le bureau ;
- les valeurs saisies ne sont jamais corrigées en silence : le moteur signale, l'interface montre ;
- millimètres en interne, pouces à l'affichage ;
- un jalon = une version testable, livrée avec sa liste de vérifications.

La source fonctionnelle est l'inventaire de [docs/FEATURES.md](docs/FEATURES.md) (chaque fonction y a son identifiant et son jalon).

**Avancement : 6 livrés sur 10** (60 %)

## Restant à faire

1. **[R-007 v0.7 — Bureau et automatisation (jalon 7)](roadmap/items/R-007-v0-7-bureau-et-automatisation-jalon-7.md)** · À tester · jalon v0.7
   Application de bureau Tauri, bibliothèque de préréglages, CLI et serveur MCP, solveur de taux d'ouverture, dispositions scriptées.
2. **[R-008 v1.0 — Installeurs signés, documentation et tests de bout en bout](roadmap/items/R-008-v1-0-installeurs-signes-documentation-et-tests.md)** · Idée · jalon v1.0
   Installeurs signés, documentation utilisateur en français et en anglais, tests de bout en bout de l'interface.
3. **[R-009 v1.0 — Pièces d'exemple validées à l'atelier](roadmap/items/R-009-v1-0-pieces-d-exemple-validees-a-l-atelier.md)** · Idée · jalon v1.0
   Pièces d'exemple fabriquées et validées : laser (contreplaqué 3 mm, acrylique), CNC (alu, MDF), impression 3D.
4. **[R-012 Figure de FabVents dans l'application](roadmap/items/R-012-figure-de-fabvents-dans-l-application.md)** · Idée
   Poser la figure de la grille de ventilateur (état vide de l'espace de travail, premier lancement, À propos), selon la skill fab-illustration-system. À spécifier.

## Actions à faire

- [ ] [A-001](roadmap/actions/A-001-tester-la-v0-7-application-de-bureau-et-serveur.md) Tester la v0.7 : application de bureau et serveur MCP (tester, R-007)
- [ ] [A-002](roadmap/actions/A-002-importer-un-step-de-fabvents-dans-fusion-360-et.md) Importer un STEP de FabVents dans Fusion 360 et PrusaSlicer (tester, R-004)

## Livré

### v0.1 : Tranche verticale

- [R-001 v0.1 — Tranche verticale (jalon 1)](roadmap/items/R-001-v0-1-tranche-verticale-jalon-1.md) · livrée le 2026-10-23
  Dessiner une grille de ventilation à trous de taille variable et l'envoyer à la découpeuse laser, de bout en bout.

### v0.2 : Dispositions et contrôleurs

- [R-002 v0.2 — Dispositions et contrôleurs (jalon 2)](roadmap/items/R-002-v0-2-dispositions-et-controleurs-jalon-2.md) · livrée le 2026-10-23
  Six nouvelles dispositions, contrôleurs courbe et image, canal Espacement, options de remplissage et de disparition, outil Règle.

### v0.3 : Contours et découpe

- [R-003 v0.3 — Contours et découpe (jalon 3)](roadmap/items/R-003-v0-3-contours-et-decoupe-jalon-3.md) · livrée le 2026-10-23
  Contours ellipse, importés ou dessinés, découpe au contour, trous de fixation et zones réservées, gabarits de ventilateurs, export PNG, zone perforée.

### v0.4 : Export STEP

- [R-004 v0.4 — Export STEP (jalon 4)](roadmap/items/R-004-v0-4-export-step-jalon-4.md) · livrée le 2026-11-06
  Plaque extrudée en B-Rep explicite, découpage en dalles, angle de dépouille, arcs exacts dans le DXF, fiches de validation.

### v0.5 : Voronoï et lignes fluides

- [R-005 v0.5 — Voronoï et lignes fluides (jalon 5)](roadmap/items/R-005-v0-5-voronoi-et-lignes-fluides-jalon-5.md) · livrée le 2026-11-06
  Disposition Voronoï à nervures d'épaisseur variable et disposition en lignes fluides (rubans d'épaisseur variable).

### v0.6 : Atelier de formes et morphing

- [R-006 v0.6 — Atelier de formes et morphing (jalon 6)](roadmap/items/R-006-v0-6-atelier-de-formes-et-morphing-jalon-6.md) · livrée le 2026-11-07
  Éditeur de formes (plume, Bézier, primitives cotées, symétries, calques) et canal Forme avec morphing.

## Écarté

- [R-010 Wrap — projection d'un motif sur une face 3D courbe](roadmap/items/R-010-wrap-projection-d-un-motif-sur-une-face-3d.md)
  Projeter un motif sur une face 3D courbe d'un STEP importé. Hors périmètre de la v1.0, à réévaluer après.
- [R-011 Comptes, nuage, place de marché](roadmap/items/R-011-comptes-nuage-place-de-marche.md)
  Comptes utilisateur, nuage et place de marché. Hors périmètre de la v1.0, à réévaluer après.
