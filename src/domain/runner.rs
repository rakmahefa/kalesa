use std::path::{Path, PathBuf};

use super::{BinaryType, GameTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerKind {
    Native,
    Wine,
}

impl RunnerKind {
    pub fn for_binary(binary_type: BinaryType) -> Self {
        match binary_type {
            BinaryType::LinuxElf => Self::Native,
            BinaryType::WindowsPe => Self::Wine,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub kind: RunnerKind,
    pub wine_prefix: Option<PathBuf>,
}

impl Runner {
    pub fn for_target(target: &GameTarget, project_dir: &Path) -> Self {
        let kind = RunnerKind::for_binary(target.binary_type);
        let wine_prefix = matches!(kind, RunnerKind::Wine)
            .then(|| project_dir.join(".workdir/wine"));
        Self { kind, wine_prefix }
    }

    pub fn is_wine(&self) -> bool {
        matches!(self.kind, RunnerKind::Wine)
    }
}
