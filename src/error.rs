use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KalesaError {
    #[error("I/O error while processing {context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("target executable does not exist: {0}")]
    TargetNotFound(PathBuf),

    #[error("target is not a regular file: {0}")]
    TargetNotFile(PathBuf),

    #[error("unsupported binary format: {0}")]
    UnsupportedBinary(PathBuf),

    #[error("invalid ELF binary: {0}")]
    InvalidElf(PathBuf),

    #[error("invalid PE binary: {0}")]
    InvalidPe(PathBuf),

    #[error("Windows runner requires a Wine prefix")]
    MissingWinePrefix,

    #[error("Proton runner requires a Proton executable path")]
    MissingProtonPath,

    #[error("invalid environment variable name: {0}")]
    InvalidEnvironmentKey(String),

    #[error("invalid Desktop Entry value: {0}")]
    InvalidDesktopValue(String),

    #[error("invalid runtime launcher configuration: {0}")]
    InvalidRuntimeConfig(String),

    #[error("failed to execute launcher command '{command}': {source}")]
    CommandIo {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error("launcher command '{command}' exited with status {code:?}")]
    CommandFailed { command: String, code: Option<i32> },

    #[error("failed to serialize configuration: {0}")]
    ConfigSerialize(#[from] serde_yaml::Error),
}

impl KalesaError {
    pub fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }
}

pub type Result<T> = std::result::Result<T, KalesaError>;
