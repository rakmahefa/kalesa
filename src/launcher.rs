use std::path::{Path, PathBuf};

use crate::domain::{BinaryType, LaunchOptions, Runner, RunnerKind};
use crate::error::{KalesaError, Result};

pub fn write_launch_script(
    path: &Path,
    executable_path: &Path,
    binary_type: BinaryType,
    wine_prefix: Option<&Path>,
) -> Result<()> {
    let _target = executable_path;
    let _runner = Runner {
        kind: RunnerKind::for_binary(binary_type),
        wine_prefix: wine_prefix.map(Path::to_path_buf),
        proton_path: None,
    };
    let _launch = LaunchOptions::default();

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
