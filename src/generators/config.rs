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
