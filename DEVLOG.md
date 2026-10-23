# Journal de développement

## 2026-10-23 — v0.1 : tranche verticale

**Entrées.** Conversation de recherche sur SolidVents (`docs/research/2026-10-22-conversation-gemini-solidvents.md`, hors dépôt) : inventaire des fonctions et pistes techniques. FabSpeed comme référence d'ergonomie et d'architecture (skills `professional-workshop-ui`, `project-documentation`).

**Choix.**
- Interface Svelte 5 au lieu de l'`egui` proposé par la recherche, pour être dans le même écosystème que FabSpeed (ADR-0001).
- Moteur Rust compilé en WebAssembly et exécuté dans un Web Worker : pas de serveur, l'interface ne se fige jamais, version web statique possible (ADR-0002).
- Trous décrits par une âme (point, segment, polygone) et un arrondi : aires, nervures et contours exacts, cercles et arcs conservés dans les exports (ADR-0005).
- Mélange pondéré des contrôleurs vers leur valeur cible plutôt que les modes `Max / Add / Multiply` de la recherche (ADR-0004).
- DXF R12 plutôt que l'AC1015 minimal de la recherche, refusé par AutoCAD sans tables complètes (ADR-0003). `EXTRUDED_AREA_SOLID` écarté pour le STEP à venir (peu lu par les importeurs AP214).
- Valeurs hors domaine signalées par code (`value.must_be_positive`, `layout.too_many_holes`…), jamais bridées en silence ; trous trop petits retirés et comptés.

**Vérification.**
- 46 tests (moteur, pont WebAssembly). DXF de l'exemple et d'une plaque ronde à fentes audités par ezdxf (R12, `$INSUNITS = 4`, calques, 0 erreur) ; SVG rendus par Inkscape (120 mm → 453,5 px, échelle juste) et DXF rendus par ezdxf : orientation et arcs identiques.
- Dans le navigateur (onglet de développement) : déplacement d'un point de taille, ligne d'angle, passage en fente, annuler / rétablir, thème clair, anglais, exports SVG / DXF / projet (contenus inspectés), relecture d'un SVG exporté, plaque ronde. Grande grille 600 × 400 mm au pas de 3 mm : 29 121 trous calculés en 17 ms, rendu sous 8 ms par image.

**Défauts corrigés pendant la vérification.**
- Vue cadrée sur la taille initiale du panneau : le canevas recadre tant que l'utilisateur n'a ni zoomé ni déplacé la vue.
- Sélecteur de motif tronqué à 1 100 px : passé sur toute la largeur du panneau.
- Avertissement Svelte `assignment_value_stale` au changement de forme.
- Taux d'ouverture affiché à 238 % sur une grille qui se chevauche : valeur conservée mais signalée (couleur d'avertissement, infobulle).

**Non fait.** Import dans LightBurn, FreeCAD et Fusion 360 ; découpe réelle.

## 2026-10-23 — v0.2 : dispositions et contrôleurs

v0.1 testée et validée par l'utilisateur.

**Choix.**
- Format de projet v2, purement additif : les fichiers v1 se lisent sans migration (test `version_1_files_are_read`).
- Options géométriques sur le contrôleur (côté, prolongement, remplissage), options de canal sur l'influence (inversion et contraste d'image, symétrie et paliers d'angle, disparition aléatoire) : ADR-0006.
- Aléatoire : Bridson à rayon variable, deux points au moins à la moyenne de leurs espacements locaux ; graine SplitMix64. Disparition aléatoire tirée par position (au micromètre) : une perforation garde son tirage quand le reste bouge.
- Fibonacci à espacement variable : chaque point compte `√3/2 · s²` d'aire, le rayon suit l'aire cumulée ; densité juste à 3 % près (test), variation radiale seulement.
- Hachures croisées refaites en treillis (fentes au milieu des arêtes) : la première version, deux familles décalées d'un demi-pas, superposait des fentes aux croisements (vu au rendu).
- Images : luminance réduite à 200 px de côté au plus, en base64 dans le projet (≈ 40 Ko), décodée dans le moteur (décodeur base64 maison, sans dépendance).
- Interface : le canevas édite des « éléments » (contrôleurs du canal, ou tracés de l'onglet Chemin) ; module `geometry.ts` pour les poignées, distances, déplacements et conversions ; calcul différé décidé sur le premier calcul du geste en cours (et non sur le dernier calcul, qui pouvait inclure une disposition lente).

**Vérification.** 76 tests ; rendus Inkscape des cinq nouvelles dispositions ; parcours dans le navigateur (voir STATUS). Mesures natives : `cargo run --release -p fabvents-core --example bench -- projet.json`.

**Défauts corrigés pendant la vérification.**
- Points d'un anneau concentrique sous l'arc (polyligne) : ramenés sur le cercle exact.
- Nombre de points le long d'un chemin sensible à l'arrondi (8 espacements donnaient 8 points au lieu de 9) : marge d'arrondi.
- Image étirée par le redimensionnement de la plaque : elle suit désormais la plaque par son centre et garde ses proportions.
- Libellés tronqués ou sur deux lignes (cible, départ, rotation, côté, nom de disposition).

**Non fait.** Import dans LightBurn, FreeCAD et Fusion 360 ; découpe réelle.

## 2026-10-23 — v0.3 : contours et découpe

v0.2 testée et validée par l'utilisateur.

**Choix** (ADR-0007).
- Booléens `i_overlay` limités aux trous qui se chevauchent ou dépassent de la zone en mode Découper ; les autres trous gardent leurs cercles et arcs exacts.
- Plaque, zone perforée et éléments de plaque décrits en régions âme + arrondi : contenance et écarts exacts, arêtes vérifiées pour les contours concaves.
- Zone perforée séparée du contour (disque ou rectangle), utile aux grilles de ventilateur.
- Import SVG (`roxmltree`, `svgtypes` : transformations, unités mm / px) et DXF (polylignes à *bulge*, cercles, ellipses, lignes et arcs chaînés, `$INSUNITS`) : le plus grand contour fermé devient la plaque, les autres des trous découpés.
- Gabarits de ventilateur d'après les entraxes standard (32 à 124,5 mm) : plaque carrée, zone circulaire, 4 trous de fixation à garde 1,5 mm, grille hexagonale.
- PNG rendu par l'interface (Canvas 2D, règle *evenodd* pour les îlots), 300 dpi plafonné à 8 192 px.

**Vérification.** 97 tests ; rendus Inkscape du gabarit 120 mm et du mode Découper ; parcours dans le navigateur (voir STATUS).

**Défauts corrigés pendant la vérification.**
- Aire d'un disque découpé à 2 825 mm² au lieu de 2 827 : facettisation de la découpe passée de 0,01 à 0,002 mm.
- Libellés « Par le centre » / « Trous entiers » et sélecteur de zone qui débordaient du panneau : libellés courts (« Centre », « Entiers »), titre de zone au-dessus du sélecteur.
- Trous de fixation d'un gabarit hors de la plaque après import d'un contour : nouvel avertissement `features.outside`.

## 2026-10-23 — v0.3 validée, identité retenue

v0.3 testée et validée par l'utilisateur. Logo et couleur d'identité (vert chartreuse `#B5D93A`) retenus. Dépôt publié sur GitHub (public, GPL-3.0-or-later).
