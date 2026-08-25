pub mod config;
pub mod domain;
pub mod error;
pub mod generators;
pub mod icon;
pub mod launcher;
pub mod pipeline;

pub use domain::{BinaryType, GameMetadata, GameTarget, LaunchOptions, Runner, RunnerBackend, RunnerKind};
pub use pipeline::detect::detect_binary_type;
pub use pipeline::{run as run_setup, run_with_options as run_setup_with_options, SetupOptions};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn write_temp(name: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kalesa_bintest_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(bytes).unwrap();
        path
    }

    #[test]
    fn rejects_truncated_elf() {
        let path = write_temp("test.elf", b"\x7fELF");
        assert!(matches!(detect_binary_type(&path), Err(error::KalesaError::InvalidElf(_))));
    }

    #[test]
    fn rejects_unknown_binary() {
        let path = write_temp("test.bin", b"garbage data");
        assert!(matches!(detect_binary_type(&path), Err(error::KalesaError::UnsupportedBinary(_))));
    }

    #[test]
    fn auto_runner_maps_appimage_to_native() {
        let target = GameTarget::new(PathBuf::from("game.AppImage"), BinaryType::AppImage);
        let runner = Runner::for_target(&target, std::path::Path::new("/tmp/game"));
        assert_eq!(runner.kind, RunnerKind::Native);
    }
}
