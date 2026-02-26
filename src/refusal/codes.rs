#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    RootNotFound,
    RootPermission,
    Io,
}

impl RefusalCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RootNotFound => "E_ROOT_NOT_FOUND",
            Self::RootPermission => "E_ROOT_PERMISSION",
            Self::Io => "E_IO",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RootNotFound => "Root path does not exist",
            Self::RootPermission => "Cannot read root directory",
            Self::Io => "Filesystem error during scan",
        }
    }
}
