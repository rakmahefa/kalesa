use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::{GameMetadata, GameTarget, LaunchOptions, Runner};
use crate::error::{KalesaError, Result};

pub const CONFIG_SCHEMA_VERSION: u32 = 3;

#[derive(Serialize)]
struct WineConfig {
    prefix: PathBuf,
    arch: String,
}

#[derive(Serialize)]
struct ProtonConfig {
    path: PathBuf,
}

#[derive(Serialize)]
struct RunnerConfig {
    #[serde(rename = "type")]
    runner_type: String,
    wine: Option<WineConfig>,
    proton: Option<ProtonConfig>,
}

#[derive(Serialize)]
struct AppConfig {
    schema_version: u32,
    name: String,
    version: Option<String>,
    developer: Option<String>,
    description: Option<String>,
    categories: Vec<String>,
    runner: RunnerConfig,
    executable: ExecutableConfig,
    launch: LaunchConfig,
}

#[derive(Serialize)]
struct ExecutableConfig {
    path: PathBuf,
}

#[derive(Serialize)]
struct LaunchConfig {
    args: Vec<String>,
    env: std::collections::BTreeMap<String, String>,
    wrappers: Vec<String>,
}

pub fn write(
    path: &Path,
    target: &GameTarget,
    metadata: &GameMetadata,
    runner: &Runner,
    launch: &LaunchOptions,
) -> Result<()> {
    launch.validate()?;

    let wine = runner.wine_prefix.as_ref().map(|prefix| WineConfig {
        prefix: prefix.clone(),
        arch: "win64".to_string(),
    });
    let proton = runner
        .proton_path
        .as_ref()
        .map(|path| ProtonConfig { path: path.clone() });

    let app_config = AppConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        developer: metadata.developer.clone(),
        description: metadata.description.clone(),
        categories: metadata.categories.clone(),
        runner: RunnerConfig {
            runner_type: runner.kind.as_str().to_string(),
            wine,
            proton,
        },
        executable: ExecutableConfig {
            path: target.path.clone(),
        },
        launch: LaunchConfig {
            args: launch.args.clone(),
            env: launch.env.clone(),
            wrappers: launch.wrappers.clone(),
        },
    };

    let yaml = serde_yaml::to_string(&app_config)?;
    let mut file = File::create(path).map_err(|e| KalesaError::io("creating configuration", e))?;
    file.write_all(yaml.as_bytes())
        .map_err(|e| KalesaError::io("writing configuration", e))?;
    Ok(())
}

pub fn write_example(path: &Path) -> Result<()> {
    let mut file = File::create(path).map_err(|e| KalesaError::io("creating configuration example", e))?;
    file.write_all(CONFIG_EXAMPLE.as_bytes())
        .map_err(|e| KalesaError::io("writing configuration example", e))?;
    Ok(())
}

const CONFIG_EXAMPLE: &str = r#"# Kalesa configuration schema v3.
# This file is an editable example only. Copy its structure to config.yaml.
#
# Paths may be absolute or relative to the game directory. Relative paths make
# a generated setup easier to move between directories or machines.

# Schema version understood by Kalesa launcher v3.
schema_version: 3

# Desktop metadata displayed by launchers and menus.
name: MyGame
version: null
developer: null
description: null
categories: []

# Runtime backend.
# Allowed values: native, wine, proton.
runner:
  type: wine

  # Used by Wine and Proton. Prefer a relative path such as
  # .workdir/wine when the prefix should live beside the game.
  wine:
    prefix: /home/user/Games/pfx/MyGame
    arch: win64

  # Required only when runner.type is proton.
  proton: null
  # Example:
  # proton:
  #   path: /home/user/.steam/steam/compatibilitytools.d/Proton/Proton

# Executable to run.
# Prefer a path relative to the game directory for portability.
executable:
  path: MyGame.exe

launch:
  # Arguments are passed as separate argv entries, in this exact order.
  args: []
  # Example:
  # args:
  #   - -fullscreen
  #   - --language=fr

  # Environment variables exported before the runner starts.
  env: {}
  # Example:
  # env:
  #   WINEDEBUG: -all
  #   DXVK_LOG_LEVEL: none

  # Optional wrappers executed from left to right before the runner.
  wrappers: []
  # Example:
  # wrappers:
  #   - gamemoderun
  #   - mangohud
"#;
