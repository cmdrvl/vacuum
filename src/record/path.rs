use std::path::Path;

pub fn normalize_relative(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
