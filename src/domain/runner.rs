use std::path::{Path, PathBuf};

use super::{BinaryType, GameTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    Native,
    Wine,
    Proton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerBackend {
    Auto,
    Native,
    Wine,
    Proton,
}

impl RunnerBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Native => "native",
            Self::Wine => "wine",
            Self::Proton => "proton",
        }
    }
}

impl std::str::FromStr for RunnerBackend {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "native" => Ok(Self::Native),
            "wine" => Ok(Self::Wine),
            "proton" => Ok(Self::Proton),
            _ => Err(format!("unsupported runner backend: {value}")),
        }
    }
}

impl RunnerKind {
    pub fn for_binary(binary_type: BinaryType) -> Self {
        match binary_type {
            BinaryType::LinuxElf | BinaryType::AppImage => Self::Native,
            BinaryType::WindowsPe => Self::Wine,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wine => "wine",
            Self::Proton => "proton",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub kind: RunnerKind,
    pub wine_prefix: Option<PathBuf>,
    pub proton_path: Option<PathBuf>,
}

impl Runner {
    pub fn for_target(target: &GameTarget, project_dir: &Path) -> Self {
        Self::for_target_with_backend(target, project_dir, RunnerBackend::Auto, None, None)
    }

    pub fn for_target_with_backend(
        target: &GameTarget,
        project_dir: &Path,
        backend: RunnerBackend,
        wine_prefix: Option<PathBuf>,
        proton_path: Option<PathBuf>,
    ) -> Self {
        let kind = match backend {
            RunnerBackend::Auto => RunnerKind::for_binary(target.binary_type),
            RunnerBackend::Native => RunnerKind::Native,
            RunnerBackend::Wine => RunnerKind::Wine,
            RunnerBackend::Proton => RunnerKind::Proton,
        };

        let wine_prefix = if matches!(kind, RunnerKind::Wine | RunnerKind::Proton) {
            Some(wine_prefix.unwrap_or_else(|| project_dir.join(".workdir/wine")))
        } else {
            None
        };

        Self {
            kind,
            wine_prefix,
            proton_path,
        }
    }

    pub fn is_wine(&self) -> bool {
        matches!(self.kind, RunnerKind::Wine)
    }

    pub fn is_proton(&self) -> bool {
        matches!(self.kind, RunnerKind::Proton)
    }
}
