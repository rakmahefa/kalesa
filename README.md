# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement
standardisé : détection du type de binaire (Windows PE / Linux ELF),
extraction de l'icône, génération d'une config YAML, d'un script de
lancement et d'entrées `.desktop` (freedesktop).

## Prérequis

- **Rust >= 1.85** (le projet utilise `edition = "2024"`). Le fichier
  `rust-toolchain.toml` fixe ce numéro : avec `rustup` installé, `cargo
  build` récupérera automatiquement le bon compilateur.
- **`wine`** installé et dans le `PATH` si vous comptez lancer des jeux
  Windows (`launch.sh` généré appelle `wine` pour les cibles PE).

## Usage

```bash
kalesa <target> [--name <nom>] [--force] [--verbose]
```

- `target` : chemin vers l'exécutable du jeu (`.exe` Windows ou binaire
  Linux).
- `--name` / `-n` : nom affiché (par défaut, dérivé du nom de fichier).
- `--force` / `-f` : écrase `game.desktop` et `.directory` s'ils existent
  déjà. Sans ce flag, un fichier déjà présent est laissé intact (avertissement
  loggé) pour ne pas écraser une personnalisation manuelle.
- `--verbose` / `-v` : passe les logs en niveau `debug`. Le niveau peut aussi
  être contrôlé via la variable d'environnement `RUST_LOG`.

À l'exécution, l'outil crée :

```
.workdir/
  config/config.yaml   # runner (native/wine), chemin de l'exécutable
  bin/launch.sh         # script de lancement exécutable
  icons/game_icon.*      # icône extraite (PNG pour Windows, format d'origine pour Linux)
game.desktop            # entrée freedesktop
.directory
```

## Détection du binaire

Le type de binaire est déterminé en lisant les premiers octets du fichier :

- `\x7fELF` → Linux.
- `MZ` → on va plus loin : on lit `e_lfanew` dans l'en-tête DOS puis on
  vérifie la présence de la signature `PE\0\0` à cet offset, pour distinguer
  un vrai PE d'un simple exécutable DOS qui commence aussi par `MZ`. Si la
  signature ne peut pas être confirmée, le fichier est quand même traité
  comme "windows" (comportement conservé), mais un avertissement est loggé.

## Extraction d'icône

- **Windows (PE)** : l'icône est reconstruite en un fichier `.ico` complet et
  valide via `pelite`, puis décodée avec le décodeur ICO du crate `image`
  (qui gère correctement les entrées BMP "headerless" ainsi que les entrées
  PNG, et choisit automatiquement la plus grande résolution disponible), puis
  sauvegardée en PNG.
- **Linux (ELF)** : les ressources PE ne s'appliquent pas aux binaires ELF.
  L'outil cherche, à titre "best effort", un fichier d'icône au nom
  conventionnel à côté de l'exécutable (`icon.png`, `icon.svg`, `icon.xpm`,
  `<nom_du_binaire>.png`, `<nom_du_binaire>.svg`) et le copie s'il le trouve.
- Si aucune icône n'a pu être obtenue, l'icône de thème générique
  `applications-games` est utilisée en repli.

## Lancement du jeu

`launch.sh` :

- pour une cible Windows, exporte `WINEPREFIX`/`WINEARCH` (préfixe dédié
  sous `.workdir/wine`) et exécute `wine <jeu>` ; si `wine` n'est pas
  trouvé dans le `PATH`, le script échoue immédiatement avec un message
  explicite plutôt que de ne rien faire ;
- pour une cible Linux, rend le binaire exécutable si besoin puis l'exécute
  directement.

## Tests

```bash
cargo test
```

Couvre notamment : détection ELF/PE (y compris le cas "MZ sans vraie
signature PE"), échec propre de l'extraction d'icône sur une entrée
invalide, recherche d'icône Linux, génération de la config YAML
(Windows avec section `wine`, Linux sans), génération du script de
lancement (variante Wine / variante exécution directe), et non-écrasement
des fichiers `.desktop` sans `--force`.

## Limites connues / pistes futures

- Aucune extraction d'icône n'est tentée depuis un AppImage (nécessiterait
  de monter/décompresser l'image squashfs).
- Le préfixe Wine n'est pas initialisé automatiquement (`wineboot`) au
  premier lancement ; c'est wine qui s'en charge à la volée au premier
  `wine <jeu>`, mais un `wineboot --init` explicite avant le premier
  lancement pourrait être ajouté pour un contrôle plus fin.
