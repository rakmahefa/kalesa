use std::path::{Path, PathBuf};

use crate::domain::{
    BinaryType, GameMetadata, GameTarget, LaunchOptions, Runner,
};
use crate::error::{KalesaError, Result};

pub fn write_launch_script(
    path: &Path,
    executable_path: &Path,
    binary_type: BinaryType,
    wine_prefix: Option<&Path>,
) -> Result<()> {
    let config_path = infer_config_path(path).ok_or_else(|| {
        KalesaError::InvalidDesktopValue(
            "cannot infer .workdir/config/config.yaml from launcher path".into(),
        )
    })?;

    let target = GameTarget::new(executable_path.to_path_buf(), binary_type);
    let project_dir = executable_path.parent().ok_or_else(|| {
        KalesaError::InvalidDesktopValue("executable path has no parent directory".into())
    })?;
    let mut runner = Runner::for_target(&target, project_dir);
    if let Some(prefix) = wine_prefix {
        runner.wine_prefix = Some(prefix.to_path_buf());
    }

    let name = executable_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Game")
        .to_string();
    let metadata = GameMetadata::new(name, None);

    crate::generators::launcher::write(
        path,
        &config_path,
        &target,
        &metadata,
        &runner,
        &LaunchOptions::default(),
    )
}

pub fn write_desktop_entries(
    game_name: &str,
    current_dir: &Path,
    icon_path: Option<&Path>,
    output_dir: &Path,
    force: bool,
) -> Result<()> {
    crate::generators::desktop::write(game_name, current_dir, icon_path, output_dir, force)
}

fn infer_config_path(path: &Path) -> Option<PathBuf> {
    let bin_dir = path.parent()?;
    let workdir = bin_dir.parent()?;
    Some(workdir.join("config/config.yaml"))
}
