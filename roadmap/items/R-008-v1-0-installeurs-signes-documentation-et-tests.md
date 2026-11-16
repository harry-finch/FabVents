---
id: R-008
titre: "v1.0 — Publication : builds et release GitHub"
resume: "Release GitHub v1.0.0 en brouillon : application de bureau pour macOS (Apple Silicon), Windows et Linux, construite par un workflow sur tag, installeurs non signés."
statut: specifiee
jalon: v1.0
cree: 2026-11-12
maj: 2026-11-16
livree:
---

## But

Mettre FabVents à disposition de makers et de fablabs sans qu'ils compilent : une release GitHub v1.0.0 avec l'application de bureau prête à installer sur macOS, Windows et Linux, construite par un workflow reproductible.

## Inclus

- Version `1.0.0` partout où elle est écrite (`Cargo.toml`, `tauri.conf.json`, `ui/package.json`), sans le suffixe `-dev`.
- Workflow GitHub Actions déclenché par un tag `v*` : tests (`cargo test --workspace`, `npm run check`), puis builds Tauri sur trois runners.
- Fichiers attachés à une release GitHub créée en brouillon : `.dmg` macOS Apple Silicon, installeur Windows (`.msi` ou `.exe`), `.AppImage` et `.deb` Linux.
- Notes de release en français : fonctions, limites connues (reprises de `STATUS.md`), consigne « clic droit → Ouvrir » pour macOS.
- Mise à jour de `README.md` (section Application de bureau : télécharger depuis les releases) et de `STATUS.md`.

## Hors périmètre

- Signature et notarisation macOS, signature Windows : pas de certificat pour la v1.0 ; les installeurs sont publiés non signés et la release le dit.
- macOS Intel.
- Documentation utilisateur FR / EN et tests de bout en bout de l'interface : R-013 et R-014.
- Mise à jour automatique de l'application.
- Publication sur un magasin d'applications.

## Dépendances

- Décision prise le 2026-11-16 : la release est construite maintenant et reste en brouillon ; R-009 (pièces d'exemple validées à l'atelier) viendra après.
- A-001 (tester la v0.7, application de bureau et serveur MCP) : le brouillon n'est levé qu'une fois la v0.7 testée, sinon la release publie du non vérifié.

## Critères d'acceptation

- Pousser le tag `v1.0.0` lance le workflow, qui échoue si les tests échouent.
- La release en brouillon contient un installeur par plateforme (macOS ARM, Windows, Linux) dont la version affichée est 1.0.0.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets` et `cd ui && npm run check` sans erreur.
- Les notes de release listent les limites connues et disent que les installeurs ne sont pas signés.
- Rien n'est publié (brouillon levé) sans l'accord de l'utilisateur.

## Vérifications de l'utilisateur

- Installer et ouvrir le `.dmg` de la release sur ce Mac.
- Installer et ouvrir l'installeur Windows sur une machine Windows, si disponible.
- Installer et ouvrir l'AppImage ou le `.deb` sur une machine Linux, si disponible.
- Relire les notes de release et lever le brouillon, après A-001.
