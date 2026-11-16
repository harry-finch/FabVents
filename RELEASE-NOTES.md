# FabVents 1.0

Générateur de motifs de perforation paramétriques pour makers et fablabs : plaque, disposition, contrôleurs posés sur le canevas, mesures avant découpe, exports SVG, DXF, STEP et PNG. Sans compte ni abonnement.

## Installer

- **macOS (Apple Silicon)** : ouvrir le `.dmg` et glisser FabVents dans Applications. Au premier lancement : clic droit sur l'application, puis Ouvrir (l'application n'est pas signée).
- **Windows** : lancer l'installeur (`-setup.exe` ou `.msi`). Si SmartScreen s'affiche : Informations complémentaires, puis Exécuter quand même (installeur non signé).
- **Linux** : `.AppImage` (le rendre exécutable) ou `.deb`.

## Fonctions

Contours (rectangle arrondi, disque, ellipse, contour importé SVG ou DXF), dispositions (grille, aléatoire, spirale, concentrique, Fibonacci, hachures croisées, chemin, Voronoï, lignes fluides, script), atelier de formes et morphing, éléments de plaque, mesures (taux d'ouverture exact, nervure minimale), solveur de taux d'ouverture, préréglages, ligne de commande `fabvents` et serveur MCP.

## Limites connues

- Installeurs non signés (pas de certificat Apple ni Windows pour cette version).
- STEP non vérifié dans Fusion 360, PrusaSlicer et SolidWorks ; DXF réels importés et exports relus dans LightBurn non vérifiés ; aucune pièce découpée ou imprimée à l'atelier.
- Moteur WebAssembly de 2,8 Mo.
- Autres points ouverts : voir `STATUS.md`.
