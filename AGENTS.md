# Consignes pour les agents

> Ce fichier est repris dans CLAUDE.md.

FabVents : générateur de motifs de perforation paramétriques (grilles de ventilation, de haut-parleur, panneaux décoratifs) pour makers et fablabs, inspiré de SolidVents. Workspace Rust (`fabvents-core` pur, `fabvents-wasm` pour l'interface) + interface Svelte 5 dans `ui/`, même écosystème que FabSpeed. Lire [README.md](README.md), [STATUS.md](STATUS.md), [ROADMAP.md](ROADMAP.md) et [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) avant de modifier.

## Skills à utiliser (`.agents/skills/`)

- `fab-design-system` : toute modification de l'interface ; jetons, composants, mise en page, textes et identité de la famille Fab, avec un kit de démarrage pour une nouvelle application. Ses valeurs chiffrées font foi.
- `professional-workshop-ui` : les mêmes principes en termes généraux.
- `project-documentation` : STATUS, DEVLOG, ROADMAP, ADR, FEATURES.

## Règles

- `fabvents-core` reste pur : aucune IO, aucun réseau, aucune dépendance d'interface ; déterministe (même projet → même géométrie, aléatoire seulement par graine du projet).
- Millimètres et degrés dans le moteur et dans les fichiers ; les pouces ne sont qu'un affichage.
- Repère du moteur : origine en bas à gauche de la plaque, Y vers le haut (convention CAO). Le canevas et le SVG retournent l'axe.
- Les valeurs saisies ne sont jamais corrigées en silence : une valeur hors domaine produit une erreur ou un avertissement du moteur (code stable), pas un bridage caché. Un trou retiré (trop petit, hors zone) est compté et signalé.
- Les identifiants de `docs/FEATURES.md` sont stables ; mettre à jour l'état d'une fonction dans le même changement que son code.
- Format de projet versionné (`format`, `version`) : toute évolution incompatible ajoute une migration et un test de lecture de l'ancien format.
- Interface : aucune couleur en dur dans les composants ni dans le dessin du canevas, tout passe par les jetons de `ui/src/app.css` ; textes par `t('clé')`, `fr.ts` est la référence et `en.ts` a les mêmes clés.
- Mettre à jour la documentation dans le même changement que le comportement.

## Commandes

- Tests : `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cd ui && npm run check`.
- Moteur pour l'interface : `cd ui && npm run wasm` (wasm-pack, sortie dans `ui/src/wasm/`, hors Git).
- Développement : `cd ui && npm run dev` → http://localhost:5173 (relancer `npm run wasm` après une modification du moteur).
- Build : `cd ui && npm run build` (moteur compris), puis `npm run preview`.
