use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::config::{AppConfig, CONFIG_SCHEMA_VERSION};
use crate::domain::BinaryType;
use crate::error::{KalesaError, Result};
use crate::pipeline::detect;

#[derive(Debug, Clone)]
struct RuntimeContext {
    config_path: PathBuf,
    game_dir: PathBuf,
    config: AppConfig,
    target: PathBuf,
}

pub fn run(config_path: &Path, extra_args: &[String]) -> Result<()> {
    let context = load(config_path)?;
    let command = build_command(&context, extra_args)?;

    let mut process = Command::new(&command[0]);
    process.args(&command[1..]);
    process.current_dir(&context.game_dir);
    process.envs(&context.config.launch.env);
    apply_runner_environment(&mut process, &context)?;

    let rendered = render_command(&command);
    println!("[+] Launching {}: {}", context.config.name, rendered);

    let status = process.status().map_err(|source| KalesaError::CommandIo {
        command: rendered.clone(),
        source,
    })?;

    exit_status_result(status, rendered)
}

fn load(config_path: &Path) -> Result<RuntimeContext> {
    let config_path = fs::canonicalize(config_path)
        .map_err(|e| KalesaError::io("canonicalizing runtime configuration", e))?;
    let yaml = fs::read_to_string(&config_path)
        .map_err(|e| KalesaError::io("reading runtime configuration", e))?;
    let config: AppConfig = serde_yaml::from_str(&yaml)?;

    validate_config(&config)?;

    let config_dir = config_path
        .parent()
        .ok_or_else(|| KalesaError::InvalidRuntimeConfig("configuration has no parent directory".into()))?;
    let workdir = config_dir
        .parent()
        .ok_or_else(|| KalesaError::InvalidRuntimeConfig("configuration is not inside .workdir/config".into()))?;
    let game_dir = workdir
        .parent()
        .ok_or_else(|| KalesaError::InvalidRuntimeConfig("configuration is not inside a game directory".into()))?;

    let target = resolve_path(game_dir, &config.executable.path);
    if !target.is_file() {
        return Err(KalesaError::TargetNotFound(target));
    }

    let binary_type = detect::detect(&target)?;
    validate_runner_target(&config.runner.runner_type, binary_type)?;

    Ok(RuntimeContext {
        config_path,
        game_dir: game_dir.to_path_buf(),
        config,
        target,
    })
}

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(KalesaError::InvalidRuntimeConfig(format!(
            "unsupported schema_version {}; expected {}",
            config.schema_version, CONFIG_SCHEMA_VERSION
        )));
    }

    if config.name.trim().is_empty() {
        return Err(KalesaError::InvalidRuntimeConfig("name cannot be empty".into()));
    }

    if config.executable.path.as_os_str().is_empty() {
        return Err(KalesaError::InvalidRuntimeConfig(
            "executable.path cannot be empty".into(),
        ));
    }

    let env_options = crate::LaunchOptions {
        args: config.launch.args.clone(),
        env: config.launch.env.clone(),
        wrappers: config.launch.wrappers.clone(),
    };
    env_options.validate()?;

    for wrapper in &config.launch.wrappers {
        if wrapper.trim().is_empty() {
            return Err(KalesaError::InvalidRuntimeConfig(
                "launch wrapper cannot be empty".into(),
            ));
        }
    }

    match config.runner.runner_type.as_str() {
        "native" => {}
        "wine" | "proton" => {
            let wine = config.runner.wine.as_ref().ok_or_else(|| {
                KalesaError::InvalidRuntimeConfig(
                    "runner.wine is required for wine/proton".into(),
                )
            })?;
            if wine.prefix.as_os_str().is_empty() {
                return Err(KalesaError::MissingWinePrefix);
            }
            if wine.arch.trim().is_empty() {
                return Err(KalesaError::InvalidRuntimeConfig(
                    "runner.wine.arch cannot be empty".into(),
                ));
            }
        }
        other => {
            return Err(KalesaError::InvalidRuntimeConfig(format!(
                "unsupported runner.type '{other}'"
            )));
        }
    }

    if config.runner.runner_type == "proton" {
        let proton = config.runner.proton.as_ref().ok_or_else(|| {
            KalesaError::MissingProtonPath
        })?;
        if proton.path.as_os_str().is_empty() {
            return Err(KalesaError::MissingProtonPath);
        }
    }

    Ok(())
}

fn validate_runner_target(runner: &str, binary_type: BinaryType) -> Result<()> {
    if matches!(runner, "wine" | "proton") && !binary_type.is_windows() {
        return Err(KalesaError::InvalidRuntimeConfig(format!(
            "runner '{runner}' requires a Windows PE executable; detected {}",
            binary_type.as_str()
        )));
    }
    Ok(())
}

