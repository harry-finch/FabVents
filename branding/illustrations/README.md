# Figures de FabVents

| Figure | Fichier | Concept |
| --- | --- | --- |
| `fan-grille` | `fan-grille.js` (page d'essai : `hairline-fan-grille.html`) | Plaque de ventilateur de 120 mm, ouverture ronde, quatre barres concentriques et quatre rayons. Au repos, les barres s'amincissent vers le centre et le moyeu est le seul trait vif. Le pointeur amincit la barre visée, puis ses voisines, par vagues : l'ouverture s'élargit là où il est. Le curseur règle le décalage (0, 40, 90 ms). |

- Concept choisi par l'utilisateur le 2026-11-12, parmi : plaque perforée, grille de ventilateur, plaque en Voronoï.
- Dessinée avec la skill `hairline-create`, contrôlée par `look.mjs` (cadre, console, lisibilité à 240 px, deux thèmes : sans défaut). Le parcours (`tour`) n'a pas été jugé par l'utilisateur : ouvrir la page avec `?play=1`.
- Emplacement dans l'application : pas encore posée, voir R-012.
- Régénérer la page : `node ../../.agents/skills/hairline-create/build.mjs fan-grille.js`.
