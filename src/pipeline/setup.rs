use std::fs;
use std::path::{Path, PathBuf};

use log::{info, warn};

use crate::domain::{BinaryType, GameTarget, Runner};
use crate::error::{KalesaError, Result};
use crate::generators::{config, desktop, icon, launcher};
use crate::pipeline::{detect, metadata};

pub fn run(target_path: &Path, custom_name: Option<String>, force: bool) -> Result<()> {
    validate_target(target_path)?;
    let canonical_target = fs::canonicalize(target_path)
        .map_err(|e| KalesaError::io("canonicalizing target", e))?;
    let current_dir =
        std::env::current_dir().map_err(|e| KalesaError::io("reading current directory", e))?;

    let binary_type = detect::detect(&canonical_target)?;
    let target = GameTarget::new(canonical_target.clone(), binary_type);
    let metadata = metadata::collect(&target, custom_name.as_deref());
    let runner = Runner::for_target(&target, &current_dir);
    info!("Detected target binary type: {}", binary_type.as_str());

    let workdir = WorkDir::new();
    workdir.create()?;

    let icon_path = match binary_type {
        BinaryType::WindowsPe => {
            let output = workdir.icons.join("game_icon.png");
            let contents = fs::read(&canonical_target)
                .map_err(|e| KalesaError::io("reading PE icon resources", e))?;
            if icon::extract_pe_icon(&contents, &output) {
                info!("Extracted icon from PE resources to {:?}", output);
                Some(output)
            } else {
                warn!("Could not extract an icon from the PE resources of {:?}", canonical_target);
                None
            }
        }
        BinaryType::LinuxElf => match metadata.icon_path {
            Some(found) => {
                let ext = found.extension().and_then(|e| e.to_str()).unwrap_or("png");
                let destination = workdir.icons.join(format!("game_icon.{ext}"));
                if fs::copy(&found, &destination).is_ok() {
                    info!("Copied sibling icon {:?} to {:?}", found, destination);
                    Some(destination)
                } else {
                    warn!("Could not copy sibling icon {:?}", found);
                    None
                }
            }
            None => {
                warn!("No sibling icon found next to {:?}", canonical_target);
                None
            }
        },
    };

    let config_path = workdir.config.join("config.yaml");
    config::write(&config_path, &target, &metadata.name, &runner, &current_dir)?;

    let launch_path = workdir.bin.join("launch.sh");
    launcher::write(&launch_path, &target, &runner)?;

    desktop::write(
        &metadata.name,
        &current_dir,
        icon_path.as_deref(),
        &current_dir,
        force,
    )?;

    info!("Architecture setup completed successfully for {}", metadata.name);
    Ok(())
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
    fn new() -> Self {
        let root = PathBuf::from(".workdir");
        Self {
            config: root.join("config"),
            bin: root.join("bin"),
            icons: root.join("icons"),
            root,
        }
    }

    fn create(&self) -> Result<()> {
        for dir in [&self.root, &self.config, &self.bin, &self.icons] {
            fs::create_dir_all(dir)
                .map_err(|e| KalesaError::io("creating workdir", e))?;
        }
        Ok(())
    }
}
