use std::fs;
use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::domain::{BinaryType, GameMetadata, GameTarget, LaunchOptions, Runner, RunnerBackend};
use crate::error::{KalesaError, Result};
use crate::generators::{config, desktop, icon, launcher};
use crate::pipeline::{detect, metadata};

#[derive(Debug, Clone, Default)]
pub struct SetupOptions {
    pub name: Option<String>,
    pub developer: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub icon: Option<PathBuf>,
    pub runner: RunnerBackend,
    pub wine_prefix: Option<PathBuf>,
    pub proton_path: Option<PathBuf>,
    pub launch: LaunchOptions,
    pub force: bool,
}

pub fn run(target_path: &Path, custom_name: Option<String>, force: bool) -> Result<()> {
    let options = SetupOptions {
        name: custom_name,
        force,
        ..SetupOptions::default()
    };
    run_with_options(target_path, options)
}

pub fn run_with_options(target_path: &Path, options: SetupOptions) -> Result<()> {
    validate_target(target_path)?;
    options.launch.validate()?;

    let canonical_target =
        fs::canonicalize(target_path).map_err(|e| KalesaError::io("canonicalizing target", e))?;
    let target_dir = canonical_target
        .parent()
        .ok_or_else(|| KalesaError::InvalidDesktopValue("target has no parent directory".into()))?;

    let binary_type = detect::detect(&canonical_target)?;
    let target = GameTarget::new(canonical_target.clone(), binary_type);
    let mut game_metadata = metadata::collect(&target, options.name.as_deref());
    apply_overrides(&mut game_metadata, &options);
    let runner = Runner::for_target_with_backend(
        &target,
        target_dir,
        options.runner,
        options.wine_prefix.clone(),
        options.proton_path.clone(),
    );
    validate_runner(&target, &runner)?;

    info!("Detected target binary type: {}", binary_type.as_str());
    info!("Selected runner backend: {}", runner.kind.as_str());

    let workdir = WorkDir::new(target_dir);
    workdir.create()?;

    let icon_path = materialize_icon(&target, &game_metadata, options.icon.is_some(), &workdir)?;
    game_metadata.icon_path = icon_path;

    let config_path = workdir.config.join("config.yaml");
    config::write(
        &config_path,
        &target,
        &game_metadata,
        &runner,
        &options.launch,
    )?;

    let config_example_path = workdir.config.join("config.yaml.example");
    config::write_example(&config_example_path)?;

    let launch_path = workdir.bin.join("launch.sh");
    launcher::write(
        &launch_path,
        &config_path,
        &target,
        &game_metadata,
        &runner,
        &options.launch,
    )?;

    desktop::write_with_metadata(&game_metadata, target_dir, target_dir, options.force)?;

    info!("Setup completed successfully for {}", game_metadata.name);
    Ok(())
}

fn apply_overrides(metadata: &mut GameMetadata, options: &SetupOptions) {
    if let Some(value) = &options.developer {
        metadata.developer = Some(value.clone());
    }
    if let Some(value) = &options.version {
        metadata.version = Some(value.clone());
    }
    if let Some(value) = &options.description {
        metadata.description = Some(value.clone());
    }
    if !options.categories.is_empty() {
        metadata.categories = options.categories.clone();
    }
    if let Some(path) = &options.icon {
        metadata.icon_path = Some(path.clone());
    }
}

fn validate_runner(target: &GameTarget, runner: &Runner) -> Result<()> {
    if matches!(
        runner.kind,
        crate::domain::RunnerKind::Wine | crate::domain::RunnerKind::Proton
    ) && !target.binary_type.is_windows()
    {
        return Err(KalesaError::InvalidDesktopValue(
            "Wine/Proton runners can only be selected for Windows PE targets".into(),
        ));
    }
    if runner.is_proton() && runner.proton_path.is_none() {
        return Err(KalesaError::MissingProtonPath);
    }
    if runner.is_wine() && runner.wine_prefix.is_none() {
        return Err(KalesaError::MissingWinePrefix);
    }
    Ok(())
}

fn materialize_icon(
    target: &GameTarget,
    game_metadata: &GameMetadata,
    explicit_icon: bool,
    workdir: &WorkDir,
) -> Result<Option<PathBuf>> {
    if target.binary_type == BinaryType::WindowsPe && !explicit_icon {
        let destination = workdir.icons.join("game_icon.png");
        let contents =
            fs::read(&target.path).map_err(|e| KalesaError::io("reading PE icon resources", e))?;
        if icon::extract_pe_icon(&contents, &destination) {
            info!("Extracted icon from PE resources to {:?}", destination);
            return Ok(Some(destination));
        }
        warn!("Could not extract an icon from PE resources");
        return Ok(None);
    }

    let Some(found) = game_metadata.icon_path.as_deref() else {
        warn!(
            "No usable icon found for {:?}; using theme fallback",
            target.path
        );
        return Ok(None);
    };

    if !found.is_file() {
        warn!(
            "Configured icon {:?} does not exist; using theme fallback",
            found
        );
        return Ok(None);
    }

    let ext = found.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let destination = workdir.icons.join(format!("game_icon.{ext}"));
    fs::copy(found, &destination).map_err(|e| KalesaError::io("copying game icon", e))?;
    info!("Copied icon {:?} to {:?}", found, destination);
    Ok(Some(destination))
}

fn validate_target(target_path: &Path) -> Result<()> {
    if !target_path.exists() {
        return Err(KalesaError::TargetNotFound(target_path.to_path_buf()));
    }
    if !target_path.is_file() {
        return Err(KalesaError::TargetNotFile(target_path.to_path_buf()));
    }
    Ok(())
}

struct WorkDir {
    root: PathBuf,
    config: PathBuf,
    bin: PathBuf,
    icons: PathBuf,
}

impl WorkDir {
    fn new(base_dir: &Path) -> Self {
        let root = base_dir.join(".workdir");
        Self {
            config: root.join("config"),
            bin: root.join("bin"),
            icons: root.join("icons"),
            root,
        }
    }

    fn create(&self) -> Result<()> {
        for dir in [&self.root, &self.config, &self.bin, &self.icons] {
            fs::create_dir_all(dir).map_err(|e| KalesaError::io("creating workdir", e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workdir_is_created_beside_target() {
        let target_dir = Path::new("/games/ChildofLight");
        let workdir = WorkDir::new(target_dir);

        assert_eq!(workdir.root, PathBuf::from("/games/ChildofLight/.workdir"));
        assert_eq!(
            workdir.config,
            PathBuf::from("/games/ChildofLight/.workdir/config")
        );
        assert_eq!(
            workdir.bin,
            PathBuf::from("/games/ChildofLight/.workdir/bin")
        );
        assert_eq!(
            workdir.icons,
            PathBuf::from("/games/ChildofLight/.workdir/icons")
        );
    }
}
