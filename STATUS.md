# État du projet

**Jalon** : **v0.3 — contours et découpe**, validée par l'utilisateur (23/10/2026), comme v0.1 et v0.2. Prochain jalon : v0.4 (export STEP), voir [ROADMAP.md](ROADMAP.md).

| Élément | État |
| --- | --- |
| Contours | Rectangle arrondi, disque, ellipse, contour libre importé (SVG, DXF) ou tracé (courbe fermée → « Utiliser comme contour ») ; contours intérieurs importés en trous découpés |
| Zone perforée | Plaque entière, disque ou rectangle arrondi, réduite de la marge |
| Bords | Par le centre, trous entiers, **découper** (booléens `i_overlay`) ; fusion des trous qui se chevauchent ; îlots comptés et signalés |
| Éléments de plaque | Onglet Plaque (touche `5`) : trous de fixation (découpés) et zones réservées (disque, rectangle, tracé), garde réglable ; signalés s'ils débordent de la plaque |
| Gabarits | Grilles de ventilateur 40 / 60 / 80 / 92 / 120 / 140 mm (menu Fichier) |
| Mesures | Taux d'ouverture exact (chevauchements, découpe, îlots) |
| Exports | SVG, DXF R12, projet, **PNG** 300 dpi (fond transparent, 8 192 px au plus) |
| Fichiers | Format de projet v3 ; v1 et v2 se lisent tels quels |
| Tests | `cargo test --workspace` : 97 tests verts ; clippy : 0 avertissement ; `npm run check` : 0 erreur ; `npm run build` |
| Vérifié | Dans le navigateur : gabarit 120 mm, mode Découper (78 trous coupés à la zone), trou de fixation ajouté puis agrandi à Ø 20, import d'un SVG à arcs et chanfrein avec un trou intérieur, avertissement de débordement, PNG 150 × 100 mm → 1 772 × 1 182 px |
| Non vérifié | Import de DXF réels (LightBurn, Inkscape, FreeCAD) ; exports relus dans LightBurn, FreeCAD, Fusion 360 ; découpe d'une pièce |

## Points ouverts

- Fibonacci : l'espacement ne varie qu'avec la distance au centre.
- Motifs où tout se chevauche : la mesure des paires et l'union dominent le calcul.
- Contour libre : la distance au bord parcourt toutes les arêtes (lent au-delà de quelques milliers de sommets).
- Importer un contour garde les éléments de plaque existants (ceux d'un gabarit peuvent alors déborder : signalé).

## Prochaine action

Lancer la v0.4 (export STEP).
