# Validation STEP — FreeCAD

Date : 06/11/2026. FreeCAD 1.1.1 (OpenCascade), `freecadcmd` ; script [`scripts/check_step.py`](../../scripts/check_step.py) : chaque solide passe `check(True)`, volume de l'ensemble.

Fichiers produits par `cargo run --release -p fabvents-core --example export -- <dossier> [projet.json]`. Volume attendu : (aire de la plaque − perforations − trous de fixation) × épaisseur, ou calcul analytique des troncs de cône pour la dépouille.

| Cas | Pièces | Faces | Volume FreeCAD (mm³) | Attendu (mm³) | Résultat |
| --- | --- | --- | --- | --- | --- |
| Projet d'exemple (200 × 150, 427 trous) | 1 | 858 cylindres, 6 plans | 60 379,8145 | 60 379,8145 | valide |
| Gabarit ventilateur 120 mm | 1 | 1 202 cylindres | 19 085,8070 | 19 085,807 | valide |
| 120 mm, mode Découper (78 trous coupés) | 1 | 1 350 cylindres, 94 plans | 18 509,0045 | 18 510,70 (polygones fins) | valide ; écart = arcs exacts |
| 120 mm, dépouille + 8° | 1 | 1 198 cônes | 17 290,1851 | 17 290,1851 | valide |
| 120 mm, dépouille − 8° | 1 | 1 198 cônes | 20 683,1938 | 20 683,1938 | valide |
| Étoiles, dépouille + 6° (sommets vifs, onglets) | 1 | 3 136 plans, 8 cônes | 25 850,3256 | — | valide |
| 120 mm, dalles 2 × 2 (coupes à travers les trous) | 4 | — | 19 085,8070 | 19 085,807 | chaque dalle valide ; somme exacte |
| Disque, fentes fusionnées, dalles 3 × 3 (638 pièces avec îlots) | 638 | — | 7 472,9827 | 7 480,25 (polygones fins) | valide |
| 120 mm, perforations fusionnées (1 242 îlots) | 1 243 | 3 888 cylindres, 2 574 plans | 8 383,3901 | 8 395,31 (polygones fins) | valide ; écart = arcs exacts des îlots |

Archive de dalles : `unzip -t` sans erreur ; chacun des quatre STEP valide seul, la somme des volumes égale le volume de la plaque entière. Plusieurs dalles d'un même fichier se touchent le long des coupes : le contrôle booléen global de FreeCAD le signale (contact, volume commun nul), d'où le contrôle par solide.

Défauts trouvés et corrigés pendant cette validation :

- îlots en triangle d'arcs concaves refusés (« Self-intersecting wire ») : coordonnées écrites à 7 décimales, sommet trop loin de l'intersection exacte des deux cercles ; passées à 10 décimales ;
- découpes en polygones fins (2 449 faces planes au lieu de 94) : arcs retrouvés (`fit`).

Non vérifié : Fusion 360, PrusaSlicer, SolidWorks.
