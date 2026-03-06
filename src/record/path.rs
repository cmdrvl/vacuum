use std::path::{Component, Path};

pub fn normalize_relative(path: &Path) -> String {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::CurDir | Component::RootDir => {}
            Component::ParentDir => components.push("..".to_string()),
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().into_owned())
            }
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
        }
    }

    components.join("/")
}

pub fn native_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{native_string, normalize_relative};

    #[cfg(windows)]
    #[test]
    fn relative_paths_are_normalized_to_forward_slashes() {
        let path = Path::new(r"nested\inner\file.csv");
        assert_eq!(normalize_relative(path), "nested/inner/file.csv");
    }

    #[cfg(not(windows))]
    #[test]
    fn relative_paths_preserve_literal_backslashes_inside_components() {
        let path = Path::new(r"nested\inner/file.csv");
        assert_eq!(normalize_relative(path), r"nested\inner/file.csv");
    }

    #[test]
    fn native_paths_preserve_os_representation() {
        let path = Path::new("a/b/c.txt");
        assert_eq!(native_string(path), path.to_string_lossy());
    }
}
