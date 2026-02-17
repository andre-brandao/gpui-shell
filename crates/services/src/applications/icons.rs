//! Freedesktop icon theme lookup.
//!
//! Resolves icon names (e.g., "firefox") to filesystem paths by searching
//! standard XDG icon theme directories (hicolor) and `/usr/share/pixmaps`.

use std::path::{Path, PathBuf};

/// Size subdirectories to search, in priority order.
const SIZE_DIRS: &[&str] = &[
    "scalable", "48x48", "64x64", "128x128", "256x256", "32x32", "24x24", "22x22", "16x16",
];

/// File extensions to try, in priority order.
const EXTENSIONS: &[&str] = &["svg", "png"];

/// Resolve an icon name or path to an actual filesystem path.
///
/// If `name` is already an absolute path that exists, returns it directly.
/// Otherwise searches hicolor icon theme directories and `/usr/share/pixmaps`.
pub fn lookup_icon(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }

    // If it's already an absolute path, check if it exists
    let path = Path::new(name);
    if path.is_absolute() {
        return path.exists().then(|| path.to_path_buf());
    }

    // Search icon theme directories
    for base in icon_theme_base_dirs() {
        let hicolor = base.join("hicolor");
        if let Some(found) = search_theme_dir(&hicolor, name) {
            return Some(found);
        }
    }

    // Fallback: /usr/share/pixmaps
    for ext in EXTENSIONS {
        let pixmap = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
        if pixmap.exists() {
            return Some(pixmap);
        }
    }

    None
}

/// Search a single theme directory for an icon name.
fn search_theme_dir(theme_dir: &Path, name: &str) -> Option<PathBuf> {
    for size in SIZE_DIRS {
        let apps_dir = theme_dir.join(size).join("apps");
        for ext in EXTENSIONS {
            let candidate = apps_dir.join(format!("{name}.{ext}"));
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Get icon theme base directories in search order.
fn icon_theme_base_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".icons"));
        dirs.push(home.join(".local/share/icons"));
    }

    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&data_dirs) {
            dirs.push(dir.join("icons"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/icons"));
        dirs.push(PathBuf::from("/usr/share/icons"));
    }

    dirs
}
