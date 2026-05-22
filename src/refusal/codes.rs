#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    GuardPreflight,
    RootNotFound,
    RootPermission,
    Io,
}

impl RefusalCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GuardPreflight => "E_GUARD_PREFLIGHT",
            Self::RootNotFound => "E_ROOT_NOT_FOUND",
            Self::RootPermission => "E_ROOT_PERMISSION",
            Self::Io => "E_IO",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::GuardPreflight => {
                "Required Claude PreToolUse guard hooks are missing or unhealthy"
            }
            Self::RootNotFound => "Root path does not exist",
            Self::RootPermission => "Cannot read root directory",
            Self::Io => "Filesystem error during scan",
        }
    }
}
