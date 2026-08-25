# Kalesa — Roadmap de robustesse et évolution

## Phase 1 — Robustesse

- [x] Introduire un `BinaryType` fort et supprimer les couples `(&str, bool)`.
- [x] Utiliser `PathBuf` / `Path` pour les chemins internes et éviter les assemblages de chemins par concaténation de chaînes.
- [x] Valider strictement les exécutables ELF et PE et refuser les formats inconnus ou invalides.
- [x] Sécuriser la génération de `launch.sh` avec un escaping shell correct.
- [x] Sécuriser la génération des fichiers `.desktop` conformément au format Desktop Entry.
- [x] Introduire des erreurs typées et préserver les causes d'I/O, de parsing et de génération.
- [x] Ajouter des tests de régression couvrant les chemins et noms de fichiers contenant des caractères spéciaux.

## Phase 2 — Qualité

- [x] `cargo fmt --check`
- [x] `cargo clippy -- -D warnings`
- [x] GitHub Actions pour format, clippy, tests et build.
- [ ] Tests d'intégration du pipeline complet de génération.
- [ ] Fixtures PE/ELF réels pour les tests déterministes.

## Phase 3 — Architecture

- [x] Introduire un domaine explicite pour la cible de jeu et le runner.
- [x] Isoler le pipeline de setup des générateurs de fichiers.
- [x] Versionner le modèle de configuration YAML.
- [x] Découpler détection, collecte de métadonnées et génération.

### Structure implémentée

```text
src/
├── domain/
│   ├── binary.rs
│   ├── game.rs
│   ├── runner.rs
│   └── mod.rs
├── pipeline/
│   ├── detect.rs
│   ├── metadata.rs
│   ├── setup.rs
│   └── mod.rs
├── generators/
│   ├── config.rs
│   ├── desktop.rs
│   ├── icon.rs
│   ├── launcher.rs
│   └── mod.rs
└── compatibility facades
    ├── config.rs
    ├── icon.rs
    └── launcher.rs
```

Les façades historiques conservent les points d'entrée publics de la v0.1.0 tout en déléguant vers les nouveaux modules.

## Phase 4 — Fonctionnalités

- [x] Support AppImage.
- [x] Améliorer la découverte d'icônes Linux/XDG et des métadonnées `.desktop`.
- [x] Arguments de lancement (`--arg`).
- [x] Variables d'environnement (`--env KEY=VALUE`).
- [x] Backends Wine/Proton configurables (`--runner`, `--wine-prefix`, `--proton-path`).
- [x] Métadonnées du jeu (`--developer`, `--version`, `--description`, `--category`, `--icon`).

### Limite AppImage

Kalesa reconnaît et lance directement les AppImages de type Linux via leur en-tête ELF et l'extension `.AppImage`. L'extraction automatique de ressources internes de l'AppImage n'est pas effectuée afin de ne pas exécuter ou monter une image potentiellement non fiable ; les fichiers `.desktop`, icônes XDG et icônes voisines sont utilisés lorsqu'ils sont disponibles.

## Ordre d'exécution

La Phase 1 et la Phase 2 doivent rester stables avant d'étendre les fonctionnalités de la Phase 4. La Phase 3 sert de fondation aux extensions futures et conserve la compatibilité comportementale obtenue pendant les phases précédentes.
