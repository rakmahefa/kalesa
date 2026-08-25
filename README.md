# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement standardisé : validation du type de binaire (Windows PE / Linux ELF / AppImage), collecte de métadonnées, découverte d’icône, génération d’une configuration YAML versionnée, d’un script de lancement et d’entrées `.desktop` (freedesktop).

## Prérequis

- **Rust >= 1.85** (le projet utilise `edition = "2024"`). Le fichier `rust-toolchain.toml` fixe ce numéro.
- **`yq` v4** dans le `PATH` pour exécuter le `launch.sh` généré. `KALESA_YQ` peut être utilisé pour fournir un chemin personnalisé vers `yq`.
- **`wine`** dans le `PATH` lorsque le runner Wine est utilisé.
- Un exécutable Proton peut être fourni avec `--proton-path` lorsque le runner Proton est utilisé.

## Usage

```bash
kalesa <target> [options]
```

### Cible

`target` est un exécutable PE Windows, un binaire ELF Linux ou une AppImage (`*.AppImage`). Kalesa valide l’en-tête ELF/PE avant de générer les artefacts.

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

Les fichiers `.desktop` AppImage suivent notamment les clés `X-AppImage-Name` et `X-AppImage-Version` lorsqu’elles sont présentes. citehttps://docs.appimage.org/reference/desktop-integration.html

### Runner

```text
--runner auto|native|wine|proton
--wine-prefix PATH       Préfixe Wine/Proton
--proton-path PATH       Exécutable Proton
```

`auto` choisit Native pour Linux/AppImage et Wine pour PE. Proton reste explicite afin d’éviter de modifier silencieusement le comportement d’une installation existante.

### Arguments et environnement

```text
--arg VALUE              Argument à intégrer au launcher, répétable
--env KEY=VALUE          Variable d’environnement, répétable
```

Les valeurs sont shell-quotées et les noms de variables sont validés avant génération.

## Configuration YAML

Kalesa utilise une configuration YAML **versionnée**. Le format actuellement généré est **schema v2**.

Exemple :

```yaml
schema_version: 2
name: ChildofLight
version: null
developer: null
description: null
categories: []
runner:
  type: wine
  wine:
    prefix: /path/to/.workdir/wine
    arch: win64
  proton: null
executable:
  path: /path/to/ChildofLight.exe
launch:
  args: []
  env: []
```

Le schéma v2 ajoute notamment :

- `schema_version` pour identifier explicitement le format de configuration ;
- les métadonnées optionnelles `version`, `developer`, `description` et `categories` ;
- un runner explicite (`native`, `wine` ou `proton`) avec sa configuration ;
- la section `launch` pour les arguments et variables d’environnement.

### Migration v1 → v2

Kalesa a évolué d’un ancien schéma v1 vers le schéma v2. Une configuration v1 peut notamment ressembler à ceci :

```yaml
name: ChildofLight
runner:
  type: windows
  wine:
    prefix: /path/to/.workdir/wine
    arch: win64
executable:
  path: /path/to/ChildofLight.exe
```

Le schéma v2 est celui utilisé par le générateur actuel. Les anciennes façades d’API restent conservées pour la compatibilité avec les points d’entrée historiques de la v0.1.0, mais elles délèguent au générateur v2.

Les fichiers de configuration déjà générés par une ancienne version de Kalesa ne sont pas automatiquement réécrits : relancer Kalesa sur la cible permet de régénérer les artefacts selon le format actuel.

## Launcher runtime v2

Le `launch.sh` généré est un **runtime générique** : il ne contient plus le runner, le chemin de l’exécutable, les arguments ou les variables d’environnement en dur. Il lit ces informations depuis `.workdir/config/config.yaml` au moment du lancement.

Le launcher v2 :

- vérifie la présence du YAML et exige `schema_version: 2` ;
- résout les chemins absolus et les chemins relatifs au dossier du jeu ;
- lit `runner.type`, `executable.path`, `launch.args` et `launch.env` depuis la configuration ;
- prend en charge `native`, `wine` et `proton` ;
- ajoute les arguments passés à `launch.sh` après ceux définis dans la configuration ;
- applique les variables d’environnement du YAML ;
- vérifie les dépendances nécessaires au runner choisi ;
- expose `KALESA_YQ` pour sélectionner un binaire `yq` personnalisé.

Le format du launcher est identifié dans le script généré par :

```bash
# Kalesa launcher format: 2
# Kalesa config schema: 2
```

## Génération

Kalesa crée :

```text
.workdir/
  config/config.yaml
  bin/launch.sh
  icons/game_icon.*
game.desktop
.directory
```

Le fichier `config.yaml` est généré en **schema v2**. Le `launch.sh` est un runtime v2 qui consomme cette configuration au lieu de dupliquer ses valeurs.

## Détection du binaire

- `\x7fELF` → Linux ; validation minimale de l’en-tête ELF.
- un ELF dont le chemin se termine par `.AppImage` → AppImage ; le fichier reste exécuté directement par le runtime AppImage.
- `MZ` → PE ; `e_lfanew` puis `PE\0\0` sont obligatoirement présents.
- tout format inconnu ou tronqué est rejeté.

Le type 2 d’AppImage est conçu comme un exécutable ELF dont le runtime monte ensuite son système de fichiers SquashFS et lance `AppRun`. citehttps://docs.appimage.org/reference/architecture.html

## Sécurité des launchers

- le YAML est validé par Kalesa avant génération ;
- les arguments et variables d’environnement sont shell-quotés lors de leur lecture par le runtime ;
- les variables d’environnement utilisent uniquement des noms valides ;
- les chemins relatifs sont résolus par rapport au dossier du jeu ;
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

- `launch.sh` requiert `yq` v4 au runtime afin de lire le YAML sans embarquer un parseur YAML fragile dans le shell.
- L’extraction de ressources internes d’une AppImage n’est pas exécutée automatiquement : Kalesa privilégie les métadonnées `.desktop`, les icônes XDG et les icônes voisines afin de ne pas exécuter/monter une image potentiellement non fiable.
- Les tests d’intégration du pipeline complet et les fixtures PE/ELF réels restent à ajouter.
- La gestion de migration automatique de configurations YAML v1 existantes n’est pas encore fournie ; la régénération avec Kalesa reste le chemin recommandé.
