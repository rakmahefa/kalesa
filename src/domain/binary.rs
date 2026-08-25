use std::path::Path;

use crate::error::Result;
use crate::pipeline::detect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    LinuxElf,
    WindowsPe,
}

impl BinaryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinuxElf => "linux",
            Self::WindowsPe => "windows",
        }
    }

    pub fn is_windows(self) -> bool {
        matches!(self, Self::WindowsPe)
    }

    pub fn detect(path: &Path) -> Result<Self> {
        detect::detect(path)
    }
}
