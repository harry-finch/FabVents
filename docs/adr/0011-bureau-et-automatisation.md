# ADR-0011 : Bureau et automatisation

Status: Accepted
Date: 2026-11-07

## Context

La v0.7 sort FabVents du navigateur : application de bureau, ligne de commande et serveur MCP pour les agents, préréglages, solveur de taux d'ouverture et dispositions scriptées (« IA sur mesure »). Le moteur reste une bibliothèque pure, compilée en WebAssembly pour l'interface.

## Decision

- **Solveur de taux d'ouverture** dans le moteur (`solve`) : dichotomie sur la taille de base (taille de la forme, nervure de Voronoï, largeur des rubans) ou sur l'espacement, d'abord pour trouver la valeur la plus ouvrante qui respecte la nervure mini (ni chevauchement ni nervure fine), puis pour le taux visé. Taux hors d'atteinte : meilleur réglage rendu avec `reached: false`, jamais de nervure sacrifiée. Tolérance ±0,25 point (le nombre de perforations varie par paliers avec l'espacement).
- **Préréglages** dans l'interface (stockage du poste, fichier `fabvents.presets` à importer / exporter) : un préréglage est un projet complet ; appliqué, il est mis à l'échelle de la plaque en cours (même logique que le redimensionnement avec « Mettre le motif à l'échelle ») et la plaque garde son contour, ses dimensions et son épaisseur.
- **CLI et MCP** dans une crate à part (`fabvents-cli`, binaire `fabvents`), sans framework : arguments lus à la main, JSON-RPC 2.0 ligne à ligne sur stdio, rien d'autre que les réponses sur la sortie standard. Six outils : guide du format, nouveau projet, calcul, solveur, aperçu PNG (`tiny-skia`, base64), export dans un fichier (chemin absolu). Descriptions en français, comme l'interface.
- **Dispositions scriptées** en Rhai dans le moteur (`script`) : le script place les germes (`point`) et déclare ses curseurs (`slider`), que l'interface affiche ; bac à sable sans horloge, sans modules ni `eval`, opérations, profondeur et tailles bornées ; `rand()` tiré de la graine du projet. Une erreur devient le message `layout.script_error` avec ligne, colonne et texte (nouveau champ `detail` des messages).
- **Application de bureau** Tauri 2 (`fabvents-desktop`) : l'interface et le moteur WebAssembly tels quels ; commandes Rust pour les fenêtres natives (greffon `dialog`), l'enregistrement sur place et les fichiers récents ; l'interface les appelle par `window.__TAURI__` (`withGlobalTauri`), sans dépendance npm de plus. Dans le navigateur, téléchargement et sélecteur de fichier comme avant.

## Alternatives considered

- Solveur côté interface (appels répétés au moteur par le worker) : plus lent, pas disponible pour la CLI et l'agent.
- `clap` et un SDK MCP : dépendances lourdes pour six commandes et trois méthodes JSON-RPC.
- Moteur derrière un serveur local dans l'application de bureau (comme FabSpeed) : inutile, FabVents n'a pas de base de données ; le WebAssembly suffit.
- Scripts en JavaScript (exécutés dans la page) : pas de bac à sable sûr, pas de CLI ni de MCP.

## Consequences

Le moteur WebAssembly passe de 1,3 à 2,8 Mo (880 Ko compressé) avec Rhai. La CLI Tauri 2.12 demandant Rust 1.90, la 2.11.5 est épinglée tant que le projet reste en Rust 1.88. Les installeurs ne sont pas signés (v1.0).

## Validation

Tests moteur (solveur : taille, espacement, cible hors d'atteinte, nervure de Voronoï ; script : curseurs, erreurs situées, boucle infinie arrêtée, aléatoire reproductible), tests du serveur MCP (poignée de main, outils, aperçu, chemin relatif refusé) et session réelle sur stdio ; parcours dans le navigateur (solveur, préréglages adaptés, script et curseurs).
