---
id: R-004
titre: "v0.4 — Export STEP (jalon 4)"
resume: "Plaque extrudée en B-Rep explicite, découpage en dalles, angle de dépouille, arcs exacts dans le DXF, fiches de validation."
statut: livree
jalon: v0.4
cree: 2026-11-06
maj: 2026-11-12
livree: 2026-11-06
---

- Plaque extrudée en **B-Rep explicite** (`MANIFOLD_SOLID_BREP`) : trous ronds en vraies faces cylindriques, polygones en faces planes. L'`EXTRUDED_AREA_SOLID` suggéré dans la recherche est écarté (mal lu par les importeurs AP214, voir `docs/FEATURES.md`).
- **Découpage en dalles** (STP Split) N × M : archive ZIP ou assemblage STEP unique. Les coupes passent entre les perforations quand c'est possible, sinon à travers (encoches exactes).
- **Angle de dépouille** : cônes analytiques, validation avant export (trou qui se referme, paroi percée) surlignée sur le canevas.
- DXF : arcs exacts (*bulge*) pour les congés ; simplification Ramer-Douglas-Peucker. *Fait au-delà* : arcs retrouvés sur toutes les découpes (mode Découper, fusions), pour tous les exports.
- Fiches de validation : import FreeCAD ([docs/validation/step-freecad.md](docs/validation/step-freecad.md)) ; Fusion 360 et PrusaSlicer à faire par l'utilisateur.

Acceptation : chaque cas de la fiche FreeCAD donne des solides valides, au volume calculé.
