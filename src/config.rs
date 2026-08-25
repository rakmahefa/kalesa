pub use crate::generators::config::CONFIG_SCHEMA_VERSION;

use std::path::Path;

use crate::domain::{GameMetadata, GameTarget, LaunchOptions, Runner};
use crate::error::Result;

pub fn write_config(
    path: &Path,
    game_name: &str,
    executable_path: &Path,
    binary_type: crate::domain::BinaryType,
    current_dir: &Path,
) -> Result<()> {
    let target = GameTarget::new(executable_path.to_path_buf(), binary_type);
    let runner = Runner::for_target(&target, current_dir);
    let metadata = GameMetadata::new(game_name.to_string(), None);
    crate::generators::config::write(path, &target, &metadata, &runner, &LaunchOptions::default())
}
