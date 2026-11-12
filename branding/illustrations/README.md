# Figures de FabVents

| Figure | Fichier | Concept |
| --- | --- | --- |
| `perforated-plate` | `perforated-plate.js` (page d'essai : `hairline-perforated-plate.html`) | Plaque carrée de 7 × 7 perforations rondes, plus grandes vers le centre comme dans le logo. Au repos, le trou central est le seul trait vif. Le pointeur gonfle les trous autour de lui, d'autant moins qu'ils sont loin, et ils reviennent quand il part. Le curseur règle la portée (26, 38, 54). |

- Concept choisi par l'utilisateur le 2026-11-12 (second choix, après essai d'une grille de ventilateur, abandonnée), parmi : plaque perforée, grille de ventilateur, plaque en Voronoï.
- Dessinée avec la skill `hairline-create`, contrôlée par `look.mjs` (cadre, console, lisibilité à 240 px, deux thèmes : sans défaut). Le parcours (`tour`) n'a pas été jugé par l'utilisateur : ouvrir la page avec `?play=1`.
- Emplacement dans l'application : pas encore posée, voir R-012.
- Régénérer la page : `node ../../.agents/skills/hairline-create/build.mjs perforated-plate.js`.
