use std::path::PathBuf;

use super::BinaryType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTarget {
    pub path: PathBuf,
    pub binary_type: BinaryType,
}

impl GameTarget {
    pub fn new(path: PathBuf, binary_type: BinaryType) -> Self {
        Self { path, binary_type }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMetadata {
    pub name: String,
    pub icon_path: Option<PathBuf>,
}

impl GameMetadata {
    pub fn new(name: String, icon_path: Option<PathBuf>) -> Self {
        Self { name, icon_path }
    }
}
