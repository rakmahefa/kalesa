use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Serialize)]
struct WineConfig {
    prefix: String,
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
    path: String,
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
    exe_filename: &str,
    exe_type: &str,
    is_windows: bool,
    current_dir_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let wine_cfg = if is_windows {
        Some(WineConfig {
            prefix: format!("{}/.workdir/wine", current_dir_str),
            arch: "win64".to_string(),
        })
    } else {
        None
    };

    let app_config = AppConfig {
        name: game_name.to_string(),
        runner: RunnerConfig {
            runner_type: exe_type.to_string(),
            wine: wine_cfg,
        },
        executable: ExecutableConfig {
            path: exe_filename.to_string(),
        },
    };

    let yaml_str = serde_yaml::to_string(&app_config)?;
    let mut file = File::create(path)?;
    file.write_all(yaml_str.as_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> std::path::PathBuf {
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

        write_config(&path, "My Game", "game.exe", "windows", true, "/tmp/mygame").unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("name: My Game"));
        assert!(content.contains("type: windows"));
        assert!(content.contains("wine:"));
        assert!(content.contains("path: game.exe"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_linux_config_without_wine_section() {
        let dir = temp_dir("linux");
        let path = dir.join("config.yaml");

        write_config(&path, "My Game", "game.bin", "linux", false, "/tmp/mygame").unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("type: linux"));
        // serde_yaml still emits the `wine:` key for `Option::None` (as
        // `wine: null`), so we check that no actual Wine settings leaked in.
        assert!(!content.contains("prefix:"));
        assert!(!content.contains("arch:"));

        let _ = fs::remove_dir_all(&dir);
    }
}
