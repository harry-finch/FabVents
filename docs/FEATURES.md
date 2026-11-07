# Inventaire des fonctions

Périmètre fonctionnel de FabVents, établi à partir de l'inventaire des fonctions de SolidVents (recherche du 22/10/2026, dossier `docs/research/`, hors dépôt) et des besoins des makers. Chaque fonction a un identifiant stable, un jalon ([ROADMAP.md](../ROADMAP.md)) et un état.

États : **fait** · **en cours** · **prévu** · **hors périmètre**. *Ajout* = fonction absente de SolidVents.

## Dispositions (LAY)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| LAY-01 | Grille carrée, hexagonale (60°), quinconce ; pas, angle, décalage | v0.1 | fait |
| LAY-02 | Rotation des formes par rangée / colonne (surface vrillée) | v0.1 | fait |
| LAY-03 | Aléatoire : Poisson-disc à densité variable, graine reproductible | v0.2 | fait |
| LAY-04 | Spirale continue | v0.2 | fait |
| LAY-05 | Anneaux concentriques | v0.2 | fait |
| LAY-06 | Fibonacci (angle d'or 137,5°) ; l'espacement y varie avec la distance au centre | v0.2 | fait |
| LAY-07 | Hachures croisées : treillis de fentes au milieu des arêtes, matière aux croisements | v0.2 | fait |
| LAY-08 | Chemin : répétition le long d'un tracé | v0.2 | fait |
| LAY-09 | Orientation selon le chemin (concentrique, spirale, chemin) | v0.2 | fait |
| LAY-10 | Voronoï : Lloyd, nervures variables, congés aux sommets | v0.5 | fait |
| LAY-11 | Lignes fluides : rubans d'épaisseur et de courbure variables | v0.5 | fait |
| LAY-12 | Disposition scriptée (« IA sur mesure ») avec curseurs déclarés | v0.7 | fait |

## Contrôleurs et canaux (FLD)

Quatre canaux : **Taille**, **Espacement**, **Angle**, **Forme**. Un contrôleur a une géométrie et, pour chaque canal où il agit, une portée et une valeur cible (ADR-0004).

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| FLD-01 | Contrôleur Point (dégradé radial) | v0.1 | fait |
| FLD-02 | Contrôleur Ligne (dégradé le long d'un segment) | v0.1 | fait |
| FLD-03 | Canal Taille | v0.1 | fait |
| FLD-04 | Canal Angle | v0.1 | fait |
| FLD-05 | Bord net (seuil franc au lieu d'un dégradé) | v0.1 | fait |
| FLD-06 | Synchronisation : un même contrôleur sur plusieurs canaux | v0.1 | fait |
| FLD-07 | Canal Espacement (densité) | v0.2 | fait |
| FLD-08 | Courbe / polyligne ouverte ou fermée, congés aux nœuds | v0.2 | fait |
| FLD-09 | Conversion d'un tracé fermé en cercle, ellipse, polygone régulier ; saisie aux cotes (inspecteur) | v0.2 | fait |
| FLD-10 | Image (halftone) sur n'importe quel canal, inverser, contraste | v0.2 | fait |
| FLD-11 | Unifier la portée / la valeur cible d'un canal | v0.2 | fait |
| FLD-12 | Remplir l'intérieur d'une courbe fermée | v0.2 | fait |
| FLD-13 | Côté unique, extension infinie des extrémités | v0.2 | fait |
| FLD-14 | Disparition aléatoire (*cull*) : centre, bord ou uniforme, graine | v0.2 | fait |
| FLD-15 | Angle symétrique, angle par paliers | v0.2 | fait |
| FLD-16 | Canal Forme (morphing) | v0.6 | fait |

## Formes (SHP)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| SHP-01 | Cercle, rectangle à coins arrondis, polygone régulier, étoile, fente | v0.1 | fait |
| SHP-02 | Taille de référence (diamètre, largeur, hauteur) suivie par le canal Taille | v0.1 | fait |
| SHP-03 | Éditeur de formes (atelier) : nœuds, Bézier, primitives cotées, congés non destructifs | v0.6 | fait |
| SHP-04 | Import SVG / DXF d'une forme | v0.6 | fait |
| SHP-05 | Symétrie miroir et radiale (2 à 12 axes), soudure sur l'axe | v0.6 | fait |
| SHP-06 | Calques et rôles booléens (union, soustraction) ; réponse au gradient par calque, pivot | v0.6 | fait |
| SHP-07 | Morphing vers une primitive, morphing personnalisé départ ↔ cible | v0.6 | fait |

## Contour et bords (BND)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| BND-01 | Plaque rectangulaire (coins arrondis) ou ronde, marge | v0.1 | fait |
| BND-02 | Bords : par le centre, trous entiers | v0.1 | fait |
| BND-03 | Bords : découpe au contour (booléens 2D) | v0.3 | fait |
| BND-04 | Ellipse ; contour importé ou dessiné | v0.3 | fait |
| BND-05 | *Ajout* : trous de fixation et zones réservées | v0.3 | fait |
| BND-06 | *Ajout* : gabarits de grilles de ventilateur standard | v0.3 | fait |

## Mesures et fabrication (FAB)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| FAB-01 | Nombre de trous, taux d'ouverture, tailles min / max | v0.1 | fait |
| FAB-02 | *Ajout* : nervure minimale mesurée et seuil d'alerte | v0.1 | fait |
| FAB-03 | *Ajout* : trous trop petits supprimés et comptés (jamais en silence) | v0.1 | fait |
| FAB-04 | Chevauchements détectés et signalés | v0.1 | fait |
| FAB-05 | Taux d'ouverture exact (booléens) avec chevauchements et découpe | v0.3 | fait |
| FAB-06 | Solveur de taux d'ouverture cible (dichotomie) | v0.7 | fait |

## Exports et fichiers (EXP)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| EXP-01 | SVG en mm, couleurs de découpe, projet embarqué (rouvrable) | v0.1 | fait |
| EXP-02 | DXF R12 en mm, calques `CUT_INNER` / `CUT_OUTER`, cercles exacts | v0.1 | fait |
| EXP-03 | Projet `.fabvents.json` ; ouverture d'un projet ou d'un SVG FabVents | v0.1 | fait |
| EXP-04 | Reprise automatique du dernier projet | v0.1 | fait |
| EXP-05 | PNG haute résolution, fond transparent | v0.3 | fait |
| EXP-06 | STEP : plaque B-Rep, cercles analytiques | v0.4 | fait |
| EXP-07 | STEP : découpage en dalles (ZIP ou assemblage) | v0.4 | fait |
| EXP-08 | STEP : angle de dépouille avec validation | v0.4 | fait |
| EXP-09 | DXF : arcs exacts (*bulge*), simplification RDP (découpes et congés, tous les exports) | v0.4 | fait |
| EXP-10 | Préréglages : bibliothèque locale, adaptation aux proportions | v0.7 | fait |
| EXP-11 | Wrap : projection sur une face 3D courbe | — | hors périmètre |

## Canevas et ergonomie (UX)

| Id | Fonction | Jalon | État |
| --- | --- | --- | --- |
| UX-01 | Zoom molette centré sur le curseur, déplacement, cadrage | v0.1 | fait |
| UX-02 | Poignées déplaçables, création de points / lignes, suppression | v0.1 | fait |
| UX-03 | Annuler / rétablir | v0.1 | fait |
| UX-04 | Champs numériques : flèches (Maj × 10), glisser sur le libellé (Maj × 10, Alt ÷ 10) | v0.1 | fait |
| UX-05 | Calcul dans un Web Worker, l'interface ne se fige jamais | v0.1 | fait |
| UX-06 | FR / EN, mm / pouces, thème sombre / clair / système | v0.1 | fait |
| UX-07 | Outil Règle | v0.2 | fait |
| UX-08 | Redimensionnement proportionnel du motif | v0.2 | fait |
| UX-09 | Calcul différé au relâchement pour les très grands motifs | v0.2 | fait |
| UX-10 | Application de bureau (Tauri) | v0.7 | fait |
| UX-11 | CLI et serveur MCP | v0.7 | fait |

## Pistes techniques de la recherche : retenues et écartées

| Piste | Décision | Raison |
| --- | --- | --- |
| Interface `egui` | Écartée | Même écosystème que FabSpeed : Svelte 5 et jetons partagés (ADR-0001). |
| Moteur dans le processus de l'interface | Retenue, via WebAssembly + Web Worker | Pas de serveur, poignées fluides, version web statique possible (ADR-0002). |
| `rayon` | Écartée pour l'instant | Pas de threads en WebAssembly sans configuration spéciale ; mono-thread suffisant à ce jour. |
| Invalidation par étages (germes → instances → mesures) | Retenue | Cache par égalité des sous-configurations dans `Pipeline`, sans drapeaux à gérer côté interface. |
| Atténuation *smoothstep* | Retenue | Continue, sans palier visible. |
| Mélange `Max` / `Add` / `Multiply` des contrôleurs | Remplacée | Mélange pondéré vers la valeur cible (ADR-0004) : pas de dépassement, valeur de base loin des contrôleurs. |
| PRNG à graine | Retenue (SplitMix64) | Motifs reproductibles ; tirage par position pour la disparition aléatoire (une perforation garde son tirage). |
| Bridson à rayon variable | Retenue | Densité pilotée par le canal Espacement ; 25 000 germes en ~65 ms (natif). |
| `i_overlay` | Retenue (v0.3) | Booléens 2D en Rust pur, compilable en WebAssembly. |
| DXF AC1015 minimal | Remplacée par DXF R12 | Un AC1015 sans tables complètes ni identifiants est refusé par AutoCAD ; le R12 est lu partout (ADR-0003). |
| STEP `EXTRUDED_AREA_SOLID` | Écartée | Entité de géométrie constructive peu lue par les importeurs AP214 ; B-Rep explicite à la place (v0.4). |
| STEP via `truck` | Écartée (v0.4) | Générateur B-Rep maison (ADR-0008) : arcs, cônes et dalles exacts sans noyau NURBS, fichiers validés dans FreeCAD. |
| Wrap (OpenCascade, LSCM / ARAP) | Hors périmètre | Coût sans commune mesure avec le reste. |
| `rhai` pour les dispositions scriptées | Retenue (v0.7) | Bac à sable sans horloge ni modules, opérations bornées ; +1,5 Mo de WebAssembly (ADR-0011). |
| Aperçu PNG pour MCP (`tiny-skia`) | Retenue (v0.7) | Rendu sans fenêtre, dans la CLI seulement (le moteur reste léger). |