fn build_command(context: &RuntimeContext, extra_args: &[String]) -> Result<Vec<String>> {
    let mut command = Vec::with_capacity(
        context.config.launch.wrappers.len() + context.config.launch.args.len() + 4 + extra_args.len(),
    );

    for wrapper in &context.config.launch.wrappers {
        let resolved = resolve_program(&context.game_dir, wrapper).ok_or_else(|| {
            KalesaError::InvalidRuntimeConfig(format!("launcher wrapper not found: {wrapper}"))
        })?;
        command.push(resolved.to_string_lossy().into_owned());
    }

    match context.config.runner.runner_type.as_str() {
        "native" => command.push(context.target.to_string_lossy().into_owned()),
        "wine" => {
            command.push(find_on_path("wine").ok_or_else(|| {
                KalesaError::InvalidRuntimeConfig("wine was not found in PATH".into())
            })?);
            command.push(context.target.to_string_lossy().into_owned());
        }
        "proton" => {
            let proton = context.config.runner.proton.as_ref().expect("validated proton config");
            let proton_path = resolve_path(&context.game_dir, &proton.path);
            if !proton_path.is_file() {
                return Err(KalesaError::InvalidRuntimeConfig(format!(
                    "Proton executable not found: {}",
                    proton_path.display()
                )));
            }
            command.push(proton_path.to_string_lossy().into_owned());
            command.push("run".into());
            command.push(context.target.to_string_lossy().into_owned());
        }
        other => {
            return Err(KalesaError::InvalidRuntimeConfig(format!(
                "unsupported runner.type '{other}'"
            )));
        }
    }

    command.extend(context.config.launch.args.iter().cloned());
    command.extend(extra_args.iter().cloned());
    Ok(command)
}

fn apply_runner_environment(process: &mut Command, context: &RuntimeContext) -> Result<()> {
    if matches!(context.config.runner.runner_type.as_str(), "wine" | "proton") {
        let wine = context.config.runner.wine.as_ref().expect("validated wine config");
        process.env("WINEPREFIX", resolve_path(&context.game_dir, &wine.prefix));
        process.env("WINEARCH", &wine.arch);
    }
    Ok(())
}

fn resolve_program(game_dir: &Path, value: &str) -> Option<PathBuf> {
    if value.contains('/') {
        let path = resolve_path(game_dir, Path::new(value));
        return path.is_file().then_some(path);
    }
    find_on_path(value).map(PathBuf::from)
}

fn find_on_path(program: &str) -> Option<String> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() && is_executable(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn resolve_path(game_dir: &Path, value: &Path) -> PathBuf {
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        game_dir.join(value)
    }
}

fn render_command(command: &[String]) -> String {
    command
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "_./:-".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn exit_status_result(status: ExitStatus, command: String) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(KalesaError::CommandFailed {
            command,
            code: status.code(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutableConfig, LaunchConfig, RunnerConfig, WineConfig};

    fn sample_config() -> AppConfig {
        AppConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            name: "ChildofLight".into(),
            version: None,
            developer: None,
            description: None,
            categories: Vec::new(),
            runner: RunnerConfig {
                runner_type: "wine".into(),
                wine: Some(WineConfig {
                    prefix: PathBuf::from("/home/neko/Games/pfx/ChildofLight"),
                    arch: "win64".into(),
                }),
                proton: None,
            },
            executable: ExecutableConfig {
                path: PathBuf::from("ChildofLight.exe"),
            },
            launch: LaunchConfig {
                args: vec!["-fullscreen".into(), "--language=fr".into()],
                env: BTreeMap::from([("DXVK_FRAME_RATE".into(), "30".into())]),
                wrappers: vec!["gamemoderun".into(), "mangohud".into()],
            },
        }
    }

    #[test]
    fn validates_runtime_schema() {
        validate_config(&sample_config()).unwrap();
    }

    #[test]
    fn rejects_unsupported_schema() {
        let mut config = sample_config();
        config.schema_version = 2;
        assert!(matches!(
            validate_config(&config),
            Err(KalesaError::InvalidRuntimeConfig(_))
        ));
    }

    #[test]
    fn builds_wrapped_wine_command_with_configured_args() {
        let config = sample_config();
        let context = RuntimeContext {
            config_path: PathBuf::from("/games/Child/.workdir/config/config.yaml"),
            game_dir: PathBuf::from("/games/Child"),
            target: PathBuf::from("/games/Child/ChildofLight.exe"),
            config,
        };

        let command = build_command(&context, &["--debug".into()]).unwrap();
        assert_eq!(command[0], "gamemoderun");
        assert_eq!(command[1], "mangohud");
        assert_eq!(command[2], "wine");
        assert_eq!(command.last().unwrap(), "--debug");
        assert!(command.contains(&"30".into()) == false);
    }
}
