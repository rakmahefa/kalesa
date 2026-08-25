use std::fmt::Write;
use std::path::Path;

use crate::domain::{GameMetadata, GameTarget, LaunchOptions, Runner, RunnerKind};
use crate::error::Result;

use super::escape::{bash_quote, push_array, push_env};
use super::template::TEMPLATE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLauncher {
    pub content: String,
}

pub fn render(
    target: &GameTarget,
    metadata: &GameMetadata,
    runner: &Runner,
    launch: &LaunchOptions,
    launcher_version: u32,
    config_schema_version: u32,
) -> Result<RenderedLauncher> {
    let mut prefix = String::new();
    writeln!(prefix, "# Kalesa launcher format: {launcher_version}")
        .expect("writing to String cannot fail");
    writeln!(prefix, "# Kalesa config schema: {config_schema_version}")
        .expect("writing to String cannot fail");
    writeln!(prefix, "GAME_NAME={}", bash_quote(&metadata.name)?)
        .expect("writing to String cannot fail");
    writeln!(
        prefix,
        "TARGET_VALUE={}",
        bash_quote(&path_text(&target.path)?)?
    )
    .expect("writing to String cannot fail");
    writeln!(
        prefix,
        "RUNNER={}",
        bash_quote(runner.kind.as_str())?
    )
    .expect("writing to String cannot fail");

    if let Some(prefix_path) = &runner.wine_prefix {
        writeln!(
            prefix,
            "WINE_PREFIX_VALUE={}",
            bash_quote(&path_text(prefix_path)?)?
        )
        .expect("writing to String cannot fail");
    } else {
        prefix.push_str("WINE_PREFIX_VALUE=''\n");
    }

    if runner.is_wine() || runner.is_proton() {
        prefix.push_str("WINE_ARCH_VALUE='win64'\n");
    } else {
        prefix.push_str("WINE_ARCH_VALUE=''\n");
    }

    if let Some(proton_path) = &runner.proton_path {
        writeln!(
            prefix,
            "PROTON_PATH_VALUE={}",
            bash_quote(&path_text(proton_path)?)?
        )
        .expect("writing to String cannot fail");
    } else {
        prefix.push_str("PROTON_PATH_VALUE=''\n");
    }

    push_array(&mut prefix, "CONFIG_ARGS", &launch.args)?;
    push_array(&mut prefix, "CONFIG_WRAPPERS", &launch.wrappers)?;
    push_env(&mut prefix, &launch.env)?;

    let content = TEMPLATE
        .replace("__KALESA_RUNTIME_VALUES__", &prefix)
        .replace("__RUNNER_COMMAND__", runner_command(runner.kind));

    Ok(RenderedLauncher { content })
}

fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        crate::error::KalesaError::InvalidDesktopValue("path is not valid UTF-8".into())
    })
}

fn runner_command(kind: RunnerKind) -> &'static str {
    match kind {
        RunnerKind::Native => "native",
        RunnerKind::Wine => "wine",
        RunnerKind::Proton => "proton",
    }
}
