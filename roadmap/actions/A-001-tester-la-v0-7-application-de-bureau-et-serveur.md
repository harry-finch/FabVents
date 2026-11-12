---
id: A-001
type: tester
titre: "Tester la v0.7 : application de bureau et serveur MCP"
element: R-007
statut: a-faire
cree: 2026-11-12
par: agent
fait:
---

La v0.7 (bureau et automatisation) est livrée pour essai depuis le 07/11/2026. STATUS.md demande de la tester, « dont l'application de bureau et le serveur MCP dans Claude Code », avant de préparer la v1.0.

Où : l'application de bureau construite est le .dmg de la v0.7 dans dist-desktop/ (hors Git) ; la CLI s'installe par cargo install --path crates/fabvents-cli ; la version web se lance par npm run dev dans ui/.

À essayer, d'après les critères de l'élément R-007 : fenêtres natives d'ouverture et d'enregistrement et fichiers récents ; préréglages (enregistrer, appliquer à une autre plaque, import / export) ; solveur de taux d'ouverture (cible atteinte, ou meilleur réglage signalé) ; disposition Script et ses curseurs ; session MCP complète depuis Claude Code.

Déjà vérifié par l'agent (STATUS.md) : navigateur, CLI et session MCP sur stdio. Non vérifié : l'application de bureau lancée par l'utilisateur, l'usage du serveur MCP dans Claude Code.

À répondre : ce qui marche, ce qui ne marche pas. NB : STATUS.md renvoie à une « liste dans le message de livraison » qui n'est pas dans le dépôt ; si tu l'as, elle prime.
