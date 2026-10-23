# ADR-0001 : Workspace Rust et interface Svelte de l'écosystème FabSpeed

Status: Accepted
Date: 2026-10-23

## Context

FabVents reprend les fonctions de SolidVents pour des makers. La recherche préalable proposait une application `egui` / `eframe` (canevas GPU, poignées). La demande impose d'être dans le même écosystème que FabSpeed : même ergonomie (skill `professional-workshop-ui`), même impression visuelle. FabSpeed et FabCAM utilisent Rust pour le calcul et Svelte 5 pour l'interface, Tauri 2 pour le bureau.

## Decision

- Workspace Rust : `fabvents-core` (moteur pur, sans IO ni interface) et `fabvents-wasm` (pont vers l'interface).
- Interface Svelte 5 + Vite + TypeScript dans `ui/`, icônes Lucide, police Inter, jetons et composants de base repris de FabSpeed (seule la couleur d'identité change).
- Application de bureau Tauri 2 au jalon v0.7, sur la même interface.

## Alternatives considered

- `egui` : un seul langage, rendu GPU, mais widgets, typographie et comportement différents de FabSpeed : l'impression d'écosystème commun serait perdue et aucun composant ne serait partagé.
- Interface web sans framework : plus de code à maintenir pour les panneaux, l'internationalisation et l'état.

## Consequences

Deux langages (Rust, TypeScript). Le canevas est dessiné en Canvas 2D (WebGL possible plus tard) : suffisant pour ~30 000 trous à 120 images/s (mesuré). Les composants partagés avec FabSpeed sont copiés, pas encore mis en bibliothèque commune.

## Validation

Comparaison visuelle avec FabSpeed (mêmes surfaces, mêmes contrôles, même barre d'application) ; tenue des 60 images/s pendant le déplacement d'un contrôleur sur une grande grille.
