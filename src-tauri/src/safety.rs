use std::path::{Path, PathBuf};

/// Directories that must never be deleted, nor any of their ancestors.
fn protected_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/"),
        PathBuf::from("/System"),
        PathBuf::from("/Library"),
        PathBuf::from("/Applications"),
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/etc"),
        PathBuf::from("/var"),
        PathBuf::from("/opt"),
        PathBuf::from("/private"),
        PathBuf::from("C:\\Windows"),
        PathBuf::from("C:\\Program Files"),
        PathBuf::from("C:\\Program Files (x86)"),
    ];
    if let Some(home) = dirs::home_dir() {
        for important in [
            "", // home itself
            "Documents",
            "Desktop",
            "Downloads",
            "Pictures",
            "Movies",
            "Music",
            "Library", // ~/Library itself (children may still be deletable)
            ".ssh",
            ".gnupg",
        ] {
            roots.push(if important.is_empty() {
                home.clone()
            } else {
                home.join(important)
            });
        }
    }
    roots
}

/// True only for a `.app` bundle sitting directly inside `/Applications`
/// (macOS). Used by the uninstaller: the bundle itself may be trashed, but
/// never `/Applications` itself, nested paths inside other bundles, or
/// anything under `/System`.
fn is_user_app_bundle(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "app")
        && path.parent() == Some(Path::new("/Applications"))
}

/// A path is deletable only if it lives under the user's home directory or
/// the system temp directory (exception: a top-level app bundle in
/// `/Applications`, for the uninstaller), and is not itself a protected root
/// or an ancestor of one.
pub fn validate_deletable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err(format!("Path is not absolute: {}", path.display()));
    }
    // Canonicalize to defeat `..` tricks and symlink games. The path must exist.
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", path.display()))?;

    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let tmp = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    let allowed = canonical.starts_with(&home)
        || canonical.starts_with(&tmp)
        || is_user_app_bundle(&canonical);
    if !allowed {
        return Err(format!(
            "Refusing to touch path outside home/temp: {}",
            canonical.display()
        ));
    }

    for protected in protected_roots() {
        if canonical == protected || protected.starts_with(&canonical) {
            return Err(format!(
                "Refusing to delete protected path: {}",
                canonical.display()
            ));
        }
    }
    Ok(canonical)
}

/// Move a set of paths to the system trash. Never permanently deletes.
/// Returns (freed_bytes, errors).
pub fn trash_paths(paths: &[String]) -> (u64, Vec<String>) {
    let mut freed: u64 = 0;
    let mut errors: Vec<String> = Vec::new();
    for raw in paths {
        let path = Path::new(raw);
        match validate_deletable(path) {
            Ok(canonical) => {
                let size = crate::scan::path_size(&canonical);
                match trash::delete(&canonical) {
                    Ok(()) => freed += size,
                    Err(e) => errors.push(format!("{}: {e}", canonical.display())),
                }
            }
            Err(e) => errors.push(e),
        }
    }
    (freed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_home_and_system_paths() {
        let home = dirs::home_dir().unwrap();
        assert!(validate_deletable(&home).is_err());
        assert!(validate_deletable(Path::new("/")).is_err());
        assert!(validate_deletable(Path::new("/System")).is_err());
        assert!(validate_deletable(&home.join("Documents")).is_err());
    }

    #[test]
    fn rejects_relative_and_outside_paths() {
        assert!(validate_deletable(Path::new("relative/path")).is_err());
        assert!(validate_deletable(Path::new("/usr/lib")).is_err());
    }

    #[test]
    fn app_bundle_rule_only_matches_top_level_applications() {
        assert!(is_user_app_bundle(Path::new("/Applications/Foo.app")));
        // Never /Applications itself, nested paths, non-bundles, or /System.
        assert!(!is_user_app_bundle(Path::new("/Applications")));
        assert!(!is_user_app_bundle(Path::new("/Applications/Foo.app/Contents")));
        assert!(!is_user_app_bundle(Path::new("/Applications/Utilities/Foo.app")));
        assert!(!is_user_app_bundle(Path::new("/Applications/Foo.txt")));
        assert!(!is_user_app_bundle(Path::new("/System/Applications/Mail.app")));
    }

    #[test]
    fn still_rejects_applications_root() {
        assert!(validate_deletable(Path::new("/Applications")).is_err());
    }

    #[test]
    fn accepts_temp_file() {
        let dir = std::env::temp_dir().join("pc-cleaner-safety-test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("junk.txt");
        std::fs::write(&file, b"x").unwrap();
        assert!(validate_deletable(&file).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }
}
