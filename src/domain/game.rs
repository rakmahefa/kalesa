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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameMetadata {
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub version: Option<String>,
    pub developer: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
}

impl GameMetadata {
    pub fn new(name: String, icon_path: Option<PathBuf>) -> Self {
        Self {
            name,
            icon_path,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchOptions {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl LaunchOptions {
    pub fn validate(&self) -> crate::error::Result<()> {
        for (key, _) in &self.env {
            if key.is_empty()
                || !key.chars().enumerate().all(|(i, c)| {
                    (i == 0 && (c == '_' || c.is_ascii_alphabetic()))
                        || (i > 0 && (c == '_' || c.is_ascii_alphanumeric()))
                })
            {
                return Err(crate::error::KalesaError::InvalidEnvironmentKey(key.clone()));
            }
        }
        Ok(())
    }
}