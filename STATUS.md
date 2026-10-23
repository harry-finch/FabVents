# État du projet

**Jalon** : **v0.1 — tranche verticale**, livrée pour essai (23/10/2026). Prochain jalon : v0.2 (dispositions et contrôleurs), voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Moteur | `fabvents-core` : grille carrée / hexagonale / quinconce, 5 formes (arcs exacts), contrôleurs Point et Ligne sur Taille et Angle, calcul par étages avec cache, mesures, exports SVG et DXF R12 |
| Interface | Svelte 5, mêmes jetons et composants que FabSpeed ; canevas (zoom, déplacement, poignées, portée), annuler / rétablir, FR / EN, mm / pouces, thèmes |
| Fichiers | Projet `.fabvents.json` versionné ; SVG rouvrable (projet embarqué) ; reprise automatique du dernier projet (navigateur) |
| Tests | `cargo test --workspace` : 46 tests verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur |
| Vérifié | DXF audité par ezdxf (0 erreur, mm, calques) ; SVG et DXF rendus (Inkscape, ezdxf) identiques ; dans le navigateur : 29 000 trous calculés en 17 ms, rendu à 120 images/s, glisser fluide |
| Non vérifié | Import réel dans LightBurn, FreeCAD, Fusion 360 ; découpe d'une pièce |

## Points ouverts

- Couleur d'identité et logo : provisoires (vert chartreuse `#B5D93A`), à valider.
- Taux d'ouverture surestimé quand des trous se chevauchent (signalé à l'écran) : calcul exact prévu avec la découpe au contour (v0.3).
- Les contrôleurs gardent leur position absolue quand la plaque change de taille (redimensionnement proportionnel prévu en v0.2).

## Prochaine action

Tester la v0.1 (liste de vérification dans le message de livraison), puis lancer la v0.2.
