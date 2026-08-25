# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement standardisé : validation du type de binaire (Windows PE / Linux ELF / AppImage), collecte de métadonnées, découverte d’icône, génération d’une configuration YAML versionnée, d’un script de lancement et d’entrées `.desktop` (freedesktop).

## Prérequis

- **Rust >= 1.85** (le projet utilise `edition = "2024"`). Le fichier `rust-toolchain.toml` fixe ce numéro.
- **`yq`** dans le `PATH` : le `launch.sh` relit `config.yaml` à chaque lancement.
- **`wine`** dans le `PATH` lorsque le runner Wine est utilisé.
- Un exécutable Proton peut être fourni avec `--proton-path` lorsque le runner Proton est utilisé.

Le `launch.sh` généré reste stable après la création du package. Les modifications de comportement se font dans `.workdir/config/config.yaml`, qui constitue la source de vérité du runtime.

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
  path: /path/to/ChildOfLight.exe
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

## Launcher runtime

Le launcher actuel est au **format 4** et la configuration reste au **schema v3**.

Kalesa génère le `launch.sh` une seule fois avec le package. Au runtime, `launch.sh` **recharge `config.yaml` avec `yq` à chaque exécution**. Modifier le YAML modifie donc effectivement le comportement du prochain lancement sans régénérer le `.workdir`.

Le launcher runtime :

- vérifie la présence de `.workdir/config/config.yaml` ;
- vérifie que `yq` est disponible ;
- valide `schema_version: 3` avant lecture ;
- lit `name`, `executable.path`, `runner`, Wine/Proton, `launch.args`, `launch.env` et `launch.wrappers` depuis le YAML courant ;
- résout les chemins relatifs par rapport au dossier du jeu ;
- représente les arguments et wrappers comme des tableaux Bash ;
- valide les noms de variables d’environnement avant export ;
- vérifie les dépendances du runner et des wrappers ;
- construit la commande complète sans `eval` et sans `sh -c` ;
- affiche la commande finale avec `printf %q` ;
- conserve les arguments ajoutés directement à `launch.sh` après ceux de la configuration ;
- utilise `exec` afin de préserver le code de retour du jeu.

Le générateur est organisé en modules spécialisés : rendu, template shell et génération du fichier launcher. Le parsing YAML reste une responsabilité du runtime via `yq`, tandis que la validation du package et la production initiale du schema restent des responsabilités Rust.

Le format du launcher et le schéma de configuration sont identifiés dans le script généré par :

```bash
# Kalesa launcher format: 4
# Kalesa config schema: 3
```

### Contrat `config.yaml` / `launch.sh`

```text
kalesa <game>
      │
      ├── .workdir/config/config.yaml  ← source de vérité mutable
      └── .workdir/bin/launch.sh       ← runtime stable
                                             │
                                             └── yq → config.yaml
                                                     │
                                                     └── jeu
```

Le `workdir`, le YAML et le launcher ne sont pas régénérés à chaque lancement. Ils sont produits lors de la préparation du jeu ; ensuite, `config.yaml` peut être édité à la volée.

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
- le YAML est relu avec un parseur YAML dédié au runtime, sans parsing ad hoc en Bash ;
- les arguments sont conservés comme des éléments Bash distincts ;
- les variables d’environnement utilisent uniquement des noms valides ;
- les chemins relatifs sont résolus par rapport au dossier du jeu ;
- les entrées `.desktop` rejettent les retours à la ligne ;
- un runner Wine/Proton explicitement demandé sur une cible non Windows est rejeté ;
- le launcher n’utilise ni `eval` ni `sh -c` pour construire la commande.

## Tests

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

La CI GitHub exécute ces contrôles.

Le générateur de launcher vérifie également que le runtime charge réellement les valeurs depuis `config.yaml` et non depuis les valeurs Rust utilisées lors de la génération.

## Limites connues / pistes futures

- `yq` est actuellement une dépendance runtime du launcher et doit être disponible dans le `PATH` de la machine de jeu.
- L’extraction de ressources internes d’une AppImage n’est pas exécutée automatiquement : Kalesa privilégie les métadonnées `.desktop`, les icônes XDG et les icônes voisines afin de ne pas exécuter/monter une image potentiellement non fiable.
- Les tests d’intégration du pipeline complet et les fixtures PE/ELF réels restent à renforcer.
- La migration automatique de configurations v1/v2 vers v3 n’est pas encore fournie ; la régénération avec Kalesa reste le chemin recommandé.
