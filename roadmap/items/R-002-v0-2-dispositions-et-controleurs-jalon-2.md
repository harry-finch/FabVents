---
id: R-002
titre: "v0.2 — Dispositions et contrôleurs (jalon 2)"
resume: "Six nouvelles dispositions, contrôleurs courbe et image, canal Espacement, options de remplissage et de disparition, outil Règle."
statut: livree
jalon: v0.2
cree: 2026-10-23
maj: 2026-11-12
livree: 2026-10-23
---

- Dispositions : **Aléatoire** (Poisson-disc à densité variable, graine), **Spirale**, **Concentrique**, **Fibonacci** (phyllotaxie), **Hachures croisées**, **Chemin** (répétition le long d'un tracé).
- Orientation selon le chemin (concentrique, spirale, chemin).
- Canal **Espacement** (densité locale des dispositions aléatoire, Fibonacci, chemin).
- Contrôleur **Courbe / polyligne** ouverte ou fermée, congés aux nœuds, conversion en géométrie exacte (cercle, ellipse, polygone régulier), saisie aux cotes.
- Contrôleur **Image** (halftone) sur n'importe quel canal : inverser, contraste.
- Options : côté unique et extension infinie, remplir l'intérieur d'une courbe fermée, disparition aléatoire (*cull*) avec graine, angle symétrique et par paliers, unifier portée / valeur d'un canal.
- Outil **Règle** ; redimensionnement proportionnel de tout le motif quand la plaque change de taille.

Acceptation : chaque disposition et chaque option a un test moteur (déterminisme compris : même graine, même résultat).
