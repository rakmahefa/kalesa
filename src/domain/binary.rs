#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryType {
    LinuxElf,
    AppImage,
    WindowsPe,
}

impl BinaryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinuxElf => "linux",
            Self::AppImage => "appimage",
            Self::WindowsPe => "windows",
        }
    }

    pub fn is_windows(self) -> bool {
        matches!(self, Self::WindowsPe)
    }

    pub fn is_linux(self) -> bool {
        matches!(self, Self::LinuxElf | Self::AppImage)
    }
}