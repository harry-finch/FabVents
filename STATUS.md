# État du projet

**Jalon** : **v0.4 — export STEP**, livrée pour essai (06/11/2026). v0.1 à v0.3 validées par l'utilisateur. Prochain jalon : v0.5 (Voronoï et lignes fluides), voir [ROADMAP.md](ROADMAP.md).

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
| Fichiers | Format de projet v4 ; v1 à v3 se lisent tels quels |
| Tests | `cargo test --workspace` : 121 tests verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur ; `npm run build` |
| Vérifié | FreeCAD : neuf cas de STEP valides au volume calculé ([fiche](docs/validation/step-freecad.md)). Navigateur : gabarit 120 mm en dalles 3 × 2 (coupes affichées), dépouille 8° avec dalles refusée, dépouille 40° (595 perforations en rouge, STEP grisé avec la raison), STEP à 3° téléchargé (1 198 cônes), ZIP de 4 dalles, thème clair à 1 100 px |
| Non vérifié | STEP dans Fusion 360, PrusaSlicer, SolidWorks ; DXF réels importés ; exports relus dans LightBurn ; découpe et impression d'une pièce |

## Points ouverts

- Fibonacci : l'espacement ne varie qu'avec la distance au centre.
- Motifs où tout se chevauche : la mesure des paires et l'union dominent le calcul.
- Contour libre : la distance au bord parcourt toutes les arêtes (lent au-delà de quelques milliers de sommets).
- Importer un contour garde les éléments de plaque existants (ceux d'un gabarit peuvent alors déborder : signalé, et l'export STEP est refusé).
- Le taux d'ouverture est mesuré sur les polygones fins des découpes : jusqu'à 0,1 % d'écart avec le volume exact du STEP sur les motifs très fusionnés.
- Dépouille et dalles ensemble : seulement si toutes les coupes trouvent un intervalle libre entre les perforations.

## Prochaine action

Tester la v0.4 (liste dans le message de livraison), en particulier l'import du STEP dans Fusion 360 et PrusaSlicer.
