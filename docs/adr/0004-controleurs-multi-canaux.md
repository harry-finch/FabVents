# ADR-0004 : Contrôleurs multi-canaux et mélange pondéré

Status: Accepted
Date: 2026-10-23

## Context

Dans SolidVents, des contrôleurs posés sur le canevas modulent quatre canaux (taille, espacement, angle, forme) ; un contrôleur peut être « synchronisé » entre plusieurs canaux avec des portées et des valeurs propres. La recherche proposait des attracteurs à intensité `[-1, 1]` combinés par `Max`, `Add` ou `Multiply` entre une taille mini et une taille maxi.

## Decision

- Un **contrôleur** = une géométrie (point, ligne ; courbe et image en v0.2) + une **influence optionnelle par canal** (`size`, `angle`, …) : portée (mm), valeur cible (unité du canal), bord net. La synchronisation est donc native : activer un canal sur un contrôleur existant.
- Chaque canal a une **valeur de base** (taille de la forme, rotation de la forme) : c'est la valeur loin de tout contrôleur.
- Poids `w = smoothstep(1 − d / portée)` (ou 1 / 0 en bord net). Mélange : `valeur = base + max(w) · (Σ w·cible / Σ w − base)`.
- Valeurs dans l'unité du canal (mm, degrés) : « 5 mm, 8 mm près du point » plutôt qu'une intensité abstraite.

## Alternatives considered

- `Max / Add / Multiply` : `Add` dépasse les bornes, `Multiply` annule tout loin des contrôleurs, et l'utilisateur doit raisonner en intensités relatives.
- Un contrôleur par canal (copies liées) : synchronisation plus fragile à maintenir dans le projet.

## Consequences

Deux contrôleurs qui se recouvrent donnent une valeur intermédiaire, jamais au-delà de leurs cibles (testé). L'angle est mélangé linéairement (pas de repliement à 360°), suffisant pour les usages courants. Les options de la v0.2 (côté unique, remplir une courbe fermée, disparition aléatoire, paliers d'angle) s'ajouteront à l'influence ou à la géométrie.

## Validation

Tests `field` (base loin, cible sur le contrôleur, pas de dépassement, bord net, canaux indépendants) ; essai à l'écran avec un point de taille et une ligne d'angle.
