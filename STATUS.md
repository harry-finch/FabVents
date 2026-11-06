# État du projet

**Jalon** : **v0.6 — atelier de formes et morphing**, livrée pour essai (06/11/2026). v0.1 à v0.5 validées par l'utilisateur. Prochain jalon : v0.7 (bureau et automatisation), voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Contours | Rectangle arrondi, disque, ellipse, contour libre importé (SVG, DXF) ou tracé (courbe fermée → « Utiliser comme contour ») ; contours intérieurs importés en trous découpés |
| Zone perforée | Plaque entière, disque ou rectangle arrondi, réduite de la marge |
| Bords | Par le centre, trous entiers, **découper** (booléens `i_overlay`) ; fusion des trous qui se chevauchent ; îlots comptés et signalés |
| Éléments de plaque | Onglet Plaque (touche `5`) : trous de fixation (découpés) et zones réservées (disque, rectangle, tracé), garde réglable ; signalés s'ils débordent de la plaque |
| Gabarits | Grilles de ventilateur 40 / 60 / 80 / 92 / 120 / 140 mm (menu Fichier) |
| Mesures | Taux d'ouverture exact (chevauchements, découpe, îlots) |
| Exports | SVG, DXF R12, projet, PNG 300 dpi, **STEP** AP214 (B-Rep explicite : cylindres, cônes en dépouille), **STEP par dalle** (ZIP) ; découpes en arcs exacts dans tous les exports |
| Export STEP | Fabrication → Export STEP : dépouille (± 45°), dalles colonnes × rangées (1 à 20) ; perforations en cause en rouge, coupes des dalles en pointillés ; export refusé avec la raison si la dépouille ferme ou perce une paroi, si une coupe traverse une paroi en dépouille, si un trou de fixation déborde |
| Dispositions v0.5 | **Voronoï** (taille des cellules, nervure, congé, relaxation de Lloyd pondérée, tirage) ; **lignes fluides** (écart, largeur, direction, bouts ronds ou plats) ; canaux Taille, Espacement et Angle repris (nervure / largeur, densité / écart, direction) |
| Formes v0.6 | **Forme dessinée** (atelier : calques union / soustraction, primitives cotées, nœuds, Bézier, congés, symétries miroir et radiale, réponse au canal Taille, pivot, import SVG / DXF) ; **canal Forme** (touche 6) : morphing vers une forme cible simple ou dessinée |
| Fichiers | Format de projet v5 ; v1 à v4 se lisent tels quels |
| Tests | `cargo test --workspace` : 136 tests verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur ; `npm run build` |
| Vérifié | FreeCAD : STEP des v0.4 et v0.5, et trois panneaux de formes dessinées / morphing valides au volume calculé. Navigateur : passage d'un cercle à une forme dessinée, calque étoile en soustraction (îlot signalé, puis encoches quand une branche sort), forme cible Cercle et contrôleur du canal Forme. Mesures natives : 1 435 croix en 36 ms, morphing en 61 ms |
| Non vérifié | STEP dans Fusion 360, PrusaSlicer, SolidWorks ; DXF réels importés ; exports relus dans LightBurn ; découpe et impression d'une pièce |

## Points ouverts

- Fibonacci : l'espacement ne varie qu'avec la distance au centre.
- Motifs où tout se chevauche : la mesure des paires et l'union dominent le calcul.
- Contour libre : la distance au bord parcourt toutes les arêtes (lent au-delà de quelques milliers de sommets).
- Importer un contour garde les éléments de plaque existants (ceux d'un gabarit peuvent alors déborder : signalé, et l'export STEP est refusé).
- Le taux d'ouverture est mesuré sur les polygones fins des découpes : jusqu'à 0,1 % d'écart avec le volume exact du STEP sur les motifs très fusionnés.
- Dépouille et dalles ensemble : seulement si toutes les coupes trouvent un intervalle libre entre les perforations.
- Voronoï : la relaxation de Lloyd atténue les variations de densité (deux fois plus de cellules près d'un contrôleur qui divise l'espacement par deux, au lieu de quatre) ; bord en escalier hors plaque rectangulaire sans le mode Découper.
- Lignes fluides : semées sur la transverse au centre ; une zone qu'elle ne traverse pas reste vide ; bouts ronds facettisés dans le STEP.
- Formes dessinées et formes intermédiaires du morphing : polygones facettisés à 0,004 mm (pas d'arcs dans le DXF ni le STEP) ; îlots et pièces détachées écartés (signalé) ; une cible simple prend ses paramètres par défaut.
- Atelier : pas d'outil plume à main levée (ajout de nœuds par double-clic sur un bord).

## Prochaine action

Tester la v0.6 (liste dans le message de livraison), puis lancer la v0.7.
