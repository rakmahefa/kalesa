use std::path::{Path, PathBuf};

use crate::domain::BinaryType;
use crate::error::{KalesaError, Result};

pub fn write_launch_script(
    path: &Path,
    _executable_path: &Path,
    _binary_type: BinaryType,
    _wine_prefix: Option<&Path>,
) -> Result<()> {
    let config_path = infer_config_path(path).ok_or_else(|| {
        KalesaError::InvalidDesktopValue(
            "cannot infer .workdir/config/config.yaml from launcher path".into(),
        )
    })?;
    crate::generators::launcher::write(path, &config_path)
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
