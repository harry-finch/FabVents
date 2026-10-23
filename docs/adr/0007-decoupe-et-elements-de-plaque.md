# ADR-0007 : Découpe au contour et éléments de plaque

Status: Accepted
Date: 2026-10-23

## Context

La v0.3 ajoute la découpe au contour (trous coupés par le bord de la zone perforée), la fusion des trous qui se chevauchent, les contours libres (ellipse, import SVG / DXF, tracé), et deux ajouts propres à FabVents : trous de fixation et zones réservées, gabarits de ventilateur. Jusqu'ici tout trou restait une âme + un arrondi exacte (ADR-0005), ce qui garde cercles et arcs dans les exports et rend le calcul rapide.

## Decision

- **Booléens seulement là où il le faut** : `i_overlay` (Rust pur, compilé en WebAssembly) ne traite que les trous qui se chevauchent, ou qui dépassent de la zone en mode Découper. Les autres restent des contours exacts. Facettisation de la découpe à 0,002 mm.
- **Régions** : plaque, zone perforée, trous de fixation et zones réservées partagent la représentation des trous (âme + arrondi, drapeau convexe). La contenance d'un trou et l'écart à un élément restent exacts ; un polygone concave vérifie ses arêtes.
- **Zone perforée** distincte du contour de la plaque (plaque entière, disque, rectangle arrondi), réduite de la marge.
- **Éléments de plaque** (`features`) : disque, rectangle ou tracé fermé, avec une garde ; `cut` = découpé (trou de fixation) ou simple zone sans perforation. Ils retirent les perforations trop proches ; en mode Découper, ils sont soustraits des découpes.
- **Îlots** : une matière entièrement entourée de perforations est comptée et signalée (elle tombe à la découpe), jamais supprimée en silence.
- **Import de contour** : le plus grand contour fermé devient la plaque, les contours intérieurs deviennent des éléments découpés ; origine ramenée au coin bas-gauche, Y vers le haut.
- **Format de projet** en version 3, purement additif.

## Alternatives considered

- Booléens sur tous les trous : arcs perdus dans les exports et calcul bien plus lent sur les grandes grilles.
- `geo` / `geo-clipper` : `geo` tire beaucoup de dépendances, Clipper est en C++ (chaîne WebAssembly plus lourde).
- Supprimer automatiquement les îlots : le résultat changerait sans que l'utilisateur le voie.

## Consequences

Un trou fusionné ou coupé sort en polyligne facettisée (DXF : POLYLINE, SVG : `path`). Le taux d'ouverture devient exact dans tous les cas. Les motifs où tout se chevauche restent coûteux (mesure des paires puis union).

## Validation

Tests moteur : fusion de deux carrés, découpe par la zone, îlot dans un anneau, soustraction d'une zone réservée, aire découpée d'un disque à 5·10⁻⁴ près, gabarit 120 mm, import SVG (arcs, transformations, unités) et DXF (*bulge*, lignes chaînées), trou de fixation hors plaque signalé.
