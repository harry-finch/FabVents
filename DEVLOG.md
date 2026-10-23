# Journal de développement

## 2026-10-23 — v0.1 : tranche verticale

**Entrées.** Conversation de recherche sur SolidVents (`docs/research/2026-10-22-conversation-gemini-solidvents.md`, hors dépôt) : inventaire des fonctions et pistes techniques. FabSpeed comme référence d'ergonomie et d'architecture (skills `professional-workshop-ui`, `project-documentation`).

**Choix.**
- Interface Svelte 5 au lieu de l'`egui` proposé par la recherche, pour être dans le même écosystème que FabSpeed (ADR-0001).
- Moteur Rust compilé en WebAssembly et exécuté dans un Web Worker : pas de serveur, l'interface ne se fige jamais, version web statique possible (ADR-0002).
- Trous décrits par une âme (point, segment, polygone) et un arrondi : aires, nervures et contours exacts, cercles et arcs conservés dans les exports (ADR-0005).
- Mélange pondéré des contrôleurs vers leur valeur cible plutôt que les modes `Max / Add / Multiply` de la recherche (ADR-0004).
- DXF R12 plutôt que l'AC1015 minimal de la recherche, refusé par AutoCAD sans tables complètes (ADR-0003). `EXTRUDED_AREA_SOLID` écarté pour le STEP à venir (peu lu par les importeurs AP214).
- Valeurs hors domaine signalées par code (`value.must_be_positive`, `layout.too_many_holes`…), jamais bridées en silence ; trous trop petits retirés et comptés.

**Vérification.**
- 46 tests (moteur, pont WebAssembly). DXF de l'exemple et d'une plaque ronde à fentes audités par ezdxf (R12, `$INSUNITS = 4`, calques, 0 erreur) ; SVG rendus par Inkscape (120 mm → 453,5 px, échelle juste) et DXF rendus par ezdxf : orientation et arcs identiques.
- Dans le navigateur (onglet de développement) : déplacement d'un point de taille, ligne d'angle, passage en fente, annuler / rétablir, thème clair, anglais, exports SVG / DXF / projet (contenus inspectés), relecture d'un SVG exporté, plaque ronde. Grande grille 600 × 400 mm au pas de 3 mm : 29 121 trous calculés en 17 ms, rendu sous 8 ms par image.

**Défauts corrigés pendant la vérification.**
- Vue cadrée sur la taille initiale du panneau : le canevas recadre tant que l'utilisateur n'a ni zoomé ni déplacé la vue.
- Sélecteur de motif tronqué à 1 100 px : passé sur toute la largeur du panneau.
- Avertissement Svelte `assignment_value_stale` au changement de forme.
- Taux d'ouverture affiché à 238 % sur une grille qui se chevauche : valeur conservée mais signalée (couleur d'avertissement, infobulle).

**Non fait.** Import dans LightBurn, FreeCAD et Fusion 360 ; découpe réelle.
