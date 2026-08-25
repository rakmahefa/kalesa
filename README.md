# kalesa

Outil en ligne de commande qui prépare un dossier de jeu pour un lancement
standardisé : validation du type de binaire (Windows PE / Linux ELF),
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
  Linux). Le fichier doit être un PE ou un ELF valide.
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

Le type de binaire est validé à partir de son en-tête :

- `\x7fELF` → Linux, avec validation minimale de l'en-tête ELF (classe,
  endianess et version).
- `MZ` → le champ DOS `e_lfanew` est lu puis la signature `PE\0\0` et le
  début de l'en-tête COFF sont obligatoirement présents à cet offset.
- Un fichier inconnu ou un en-tête ELF/PE tronqué ou invalide est rejeté avec
  une erreur typée ; Kalesa ne traite plus un `MZ` invalide comme un exécutable
  Windows et ne transforme plus un format inconnu en cible Linux.

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
  sous `.workdir/wine`) et exécute le chemin absolu de la cible via `wine` ;
  les chemins sont shell-quotés pour résister aux espaces, apostrophes et
  autres métacaractères ; si `wine` n'est pas trouvé dans le `PATH`, le script
  échoue immédiatement avec un message explicite ;
- pour une cible Linux, rend le binaire exécutable si besoin puis exécute son
  chemin absolu avec les mêmes garanties d'escaping.

Les entrées `.desktop` utilisent également un escaping dédié pour le champ
`Exec=` et les valeurs textuelles. Les noms ou chemins contenant un retour à
la ligne sont rejetés car ils ne peuvent pas être représentés sans ambiguïté
dans ce format.

## Tests

```bash
cargo test
```

Couvre notamment : validation ELF/PE stricte, rejet d'un `MZ` sans vraie
signature PE, rejet des formats inconnus, extraction d'icône sur une entrée
invalide, recherche d'icône Linux, génération de la config YAML, génération
sécurisée du script Wine/exécution directe, escaping des caractères spéciaux,
rejet des nouvelles lignes dans les Desktop Entries et non-écrasement des
fichiers `.desktop` sans `--force`.

## Limites connues / pistes futures

- Aucune extraction d'icône n'est tentée depuis un AppImage (nécessiterait
  de monter/décompresser l'image squashfs).
- Le préfixe Wine n'est pas initialisé automatiquement (`wineboot`) au
  premier lancement ; c'est wine qui s'en charge à la volée au premier
  `wine <jeu>`, mais un `wineboot --init` explicite avant le premier
  lancement pourrait être ajouté pour un contrôle plus fin.
