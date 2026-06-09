use std::path::{Component, Path, PathBuf};

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub(crate) fn extensionless(path: &Path) -> PathBuf {
    const TYPESCRIPT_EXTENSIONS: &[&str] = &["ts", "tsx"];
    if TYPESCRIPT_EXTENSIONS.contains(&path.extension().and_then(|ext| ext.to_str()).unwrap_or(""))
    {
        return path.with_extension("");
    }
    path.to_path_buf()
}

pub(crate) fn is_barrel_file(file: &Path) -> bool {
    file.file_name().and_then(|name| name.to_str()) == Some("index.ts")
}

pub(crate) fn is_relative_specifier(specifier: &str) -> bool {
    specifier.starts_with('.') || specifier.starts_with('/')
}
