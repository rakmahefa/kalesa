mod render;
mod template;

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::domain::{GameMetadata, GameTarget, LaunchOptions, Runner};
use crate::error::{KalesaError, Result};

pub use render::RenderedLauncher;

pub const LAUNCHER_FORMAT_VERSION: u32 = 4;
pub const CONFIG_SCHEMA_VERSION: u32 = 3;

pub fn write(
    path: &Path,
    _config_path: &Path,
    target: &GameTarget,
    metadata: &GameMetadata,
    runner: &Runner,
    launch: &LaunchOptions,
) -> Result<()> {
    launch.validate()?;
    validate_inputs(target, metadata, runner)?;

    let rendered = render::render(
        target,
        metadata,
        runner,
        launch,
        LAUNCHER_FORMAT_VERSION,
        CONFIG_SCHEMA_VERSION,
    )?;

    let mut file = File::create(path).map_err(|e| KalesaError::io("creating launch script", e))?;
    file.write_all(rendered.content.as_bytes())
        .map_err(|e| KalesaError::io("writing launch script", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|e| KalesaError::io("setting launch script permissions", e))?;
    }

    Ok(())
}

fn validate_inputs(target: &GameTarget, metadata: &GameMetadata, runner: &Runner) -> Result<()> {
    if metadata.name.trim().is_empty() {
        return Err(KalesaError::InvalidDesktopValue(
            "game name cannot be empty".into(),
        ));
    }

    if target.path.as_os_str().is_empty() {
        return Err(KalesaError::InvalidDesktopValue(
            "target executable path cannot be empty".into(),
        ));
    }

    if (runner.is_wine() || runner.is_proton()) && runner.wine_prefix.is_none() {
        return Err(KalesaError::MissingWinePrefix);
    }

    if runner.is_proton() && runner.proton_path.is_none() {
        return Err(KalesaError::MissingProtonPath);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{BinaryType, RunnerBackend};
    use std::path::PathBuf;

    #[test]
    fn renders_runtime_that_reads_mutable_yaml() {
        let target = GameTarget::new(PathBuf::from("Child of Light.exe"), BinaryType::WindowsPe);
        let runner = Runner::for_target_with_backend(
            &target,
            Path::new("/Games/Child of Light"),
            RunnerBackend::Wine,
            None,
            None,
        );
        let launch = LaunchOptions::default();

        let rendered = render::render(
            &target,
            &GameMetadata::new("Child of Light".into(), None),
            &runner,
            &launch,
            LAUNCHER_FORMAT_VERSION,
            CONFIG_SCHEMA_VERSION,
        )
        .unwrap();

        assert!(rendered.content.contains("Kalesa launcher format: 4"));
        assert!(rendered.content.contains("Kalesa config schema: 3"));
        assert!(rendered.content.contains("require_command yq"));
        assert!(rendered
            .content
            .contains("yq -r '.executable.path // \"\"' \"$CONFIG_FILE\""));
        assert!(rendered
            .content
            .contains("yq -r '.runner.type // \"\"' \"$CONFIG_FILE\""));
        assert!(rendered.content.contains(".launch.args[]?"));
        assert!(rendered.content.contains(".launch.wrappers[]?"));
        assert!(rendered.content.contains(".launch.env // {}"));
        assert!(!rendered.content.contains("Child of Light.exe\"\nRUNNER='wine'"));
        assert!(!rendered.content.contains("CONFIG_ARGS=( '--language=fr'"));
    }
}
