# ADR-0010 : Atelier de formes et morphing

Status: Accepted
Date: 2026-11-06

## Context

La v0.6 permet de dessiner sa propre forme de perforation (calques, nœuds, courbes, congés, symétries, union et soustraction, import SVG / DXF) et ajoute un quatrième canal, Forme, qui fait glisser les perforations vers une forme cible. Le moteur repose sur des perforations décrites par une âme et un arrondi ; il doit rester rapide sur des milliers de trous.

## Decision

- **Modèle** : `ShapeKind::Custom { layers, pivot }` dans un repère où 1 = la taille de la forme. Un calque : nœuds (position, congé, poignées de Bézier relatives), rôle union / soustraction, symétrie (miroir vertical / horizontal, radiale de 2 à 12 copies autour du pivot), réponse au canal Taille (1 suit la taille, 0 garde la taille de base). Le pivot est le point posé sur le germe, centre de rotation et d'agrandissement. Format de projet v5, additif.
- **Géométrie** : calques facettisés (flèche 0,004 mm à la taille de la perforation), Bézier cubiques, congés aux seuls coins entre deux segments droits (un disque exact = carré dont les congés valent le demi-côté), copies de symétrie soudées, calques combinés dans l'ordre par `i_overlay`. La perforation est le plus grand morceau ; pièces détachées et îlots de matière sont écartés et signalés (`shape.parts_dropped`). Polygone sans arrondi pour le reste du moteur.
- **Une fois par calcul** (`PreparedShape`) : contour en unités de taille si tous les calques suivent la taille (puis seulement déplacé, tourné, mis à l'échelle par trou), sinon mis en cache par taille au centième de millimètre.
- **Morphing** : canal Forme de 0 à 1 (cible 100 % par défaut) ; départ et cible rééchantillonnés à 128 points le long du contour, cible décalée pour minimiser l'écart (pas de vrillage) ; interpolation linéaire. À 0 et à 1, la géométrie exacte (cercle, arcs) est rendue. Le canal n'apparaît qu'avec une forme cible.
- **Mesures** : distance entre âmes à nombreux sommets bornée par les sommets qui se font face, puis segments filtrés par la boîte de l'autre âme (exact) ; grille de segments quand il reste beaucoup de paires (rubans).
- **Atelier** : fenêtre modale (une tâche qui interrompt : dessiner une forme), vue SVG par jetons, aperçu calculé par le moteur (`shape_preview`), chaque geste une étape d'annulation.

## Alternatives considered

- Arrondis exacts dans les formes dessinées (contours à arcs) : il faudrait des booléens sur arcs ; la facettisation à 0,004 mm suffit à la découpe, et les primitives restent exactes.
- Garder les îlots d'une forme comme pièces : de la matière flottante dans un trou n'a pas de sens pour une plaque découpée ; signalé plutôt que découpé.
- Morphing par correspondance de sommets : impossible entre formes de nombres de sommets différents ; le rééchantillonnage par abscisse curviligne est général.

## Consequences

Les formes dessinées et les formes intermédiaires du morphing sortent en polygones (faces planes en STEP, sans arcs). Les paramètres d'une cible simple sont ceux par défaut (une cible dessinée se règle dans l'atelier). Le contrôle complet de FreeCAD devient lent au-delà de quelques dizaines de milliers de faces planes (validation faite sur un panneau de 60 mm).

## Validation

Tests moteur : aires exactes (carré, disque par congés, soustraction, miroir, radiale, réponse, cercle de Bézier à 0,1 %), import SVG en calques, morphing exact aux deux bouts, grille de formes dessinées et morphing piloté par un contrôleur. Trois panneaux STEP valides dans FreeCAD au volume calculé. Mesures : 1 435 croix en 36 ms, morphing en 61 ms.
