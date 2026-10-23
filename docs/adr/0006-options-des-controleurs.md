# ADR-0006 : Options des contrôleurs

Status: Accepted
Date: 2026-10-23

## Context

La v0.2 ajoute aux contrôleurs les options de SolidVents : côté unique, prolongement à l'infini, remplissage d'un tracé fermé, image (inversion, contraste), angle symétrique et par paliers, disparition aléatoire, unification des portées et des cibles. Certaines options concernent la géométrie (le même tracé sur tous les canaux), d'autres un seul canal.

## Decision

- **Sur le contrôleur** (communes à ses canaux) : `side` (deux côtés, gauche, droite — ligne et courbe ouverte), `extend` (premier et dernier segments prolongés), `fill` (tracé fermé : distance nulle à l'intérieur). Champs omis quand ils valent leur défaut.
- **Sur l'influence** (propres à un canal) : `invert` et `contrast` (image), `symmetric` et `step` (angle), `cull` (taille : part, répartition centre / bord / uniforme, graine).
- **Image** : une géométrie comme les autres (`at`, `width`, `height`, `raster` en base64), poids = obscurité contrastée, sans portée ; le seuil franc devient un seuil à 50 %.
- **Symétrie** : la cible devient `base ± (cible − base)` selon le côté (droite / gauche d'un point, gauche / droite d'un tracé ouvert, intérieur / extérieur d'un tracé fermé).
- **Paliers** : la valeur finale est arrondie au plus petit pas des contrôleurs qui agissent au point.
- **Disparition** : tirage `[0, 1)` attaché à la position (au micromètre) et à la graine ; probabilité `part × proximité` (centre), `part × (1 − proximité)` (bord) ou `part` (uniforme), proximité linéaire `1 − d / portée`.
- **Unifier** est une action de l'interface (copie de la valeur), pas un état du projet.

## Alternatives considered

- Options toutes sur l'influence : le même tracé pourrait être d'un côté pour la taille et des deux pour l'angle, au prix d'une interface plus lourde ; SolidVents attache ces options au tracé.
- Image stockée à part (fichier lié) : le projet ne serait plus autonome ; 40 Ko en base64 restent raisonnables.
- Disparition tirée par numéro de perforation : le tirage changerait dès qu'un trou apparaît ou disparaît ailleurs.

## Consequences

Le format passe en version 2, sans migration (champs nouveaux optionnels). Les projets avec images grossissent (jusqu'à ~90 Ko pour 256 × 256 px) et l'historique d'annulation garde des copies complètes.

## Validation

Tests `field` (côté, prolongement, remplissage, symétrie et paliers, image, disparition reproductible et proportionnelle), `pipeline` (disparition dans un motif), `validate` (géométries et options invalides) ; essais dans le navigateur.
