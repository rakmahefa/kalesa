use std::path::Path;

use crate::domain::{BinaryType, GameTarget, LaunchOptions, Runner, RunnerKind};
use crate::error::Result;

pub fn write_launch_script(
    path: &Path,
    executable_path: &Path,
    binary_type: BinaryType,
    wine_prefix: Option<&Path>,
) -> Result<()> {
    let target = GameTarget::new(executable_path.to_path_buf(), binary_type);
    let runner = Runner {
        kind: RunnerKind::for_binary(binary_type),
        wine_prefix: wine_prefix.map(Path::to_path_buf),
        proton_path: None,
    };
    crate::generators::launcher::write(path, &target, &runner, &LaunchOptions::default())
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
