use crate::domain::{GameMetadata, GameTarget, LaunchOptions, Runner};
use crate::error::Result;

use super::template::TEMPLATE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLauncher {
    pub content: String,
}

pub fn render(
    _target: &GameTarget,
    _metadata: &GameMetadata,
    _runner: &Runner,
    _launch: &LaunchOptions,
    launcher_version: u32,
    config_schema_version: u32,
) -> Result<RenderedLauncher> {
    let content = TEMPLATE
        .replace("__KALESA_LAUNCHER_VERSION__", &launcher_version.to_string())
        .replace("__KALESA_CONFIG_SCHEMA_VERSION__", &config_schema_version.to_string());

    Ok(RenderedLauncher { content })
}
