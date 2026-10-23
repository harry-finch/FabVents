# ADR-0005 : Trous décrits par une âme et un arrondi

Status: Accepted
Date: 2026-10-23

## Context

Les trous doivent être dessinés vite, mesurés juste (aire, nervure entre voisins) et exportés sans facettisation (cercles et congés exacts en DXF, SVG et plus tard STEP). La recherche proposait des polygones de 64 sommets puis une simplification, et une distance entre polygones coûteuse.

## Decision

Chaque trou est une **âme** (point, segment ou polygone convexe — l'étoile, non convexe, n'a pas d'arrondi) plus un **arrondi** `r` : son contour est la somme de Minkowski de l'âme et d'un disque de rayon `r`.

| Forme | Âme | Arrondi |
| --- | --- | --- |
| Cercle | centre | rayon |
| Fente | segment | demi-largeur |
| Rectangle à coins arrondis | rectangle réduit de `r` | rayon de coin |
| Polygone, étoile | polygone | 0 |

- Contour exact (`contour::Contour`) : cercle, ou sommets avec arcs en *bulge* — repris tel quel par le DXF (code 42) et le SVG (commande `A`).
- Aire exacte par formule.
- Nervure entre deux trous = distance entre âmes − arrondis : exacte, en O(nombre d'arêtes des âmes) (16 paires de segments pour deux rectangles).
- Test « trou entier dans la zone » : sommets de l'âme à `marge + r` du bord, exact pour une zone convexe (rectangle arrondi, disque).

## Alternatives considered

- Polygones facettisés partout : mesures approchées, exports lourds, congés perdus.
- Formes analytiques au cas par cas : une distance par paire de types, difficile à étendre.

## Consequences

Les formes libres (éditeur de formes, import SVG, v0.6) et les contours non convexes (v0.3) demanderont des polygones généraux et des booléens 2D (`i_overlay`) ; l'âme arrondie reste le chemin rapide des formes de base.

## Validation

Tests : aires exactes de toutes les formes, nervure exacte entre fentes, rectangle à coins pleins réduit à une fente ; audit des exports.
