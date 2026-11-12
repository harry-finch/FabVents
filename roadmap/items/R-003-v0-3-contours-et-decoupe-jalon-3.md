---
id: R-003
titre: "v0.3 — Contours et découpe (jalon 3)"
resume: "Contours ellipse, importés ou dessinés, découpe au contour, trous de fixation et zones réservées, gabarits de ventilateurs, export PNG, zone perforée."
statut: livree
jalon: v0.3
cree: 2026-10-23
maj: 2026-11-12
livree: 2026-10-23
---

- Contours : ellipse, contour importé (SVG, DXF) ou dessiné.
- **Découpe au contour** (booléens 2D, `i_overlay`) ; fusion des trous qui se chevauchent ; taux d'ouverture exact.
- *Ajout FabVents* : **trous de fixation et zones réservées** (aucune perforation autour des vis, d'un logo, d'un passage de câble).
- *Ajout FabVents* : **gabarits** de grilles de ventilateur 40 / 60 / 80 / 92 / 120 / 140 mm avec leurs trous de fixation.
- Export **PNG** haute résolution, fond transparent.
- *Ajout* : **zone perforée** (disque ou rectangle) distincte du contour de la plaque.

Acceptation : découpe, fusion et îlots testés sur des cas à aire connue ; contours SVG et DXF (arcs, transformations, unités) relus par le moteur.
