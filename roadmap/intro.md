**But de la v1.0** : une application de bureau (et une version web sans installation) qui reprend les fonctions utiles de SolidVents pour les makers et les fablabs : motifs de perforation paramétriques pilotés par des contrôleurs posés sur le canevas, contrôles de fabrication, exports laser / CNC (SVG, DXF) et CAO (STEP). Sans compte ni abonnement : les fonctions « Pro » de SolidVents sont incluses.

**Principes** (détail dans [AGENTS.md](AGENTS.md)) :

- moteur Rust pur et déterministe (`fabvents-core`), testé, sans IO ni interface ;
- même écosystème que FabSpeed : Svelte 5, jetons *Professional Workshop UI*, français et anglais, thème sombre de référence et thème clair, Tauri pour le bureau ;
- les valeurs saisies ne sont jamais corrigées en silence : le moteur signale, l'interface montre ;
- millimètres en interne, pouces à l'affichage ;
- un jalon = une version testable, livrée avec sa liste de vérifications.

La source fonctionnelle est l'inventaire de [docs/FEATURES.md](docs/FEATURES.md) (chaque fonction y a son identifiant et son jalon).
