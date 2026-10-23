# État du projet

**Jalon** : **v0.2 — dispositions et contrôleurs**, livrée pour essai (23/10/2026). v0.1 validée par l'utilisateur. Prochain jalon : v0.3 (contours et découpe), voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Dispositions | Grille, aléatoire (Poisson-disc à densité variable, graine), spirale, concentrique, Fibonacci, hachures croisées, chemin (rangées parallèles, congés) ; orientation selon le tracé |
| Contrôleurs | Point, ligne, courbe (congés par nœud), cercle, ellipse, polygone régulier, image ; canaux Taille, Espacement, Angle ; côté unique, prolongement, intérieur rempli, angle symétrique et par paliers, disparition aléatoire, unification portée / cible |
| Canevas | Outils Courbe et Règle, import d'image (bouton, `I`, glisser-déposer), nœuds (double-clic, Alt + clic), menu contextuel, conversion en forme exacte, onglet Chemin ; calcul différé au relâchement si le premier calcul d'un geste dépasse 60 ms |
| Plaque | Contrôleurs et chemins suivent la plaque redimensionnée ; option « Mettre le motif à l'échelle » |
| Fichiers | Format de projet v2 ; les projets v1 se lisent tels quels (test) |
| Tests | `cargo test --workspace` : 76 tests verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur ; `npm run build` |
| Vérifié | Dans le navigateur : les sept dispositions, espacement local (aléatoire), courbe fermée remplie puis convertie en ellipse, image en dégradé, chemin à congés sur 3 rangées, fentes concentriques orientées, angle symétrique, règle, redimensionnement à l'échelle. Natif : 25 000 germes aléatoires en 66 ms, grille de 43 000 trous en 8 ms |
| Non vérifié | Import réel dans LightBurn, FreeCAD, Fusion 360 ; découpe d'une pièce |

## Points ouverts

- Couleur d'identité et logo : provisoires (vert chartreuse `#B5D93A`), à valider.
- Fibonacci : l'espacement ne varie qu'avec la distance au centre (la spirale répartit chaque rayon sur tout le tour) ; un contrôleur d'espacement décentré a un effet diffus.
- Taux d'ouverture surestimé quand des trous se chevauchent (signalé) : calcul exact en v0.3.
- Motifs où tout se chevauche : la mesure des paires domine le calcul (≈ 140 ms pour 285 000 paires).

## Prochaine action

Tester la v0.2 (liste dans le message de livraison), puis lancer la v0.3.
