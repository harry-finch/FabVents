---
id: R-001
titre: "v0.1 — Tranche verticale (jalon 1)"
resume: "Dessiner une grille de ventilation à trous de taille variable et l'envoyer à la découpeuse laser, de bout en bout."
statut: livree
jalon: v0.1
cree: 2026-10-23
maj: 2026-11-12
livree: 2026-10-23
---

**But** : dessiner une grille de ventilation à trous de taille variable et l'envoyer à la découpeuse laser, de bout en bout.

Inclus :

- Workspace Rust (`fabvents-core`, `fabvents-wasm`), interface Svelte, moteur en WebAssembly dans un Web Worker (ADR-0002).
- Plaque rectangulaire (coins arrondis) ou ronde, épaisseur, marge de bord.
- Disposition **Grille** : carrée, hexagonale (60°), quinconce ; pas X/Y, angle, décalage, rotation par rangée / colonne.
- Formes : cercle, rectangle (coins arrondis), polygone régulier, étoile, fente ; taille de référence et rotation.
- Contrôleurs **Point** et **Ligne** sur les canaux **Taille** et **Angle** : portée, valeur cible, bord net ; un contrôleur peut agir sur plusieurs canaux (synchronisation, ADR-0004).
- Bords : par le centre, trous entiers.
- Mesures : nombre de trous, taux d'ouverture, tailles min / max, nervure minimale, chevauchements ; avertissements de fabrication (nervure sous le minimum, trous supprimés car trop petits).
- Canevas : zoom à la molette centré sur le curseur, déplacement (Espace + glisser, bouton du milieu, outil Main), cadrage, poignées déplaçables, création de points et de lignes, suppression, annuler / rétablir.
- Exports : **SVG** (projet embarqué, rouvrable), **DXF** R12 en mm (calques `CUT_INNER` / `CUT_OUTER`), projet `.fabvents.json`. Ouverture d'un projet ou d'un SVG FabVents (glisser-déposer).
- Reprise automatique du dernier projet ; FR / EN ; mm / pouces ; thème.

Critères d'acceptation :

- `cargo test --workspace`, `cargo clippy --workspace --all-targets` et `npm run check` sans erreur.
- Une grille 200 × 150 mm de ~2 000 trous se recalcule pendant le déplacement d'un contrôleur sans saccade visible.
