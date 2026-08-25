# Kalesa — Roadmap de robustesse et évolution

## Phase 1 — Robustesse

- [ ] Introduire un `BinaryType` fort et supprimer les couples `(&str, bool)`.
- [ ] Utiliser `PathBuf` / `Path` pour les chemins internes et éviter les assemblages de chemins par concaténation de chaînes.
- [ ] Valider strictement les exécutables ELF et PE et refuser les formats inconnus ou invalides.
- [ ] Sécuriser la génération de `launch.sh` avec un escaping shell correct.
- [ ] Sécuriser la génération des fichiers `.desktop` conformément au format Desktop Entry.
- [ ] Introduire des erreurs typées et préserver les causes d'I/O, de parsing et de génération.
- [ ] Ajouter des tests de régression couvrant les chemins et noms de fichiers contenant des caractères spéciaux.

## Phase 2 — Qualité

- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] GitHub Actions pour format, clippy, tests et build.
- [ ] Tests d'intégration du pipeline complet de génération.
- [ ] Fixtures PE/ELF réels pour les tests déterministes.

## Phase 3 — Architecture

- [ ] Introduire un domaine explicite pour la cible de jeu et le runner.
- [ ] Isoler le pipeline de setup des générateurs de fichiers.
- [ ] Versionner le modèle de configuration YAML.
- [ ] Découpler détection, collecte de métadonnées et génération.

## Phase 4 — Fonctionnalités

- [ ] Support AppImage.
- [ ] Améliorer la découverte d'icônes Linux/XDG.
- [ ] Arguments de lancement.
- [ ] Variables d'environnement.
- [ ] Backends Wine/Proton configurables.
- [ ] Métadonnées du jeu.

## Ordre d'exécution

La Phase 1 doit être terminée et validée avant d'entamer la Phase 2. Les Phases 3 et 4 doivent conserver la compatibilité comportementale obtenue pendant les phases précédentes, sauf changement explicitement documenté.
