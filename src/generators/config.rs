use serde::Serialize;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::{GameTarget, Runner};
use crate::error::{KalesaError, Result};

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct WineConfig {
    prefix: PathBuf,
    arch: String,
}

#[derive(Serialize)]
struct RunnerConfig {
    #[serde(rename = "type")]
    runner_type: String,
    wine: Option<WineConfig>,
}

#[derive(Serialize)]
struct ExecutableConfig {
    path: PathBuf,
}

#[derive(Serialize)]
struct AppConfig {
    schema_version: u32,
    name: String,
    runner: RunnerConfig,
    executable: ExecutableConfig,
}

pub fn write(
    path: &Path,
    target: &GameTarget,
    game_name: &str,
    runner: &Runner,
    _current_dir: &Path,
) -> Result<()> {
    let wine = runner.wine_prefix.as_ref().map(|prefix| WineConfig {
        prefix: prefix.clone(),
        arch: "win64".to_string(),
    });

    let app_config = AppConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        name: game_name.to_string(),
        runner: RunnerConfig {
            runner_type: target.binary_type.as_str().to_string(),
            wine,
        },
        executable: ExecutableConfig {
            path: target.path.clone(),
        },
    };

    let yaml = serde_yaml::to_string(&app_config)?;
    let mut file = File::create(path).map_err(|e| KalesaError::io("creating configuration", e))?;
    file.write_all(yaml.as_bytes())
        .map_err(|e| KalesaError::io("writing configuration", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kalesa_config_test_{}_{}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_versioned_windows_config() {
        let dir = temp_dir("windows");
        let path = dir.join("config.yaml");
        let target_path = dir.join("My Game [1998].exe");
        let target = GameTarget::new(target_path.clone(), crate::domain::BinaryType::WindowsPe);
        let runner = Runner::for_target(&target, &dir);

        write(&path, &target, "My Game", &runner, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("schema_version: 1"));
        assert!(content.contains("name: My Game"));
        assert!(content.contains("type: windows"));
        assert!(content.contains("wine:"));
        assert!(content.contains("My Game [1998].exe"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_linux_config_without_wine() {
        let dir = temp_dir("linux");
        let path = dir.join("config.yaml");
        let target = GameTarget::new(dir.join("game.bin"), crate::domain::BinaryType::LinuxElf);
        let runner = Runner::for_target(&target, &dir);

        write(&path, &target, "My Game", &runner, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("schema_version: 1"));
        assert!(content.contains("type: linux"));
        assert!(!content.contains("prefix:"));
        assert!(!content.contains("arch:"));

        let _ = fs::remove_dir_all(&dir);
    }
}
