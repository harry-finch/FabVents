# ADR-0003 : Formats de fichiers

Status: Accepted
Date: 2026-10-23

## Context

Les makers envoient les motifs vers des logiciels de découpe laser (LightBurn, RDWorks, Inkscape), de FAO CNC et de CAO, et veulent pouvoir reprendre un motif plus tard. La recherche proposait un DXF AC1015 minimal et un SVG avec le projet embarqué.

## Decision

- **Projet** : JSON (`.fabvents.json`), `format = "fabvents.project"`, `version` entière. Champ absent → valeur par défaut ; version plus récente → refus explicite (`project.too_new`) ; évolution incompatible → migration et test de l'ancien format.
- **SVG** : `width` / `height` en mm, `viewBox` en mm, Y retourné ; traits de 0,1 mm sans remplissage, contour rouge `#ff0000` (`cut-outer`), perforations bleues `#0000ff` (`cut-inner`) ; cercles en `<circle>`, arcs en commandes `A`. Le projet complet est dans `<metadata id="fabvents-project">` (CDATA, `]]>` échappé) : un SVG exporté se rouvre dans FabVents.
- **DXF R12 (AC1009)** : `$INSUNITS = 4` (mm), calques `CUT_INNER` (couleur 5) et `CUT_OUTER` (couleur 1), `CIRCLE` pour les trous ronds, `POLYLINE` fermées avec arcs exacts (*bulge*, code 42) pour les autres contours.

## Alternatives considered

- DXF AC1015 (2000) minimal : AutoCAD exige alors les tables complètes, les identifiants et la section OBJECTS ; un fichier incomplet est refusé. Le R12 est lu partout et suffit (polylignes à arcs comprises).
- Projet uniquement dans le SVG : un fichier JSON reste utile pour les scripts et le futur serveur MCP.

## Consequences

Les polylignes R12 sont plus verbeuses que les `LWPOLYLINE` (un `VERTEX` par sommet). Les couleurs du SVG sont des conventions de découpe, pas un rendu : un aperçu rempli viendra avec l'export PNG (v0.3).

## Validation

Tests : aller-retour projet ↔ SVG, structure DXF (unités, calques, nombre d'entités et d'arcs). Audit ezdxf sans erreur ; rendus Inkscape (SVG) et ezdxf (DXF) identiques. Reste à valider : import LightBurn, FreeCAD, Fusion 360.
