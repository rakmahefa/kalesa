# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement standardisé : validation du type de binaire (Windows PE / Linux ELF / AppImage), collecte de métadonnées, découverte d'icône, génération d'une config YAML, d'un script de lancement et d'entrées `.desktop` (freedesktop).

## Prérequis

- **Rust >= 1.85** (le projet utilise `edition = "2024"`). Le fichier `rust-toolchain.toml` fixe ce numéro.
- **`wine`** dans le `PATH` pour utiliser le backend Wine.
- Un exécutable Proton peut être fourni avec `--proton-path` pour le backend Proton.

## Usage

```bash
kalesa <target> [options]
```

### Cible

`target` est un exécutable PE Windows, un binaire ELF Linux ou une AppImage (`*.AppImage`). Kalesa valide l'en-tête ELF/PE avant de générer les artefacts.

### Métadonnées

```text
--name NAME              Nom affiché
--developer NAME         Développeur/studio
--version VERSION        Version du jeu
--description TEXT       Description courte
--category CATEGORY      Catégorie freedesktop, répétable
--icon PATH              Icône explicite
```

Pour les cibles Linux/AppImage, Kalesa inspecte les fichiers `.desktop` voisins, puis cherche les icônes dans les chemins XDG (`XDG_DATA_HOME`, `XDG_DATA_DIRS`, thèmes hicolor et pixmaps), avant de revenir aux noms conventionnels voisins.

Les fichiers `.desktop` AppImage suivent notamment les clés `X-AppImage-Name` et `X-AppImage-Version` lorsqu'elles sont présentes. citehttps://docs.appimage.org/reference/desktop-integration.html

### Runner

```text
--runner auto|native|wine|proton
--wine-prefix PATH       Préfixe Wine/Proton
--proton-path PATH       Exécutable Proton
```

`auto` choisit Native pour Linux/AppImage et Wine pour PE. Proton reste explicite afin d'éviter de modifier silencieusement le comportement d'une installation existante.

### Arguments et environnement

```text
--arg VALUE              Argument à intégrer au launcher, répétable
--env KEY=VALUE           Variable d'environnement, répétable
```

Les valeurs sont shell-quotées et les noms de variables sont validés avant génération.

### Génération

Kalesa crée :

```text
.workdir/
  config/config.yaml
  bin/launch.sh
  icons/game_icon.*
game.desktop
.directory
```

Le schéma YAML est versionné (`schema_version: 2`) et contient le runner sélectionné, les métadonnées du jeu et les options de lancement.

## Détection du binaire

- `\x7fELF` → Linux ; validation minimale de l'en-tête ELF.
- un ELF dont le chemin se termine par `.AppImage` → AppImage ; le fichier reste exécuté directement par le runtime AppImage.
- `MZ` → PE ; `e_lfanew` puis `PE\0\0` sont obligatoirement présents.
- tout format inconnu ou tronqué est rejeté.

Le type 2 d'AppImage est conçu comme un exécutable ELF dont le runtime monte ensuite son système de fichiers SquashFS et lance `AppRun`. citehttps://docs.appimage.org/reference/architecture.html

## Sécurité des launchers

- chemins et arguments sont shell-quotés ;
- les variables d'environnement utilisent uniquement des noms valides ;
- les entrées `.desktop` rejettent les retours à la ligne ;
- un runner Wine/Proton explicitement demandé sur une cible non Windows est rejeté.

## Tests

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

La CI GitHub exécute ces quatre contrôles.

## Limites connues / pistes futures

- L'extraction de ressources internes d'une AppImage n'est pas exécutée automatiquement : Kalesa privilégie les métadonnées `.desktop`, les icônes XDG et les icônes voisines afin de ne pas exécuter/monter une image potentiellement non fiable.
- Les tests d'intégration du pipeline complet et les fixtures PE/ELF réels restent à ajouter.
