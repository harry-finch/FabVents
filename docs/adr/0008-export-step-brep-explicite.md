# ADR-0008 : Export STEP en B-Rep explicite, dépouille et dalles

Status: Accepted
Date: 2026-11-06

## Context

La v0.4 exporte la plaque en volume (STEP) pour la CAO et l'impression 3D, avec deux fonctions de SolidVents : l'angle de dépouille des parois et le découpage en dalles. La recherche proposait `EXTRUDED_AREA_SOLID` (géométrie constructive, peu lue par les importeurs AP214) ou le noyau `truck`. Les perforations sont déjà décrites exactement (cercles, segments et arcs à *bulge*), sauf les découpes du mode Découper et les fusions, sorties des booléens en polygones fins (0,002 mm).

## Decision

- **Générateur STEP maison** (AP214, `MANIFOLD_SOLID_BREP`) : faces du dessus et du dessous planes ; une paroi par segment, plane pour un segment droit, cylindrique pour un arc, conique pour un arc en dépouille. Cercles complets coupés en deux demi-arcs. Boucles orientées matière à gauche (`brep`). Coordonnées à 10 décimales pour rester sous l'incertitude déclarée (1e-7 mm).
- **Dépouille** sur les seules perforations (pas le contour de la plaque) : le fond de chaque boucle est décalé de `épaisseur × tan(angle)` côté matière, arcs concentriques, sommets vifs en onglet. Le moteur refuse l'export si une perforation se referme ou perd son arrondi (`draft.collapse`), ou si, au dessous, elle touche une voisine, un trou de fixation ou le bord (`draft.pierce`, croissance prudente par perforation) ; les perforations en cause sont surlignées sur le canevas.
- **Dalles** : coupes de préférence dans un intervalle libre entre les perforations, à moins d'un quart de dalle des coupes régulières ; sinon à travers, à mi-chemin des bords des perforations (pas de copeau). Une coupe découpe toutes les boucles à la fois (contour, îlots, trous) avec des encoches exactes. Une coupe à travers une paroi en dépouille aurait une section non plane : refusée (`tiles.draft`).
- **Contours exacts des découpes** (`fit`) : arcs retrouvés sur les cercles connus, raccords recalés sur les intersections exactes, segments alignés fusionnés. Profite aussi au SVG, au DXF et au canevas.
- Un seul fichier (une pièce par dalle, îlots à part) ou une archive ZIP stockée (un STEP par dalle, `<projet>-c<colonne>r<rangée>.step`).
- Vérification par FreeCAD (`scripts/check_step.py`) : validité de chaque solide et volume, sur une matrice de cas ([fiche](../validation/step-freecad.md)).

## Alternatives considered

- `truck` : noyau NURBS complet, mais il aurait fallu le même travail de dalles et de dépouille, une dépendance lourde en WebAssembly, et des arcs approchés en NURBS.
- Garder les découpes en polygones : fichiers dix fois plus gros (28 000 faces planes sur un motif dense), arcs perdus en CAO.
- Dalles toujours entre les perforations : impossible sur une grille en quinconce, la disposition la plus courante.

## Consequences

Les fichiers s'ouvrent dans FreeCAD avec des solides valides au volume calculé ; Fusion 360 et PrusaSlicer restent à confirmer. Le taux d'ouverture affiché reste mesuré sur les polygones fins (écart de l'ordre de 0,1 % avec le volume exact sur les motifs très fusionnés). Dépouille et dalles ensemble ne marchent que si les coupes trouvent des intervalles libres.

## Validation

121 tests (moteur, pont WebAssembly) ; fiche FreeCAD de neuf cas ; export depuis l'interface (STEP avec dépouille, ZIP de quatre dalles).
