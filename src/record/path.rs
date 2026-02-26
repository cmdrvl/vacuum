use std::path::Path;

pub fn normalize_relative(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn native_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{native_string, normalize_relative};

    #[test]
    fn relative_paths_are_normalized_to_forward_slashes() {
        let path = Path::new("nested\\inner/file.csv");
        assert_eq!(normalize_relative(path), "nested/inner/file.csv");
    }

    #[test]
    fn native_paths_preserve_os_representation() {
        let path = Path::new("a/b/c.txt");
        assert_eq!(native_string(path), path.to_string_lossy());
    }
}
