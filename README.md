# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement standardisé : validation du type de binaire (Windows PE / Linux ELF / AppImage), collecte de métadonnées, découverte d’icône, génération d’une configuration YAML versionnée, d’un script de lancement et d’entrées `.desktop` (freedesktop).

## Prérequis

- **Rust >= 1.85** (le projet utilise `edition = "2024"`). Le fichier `rust-toolchain.toml` fixe ce numéro.
- **`wine`** dans le `PATH` lorsque le runner Wine est utilisé.
- Un exécutable Proton peut être fourni avec `--proton-path` lorsque le runner Proton est utilisé.

Le `launch.sh` généré n’a plus de dépendance à `yq` : les valeurs du runtime sont matérialisées directement depuis le même modèle Rust que le YAML au moment de la génération.

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

### Runner

```text
--runner auto|native|wine|proton
--wine-prefix PATH       Préfixe Wine/Proton
--proton-path PATH       Exécutable Proton
```

`auto` choisit Native pour Linux/AppImage et Wine pour PE. Proton reste explicite afin d’éviter de modifier silencieusement le comportement d’une installation existante.

### Arguments, environnement et wrappers

```text
--arg VALUE              Argument à intégrer au launcher, répétable
--env KEY=VALUE          Variable d’environnement, répétable
--wrapper COMMAND        Wrapper à placer devant le runner, répétable
```

Exemple :

```bash
kalesa ChildOfLight.exe \
  --runner wine \
  --wrapper gamemoderun \
  --wrapper mangohud \
  --arg -fullscreen \
  --arg "--language=fr" \
  --env WINEDEBUG=-all
```

Les arguments restent des éléments distincts, les variables d’environnement deviennent une map YAML déterministe et les wrappers sont conservés comme une liste ordonnée.

## Configuration YAML

Kalesa utilise une configuration YAML **versionnée**. Le format actuellement généré est **schema v3**.

Exemple :

```yaml
schema_version: 3
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
  args:
    - -fullscreen
    - --language=fr
  env:
    WINEDEBUG: -all
    DXVK_LOG_LEVEL: none
  wrappers:
    - gamemoderun
    - mangohud
```

Le schéma v3 conserve `args` comme une liste YAML native et transforme `env` en mapping `KEY: VALUE`. `wrappers` est une liste ordonnée appliquée avant le runner sélectionné.

### Migration v1 → v2 → v3

Kalesa a évolué d’un ancien schéma v1 vers v2 puis v3. Une configuration existante n’est pas réécrite automatiquement : relancer Kalesa sur la cible reste le chemin recommandé pour régénérer les artefacts selon le format actuel.

## Launcher runtime v3

Le `launch.sh` généré est un runtime Bash strict. Il ne parse pas le YAML au lancement et ne dépend donc pas de `yq`. Les valeurs sont générées depuis le même modèle Rust que `config.yaml` afin d’éviter une divergence de parsing entre générateur et runtime.

Le launcher v3 :

- vérifie la présence du fichier de configuration et utilise explicitement le schema v3 ;
- résout les chemins absolus et relatifs au dossier du jeu ;
- représente les arguments et wrappers comme des tableaux Bash ;
- applique les variables d’environnement sous forme de valeurs shell-quotées ;
- vérifie les dépendances du runner et des wrappers ;
- construit la commande complète sans `eval` et sans `sh -c` ;
- affiche la commande finale avec `echo` avant l’exécution ;
- conserve les arguments ajoutés directement à `launch.sh` après ceux de la configuration ;
- utilise `exec` afin de préserver le code de retour du jeu.

Le format du launcher est identifié dans le script généré par :

```bash
# Kalesa launcher format: 3
# Kalesa config schema: 3
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

## Détection du binaire

- `\x7fELF` → Linux ; validation minimale de l’en-tête ELF.
- un ELF dont le chemin se termine par `.AppImage` → AppImage ; le fichier reste exécuté directement par le runtime AppImage.
- `MZ` → PE ; `e_lfanew` puis `PE\0\0` sont obligatoirement présents.
- tout format inconnu ou tronqué est rejeté.

## Sécurité des launchers

- le YAML est validé par Kalesa avant génération ;
- les arguments et variables d’environnement sont shell-quotés lors de la génération du runtime ;
- les variables d’environnement utilisent uniquement des noms valides ;
- les chemins relatifs sont résolus par rapport au dossier du jeu ;
- les entrées `.desktop` rejettent les retours à la ligne ;
- un runner Wine/Proton explicitement demandé sur une cible non Windows est rejeté ;
- le launcher n’utilise ni `eval` ni `sh -c` pour construire la commande.

## Tests

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

La CI GitHub exécute ces quatre contrôles.

## Limites connues / pistes futures

- Le launcher v3 embarque les valeurs issues du YAML au moment de la génération ; modifier manuellement `config.yaml` sans régénérer `launch.sh` ne modifie donc pas le runtime embarqué.
- L’extraction de ressources internes d’une AppImage n’est pas exécutée automatiquement : Kalesa privilégie les métadonnées `.desktop`, les icônes XDG et les icônes voisines afin de ne pas exécuter/monter une image potentiellement non fiable.
- Les tests d’intégration du pipeline complet et les fixtures PE/ELF réels restent à renforcer.
- La migration automatique de configurations v1/v2 vers v3 n’est pas encore fournie ; la régénération avec Kalesa reste le chemin recommandé.
