pub fn is_hook_file(path: &std::path::Path) -> bool {
    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();

    if file_stem.ends_with(".hook") || file_stem.ends_with(".hooks") {
        return true;
    }

    if let Some(parent) = path.parent()
        && parent.file_name().map(|n| n.to_string_lossy()) == Some("hooks".into())
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn hook_file_by_suffix() {
        assert!(is_hook_file(Path::new("useAuth.hook.ts")));
        assert!(is_hook_file(Path::new("useAuth.hooks.ts")));
    }

    #[test]
    fn hook_file_in_hooks_folder() {
        assert!(is_hook_file(Path::new("src/hooks/useAuth.ts")));
        assert!(is_hook_file(Path::new("hooks/useAuth.tsx")));
    }

    #[test]
    fn non_hook_file() {
        assert!(!is_hook_file(Path::new("src/components/Button.tsx")));
        assert!(!is_hook_file(Path::new("src/utils/format.ts")));
    }
}
