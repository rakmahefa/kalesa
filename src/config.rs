use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{KalesaError, Result};
use crate::BinaryType;

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
    name: String,
    runner: RunnerConfig,
    executable: ExecutableConfig,
}

pub fn write_config(
    path: &Path,
    game_name: &str,
    executable_path: &Path,
    binary_type: BinaryType,
    current_dir: &Path,
) -> Result<()> {
    let wine_cfg = binary_type.is_windows().then(|| WineConfig {
        prefix: current_dir.join(".workdir/wine"),
        arch: "win64".to_string(),
    });

    let app_config = AppConfig {
        name: game_name.to_string(),
        runner: RunnerConfig {
            runner_type: binary_type.as_str().to_string(),
            wine: wine_cfg,
        },
        executable: ExecutableConfig {
            path: executable_path.to_path_buf(),
        },
    };

    let yaml_str = serde_yaml::to_string(&app_config)?;
    let mut file = File::create(path).map_err(|e| KalesaError::io("creating configuration", e))?;
    file.write_all(yaml_str.as_bytes())
        .map_err(|e| KalesaError::io("writing configuration", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
    fn writes_windows_config_with_wine_section() {
        let dir = temp_dir("windows");
        let path = dir.join("config.yaml");
        let target = dir.join("My Game [1998].exe");

        write_config(&path, "My Game", &target, BinaryType::WindowsPe, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("name: My Game"));
        assert!(content.contains("type: windows"));
        assert!(content.contains("wine:"));
        assert!(content.contains("My Game [1998].exe"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_linux_config_without_wine_settings() {
        let dir = temp_dir("linux");
        let path = dir.join("config.yaml");
        let target = dir.join("game.bin");

        write_config(&path, "My Game", &target, BinaryType::LinuxElf, &dir).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("type: linux"));
        assert!(content.contains("game.bin"));
        assert!(!content.contains("prefix:"));
        assert!(!content.contains("arch:"));

        let _ = fs::remove_dir_all(&dir);
    }
}
