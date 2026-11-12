---
id: R-007
titre: "v0.7 — Bureau et automatisation (jalon 7)"
resume: "Application de bureau Tauri, bibliothèque de préréglages, CLI et serveur MCP, solveur de taux d'ouverture, dispositions scriptées."
statut: a-tester
jalon: v0.7
cree: 2026-11-07
maj: 2026-11-12
livree:
---

Livrée pour essai le 07/11/2026, en attente de validation par l'utilisateur. Texte d'origine de l'ancienne feuille de route repris dans « Inclus » et « Critères d'acceptation ».

## Inclus

- Application **Tauri** (macOS, Windows, Linux) : fenêtres natives d'ouverture / d'enregistrement, fichiers récents.
- Bibliothèque locale de préréglages (import / export JSON, adaptation aux proportions de la plaque).
- **CLI** et **serveur MCP** (stdio) : générer, exporter, solveur de taux d'ouverture cible (dichotomie), aperçu PNG pour l'agent.
- Dispositions scriptées (« IA sur mesure ») : script `rhai` en bac à sable, curseurs déclarés par le script.

## Critères d'acceptation

Solveur dans la tolérance ou meilleur réglage signalé, session MCP complète sur stdio, script et curseurs dans l'interface, application de bureau construite et lancée.

## Vérifications de l'utilisateur

- Tester la v0.7, dont l'application de bureau et le serveur MCP dans Claude Code (« Prochaine action » de `STATUS.md`) : voir l'action A-001.
