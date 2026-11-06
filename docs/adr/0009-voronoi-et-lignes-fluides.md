# ADR-0009 : Voronoï et lignes fluides

Status: Accepted
Date: 2026-11-06

## Context

La v0.5 ajoute deux dispositions où la disposition donne elle-même la forme des perforations : les cellules d'un diagramme de Voronoï (motif organique, alvéoles) et les rubans des lignes fluides. Les autres dispositions posent une forme choisie dans la section Forme. Le reste du moteur (mesures exactes des nervures, découpe au bord, contours exacts, STEP) repose sur des trous décrits par une âme et un arrondi.

## Decision

- **Voronoï maison**, sans crate : cellule = boîte coupée par les demi-plans médiateurs des voisins, cherchés par anneaux de grille jusqu'à ce qu'aucun voisin plus lointain ne puisse encore couper. Germes Poisson-disc (canal Espacement), **relaxation de Lloyd pondérée** par `1 / espacement²` pour garder la densité voulue.
- **Perforation d'une cellule = âme + arrondi** : cellule réduite de la demi-nervure et du congé (convexe, réduction exacte par demi-plans), arrondie du congé. La nervure entre deux cellules vaut donc exactement la nervure demandée ; mesures, STEP (cylindres aux sommets) et arcs exacts suivent sans code spécifique.
- **Canaux** : Taille → nervure (Voronoï) ou largeur des rubans ; Espacement → taille des cellules ou écart des lignes ; Angle → direction des lignes (les contrôleurs les courbent). La courbure n'est pas un canal à part : elle vient du champ d'angle.
- **Lignes fluides** : lignes de courant (Runge-Kutta 2) semées le long de la transverse au centre de la plaque, au pas du canal Espacement ; une ligne s'arrête au bord (marge et demi-largeur), devant un trou de fixation ou une zone réservée, ou trop près d'une autre ligne (écart minimal : la moitié de l'écart, et au moins 1,25 largeur). Ruban : décalage de part et d'autre selon la largeur locale, bouts ronds ou plats, boucles fondues par `i_overlay`, sommets alignés retirés à 0,005 mm. Le ruban est un polygone (âme sans arrondi).
- **Distance entre longues âmes** (rubans) par grille de segments limitée au rayon de recherche des nervures (1,1 s → 50 ms sur 47 rubans).

## Alternatives considered

- `voronoice` / `spade` : triangulation de Delaunay complète pour quelques centaines de cellules, dépendance de plus, et les cellules devraient de toute façon être coupées par la boîte puis réduites.
- Lloyd sans pondération : cellules régulières mais densité aplatie ; la pondération garde l'effet des contrôleurs d'espacement (atténué par la relaxation, signalé dans l'infobulle).
- Lignes fluides à espacement uniforme (Jobard-Lefer complet, germes ajoutés le long des lignes) : couverture plus régulière dans les champs très courbés ; reporté, le semis transverse suffit aux motifs courants.

## Consequences

Les cellules en bord de plaque non rectangulaire sont coupées par le mode Découper ou retirées en mode Trous entiers (bord en escalier). Les bouts ronds des rubans sont facettisés (faces planes en STEP). Une zone que la transverse ne traverse pas (contour libre en L) peut rester sans lignes.

## Validation

Tests moteur : nervure minimale égale à la nervure demandée (1e-6 mm), densité plus forte près d'un contrôleur d'espacement, rubans sans chevauchement (droits et courbés), aire exacte d'un ruban, cellules qui pavent la boîte. Cinq panneaux STEP valides dans FreeCAD, au volume calculé.
