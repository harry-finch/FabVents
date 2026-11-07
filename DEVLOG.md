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

## 2026-10-23 — Skill `fab-design-system`

Thème, style graphique et philosophie UX de la famille Fab réunis dans une skill réutilisable (`.agents/skills/fab-design-system/`), tirée du code de FabSpeed et de FabVents : dix références (philosophie, jetons sombre / clair, typographie, mise en page, composants, interactions, textes et langues, canevas, identité et logo, revue) et un kit de démarrage Svelte 5 (jetons, thème, langues, unités, infobulles, onze composants) qui passe `svelte-check` et `vite build`. Relevé au passage : les logos de FabCAM (bleu) et de KiPartManager (ambre) sont proches des couleurs de sens ; la skill demande une teinte distincte pour toute nouvelle application.

## 2026-11-06 — v0.4 : export STEP

**Choix** (ADR-0008).
- Générateur STEP maison en B-Rep explicite plutôt que `truck` : faces planes, cylindriques et coniques exactes, cercles en deux demi-arcs.
- Dépouille sur les perforations : fond décalé côté matière, contrôles `draft.collapse` / `draft.pierce` avec les perforations en cause sur le canevas.
- Dalles : coupes entre les perforations si possible, sinon à travers avec des encoches exactes (une grille en quinconce n'a aucune coupe libre) ; refusées à travers une paroi en dépouille (`tiles.draft`).
- Arcs retrouvés sur les découpes (`fit`) : le mode Découper passe de 2 449 à 94 faces planes, et le SVG et le DXF y gagnent des arcs vrais. Congés des tracés exacts (`curve::contour`).
- Format de projet v4 (dépouille et dalles dans `fabrication`), additif.

**Vérification.** 121 tests ; neuf cas validés dans FreeCAD ([fiche](docs/validation/step-freecad.md)) ; parcours dans le navigateur (voir STATUS).

**Défauts corrigés pendant la vérification.**
- Coupes de dalles introuvables sur une grille en quinconce (premier essai : coupes seulement entre les perforations) : coupe de toutes les boucles à la fois, à travers les perforations.
- Îlots en triangle d'arcs concaves refusés par FreeCAD : coordonnées à 10 décimales au lieu de 7.
- Raccords entre deux arcs restés en petits segments : tolérance « sur le cercle » portée à 3 µm et raccords recalés sur l'intersection exacte.
- Trou de fixation à cheval sur le bord : solide invalide ; export STEP refusé (`export.step_features_outside`).

## 2026-11-06 — v0.4 validée ; v0.5 : Voronoï et lignes fluides

v0.4 validée par l'utilisateur.

**Choix** (ADR-0009).
- Voronoï maison par demi-plans, relaxation de Lloyd pondérée par la densité ; perforation = cellule réduite (âme convexe) + congé (arrondi) : nervures exactes, STEP à cylindres aux sommets sans code dédié.
- Lignes fluides : lignes de courant semées sur une transverse, écart minimal entre lignes, rubans de largeur variable nettoyés par `i_overlay`.
- La disposition peut imposer le contour d'une perforation (`Seed::shape`) ; les canaux Taille, Espacement et Angle y règlent nervure / largeur, densité / écart, direction.

**Vérification.** 129 tests ; cinq panneaux STEP valides dans FreeCAD ; rendus Inkscape ; parcours dans le navigateur ; mesures natives (930 cellules en 23 ms).

**Défauts corrigés pendant la vérification.**
- Cellules manquantes le long d'un bord : la boîte des cellules, élargie de la demi-nervure, plaçait la perforation exactement sur la marge (test « trou entier » au millionième) ; boîte ramenée à la zone perforable.
- Mesure des nervures entre rubans en 1,1 s (toutes les paires de segments) : grille de segments limitée au rayon de recherche, 50 ms.
- Rubans qui se chevauchaient quand la largeur augmente : écart minimal entre lignes porté à 1,25 largeur.

## 2026-11-06 — v0.5 validée ; v0.6 : atelier de formes et morphing

v0.5 validée par l'utilisateur.

**Choix** (ADR-0010).
- Forme dessinée par calques (union, soustraction, symétries, réponse au canal Taille, pivot), combinée par `i_overlay` ; préparée une fois par calcul (`PreparedShape`), puis seulement placée par trou.
- Canal Forme : contours rééchantillonnés à 128 points et appariés, exacts aux deux bouts.
- Atelier de formes en fenêtre modale, aperçu calculé par le moteur ; chaque geste est une étape d'annulation.
- Format de projet v5, additif.

**Vérification.** 136 tests ; trois panneaux STEP valides dans FreeCAD ; parcours dans le navigateur ; mesures natives.

**Défauts corrigés pendant la vérification.**
- Mesure des nervures en 260 ms (croix) et 820 ms (morphing) : borne par les sommets en vis-à-vis et filtrage des segments, 24 ms et 51 ms ; grille de segments gardée pour les rubans.
- Disque obtenu par congés 0,24 % trop petit : flèche des calques ramenée de 0,01 à 0,004 mm.
- Rangées de formes qui passaient à la ligne dans le panneau (six et sept boutons) ; étiquette du canevas du canal Forme en mm au lieu de %.
- Contrôle FreeCAD complet trop lent sur 150 000 faces planes : validation sur un panneau de 60 mm.

## 2026-11-07 — v0.6 validée ; v0.7 : bureau et automatisation

v0.6 validée par l'utilisateur.

**Choix** (ADR-0011).
- Solveur de taux d'ouverture dans le moteur : dichotomie sur la taille de base ou l'espacement, sans jamais passer sous la nervure mini ; meilleur réglage rendu et signalé si la cible est hors d'atteinte.
- Préréglages dans l'interface (bibliothèque du poste, fichier `fabvents.presets`), adaptés à la plaque en cours.
- Crate `fabvents-cli` : binaire `fabvents` (info, export, solve, template) et serveur MCP sur stdio, sans framework ; aperçu PNG par `tiny-skia`.
- Dispositions scriptées en Rhai (bac à sable), curseurs déclarés par le script ; messages du moteur avec un champ `detail`.
- Application de bureau Tauri 2 : l'interface et le moteur WebAssembly tels quels, commandes Rust pour les fichiers.

**Vérification.** 151 tests ; parcours dans le navigateur ; CLI et session MCP réelle ; application de bureau construite.

**Défauts corrigés pendant la vérification.**
- Préréglage non appliqué : le projet venait d'une liste réactive (proxy Svelte) que `structuredClone` refuse ; instantané à la place, et geste d'annulation fermé même en cas d'erreur.
- Cible du canal Forme mise à l'échelle comme une longueur au redimensionnement (100 % → 141 %) : seules les cibles Taille et Espacement suivent l'échelle.
- Moteur WebAssembly à 3,2 Mo avec Rhai : fermetures et entiers 32 bits retirés, 2,8 Mo.
- CLI Tauri 2.12 incompatible avec Rust 1.88 : 2.11.5 épinglée.
