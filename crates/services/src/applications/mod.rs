//! Applications service for managing installed desktop applications.
//!
//! This module provides functionality for scanning and launching desktop
//! applications from standard XDG directories.

pub mod icons;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use tracing::{debug, error};

/// Represents a desktop application entry.
#[derive(Debug, Clone)]
pub struct Application {
    /// Application name.
    pub name: String,
    /// Exec command.
    pub exec: String,
    /// Icon name or path.
    pub icon: Option<String>,
    /// Resolved icon filesystem path (for rendering).
    pub icon_path: Option<PathBuf>,
    /// Description or comment.
    pub description: Option<String>,
    /// Path to the desktop file.
    pub desktop_file: PathBuf,
    /// The `StartupWMClass` desktop-entry key, when present. Used to
    /// reliably match a running window's app_id/class back to this entry.
    pub startup_wm_class: Option<String>,
}

impl Application {
    /// Launch the application.
    pub fn launch(&self) {
        let exec = self.exec.clone();
        let name = self.name.clone();

        thread::spawn(move || {
            // Remove field codes like %f, %F, %u, %U, etc.
            let exec_cleaned = exec
                .replace("%f", "")
                .replace("%F", "")
                .replace("%u", "")
                .replace("%U", "")
                .replace("%d", "")
                .replace("%D", "")
                .replace("%n", "")
                .replace("%N", "")
                .replace("%i", "")
                .replace("%c", "")
                .replace("%k", "");

            debug!("Launching application: {} ({})", name, exec_cleaned.trim());

            match Command::new("sh").args(["-c", &exec_cleaned]).spawn() {
                Ok(_) => debug!("Application launched: {}", name),
                Err(e) => error!("Failed to launch {}: {}", name, e),
            }
        });
    }

    /// Get the icon name for lookup (without path or extension).
    pub fn icon_name(&self) -> Option<&str> {
        self.icon.as_ref().map(|i| {
            // If it's a path, extract just the filename without extension
            if i.contains('/') {
                std::path::Path::new(i)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(i)
            } else {
                i.as_str()
            }
        })
    }
}

/// Service for managing installed applications.
#[derive(Debug, Clone)]
pub struct ApplicationsService {
    apps: Vec<Application>,
}

impl ApplicationsService {
    /// Create a new applications service by scanning for desktop entries.
    pub fn new() -> Self {
        let apps = scan_applications();
        debug!("Found {} applications", apps.len());
        Self { apps }
    }

    /// Get all applications.
    pub fn all(&self) -> &[Application] {
        &self.apps
    }

    /// Filter applications by search query.
    pub fn search(&self, query: &str) -> Vec<&Application> {
        self.search_indices(query)
            .into_iter()
            .map(|ix| &self.apps[ix])
            .collect()
    }

    /// Filter applications by search query, returning indices into
    /// [`Self::all`].
    ///
    /// Callers that need to hold matches across frames want this rather
    /// than [`Self::search`]: the indices are plain `usize`, so they can be
    /// stored without borrowing the service.
    pub fn search_indices(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return (0..self.apps.len()).collect();
        }

        let query_lower = query.to_lowercase();
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, app)| {
                app.name.to_lowercase().contains(&query_lower)
                    || app
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&query_lower))
            })
            .map(|(ix, _)| ix)
            .collect()
    }

    /// Find an application by name (exact match, case-insensitive).
    pub fn find_by_name(&self, name: &str) -> Option<&Application> {
        let name_lower = name.to_lowercase();
        self.apps
            .iter()
            .find(|app| app.name.to_lowercase() == name_lower)
    }

    /// Rescan for applications.
    pub fn refresh(&mut self) {
        self.apps = scan_applications();
        debug!("Refreshed applications, found {}", self.apps.len());
    }
}

impl Default for ApplicationsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan for desktop applications in standard XDG directories.
fn scan_applications() -> Vec<Application> {
    let mut seen = HashMap::new();

    // Standard XDG directories for desktop entries
    let dirs = get_application_dirs();

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "desktop").unwrap_or(false)
                    && let Some(app) = parse_desktop_file(&path)
                {
                    // Use name as key to deduplicate (user entries override system)
                    seen.insert(app.name.clone(), app);
                }
            }
        }
    }

    let mut apps: Vec<_> = seen.into_values().collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// Get XDG application directories in priority order.
fn get_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // User-specific directory (higher priority)
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    // System directories
    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        for dir in std::env::split_paths(&data_dirs) {
            dirs.push(dir.join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }

    dirs
}

/// Parse a desktop file and extract application information.
fn parse_desktop_file(path: &PathBuf) -> Option<Application> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut description = None;
    let mut startup_wm_class = None;
    let mut no_display = false;
    let mut hidden = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Exec" => exec = Some(value.to_string()),
                "Icon" => icon = Some(value.to_string()),
                "Comment" if description.is_none() => description = Some(value.to_string()),
                "GenericName" if description.is_none() => description = Some(value.to_string()),
                "StartupWMClass" => startup_wm_class = Some(value.to_string()),
                "NoDisplay" => no_display = value == "true",
                "Hidden" => hidden = value == "true",
                _ => {}
            }
        }
    }

    // Skip hidden or no-display entries
    if no_display || hidden {
        return None;
    }

    let name = name?;
    let exec = exec?;

    let icon_path = icon.as_deref().and_then(icons::lookup_icon);

    Some(Application {
        name,
        exec,
        icon,
        icon_path,
        description,
        desktop_file: path.clone(),
        startup_wm_class,
    })
}

/// Match a window's `app_id`/class against the application catalog.
///
/// Tries an exact (case-insensitive) `StartupWMClass` match first, then
/// falls back to comparing against the `Exec` command's first whitespace-
/// separated token (its binary name), also case-insensitive. Finally, it
/// compares against the desktop-file filename and stem.
pub fn match_app_id<'a>(apps: &'a [Application], app_id: &str) -> Option<&'a Application> {
    let app_id_lower = app_id.to_lowercase();

    apps.iter()
        .find(|a| {
            a.startup_wm_class
                .as_deref()
                .is_some_and(|c| c.to_lowercase() == app_id_lower)
        })
        .or_else(|| {
            apps.iter().find(|a| {
                a.exec
                    .split_whitespace()
                    .next()
                    .map(|bin| bin.to_lowercase() == app_id_lower)
                    .unwrap_or(false)
            })
        })
        .or_else(|| {
            apps.iter().find(|a| {
                a.desktop_file
                    .file_name()
                    .and_then(|filename| filename.to_str())
                    .is_some_and(|filename| filename.to_lowercase() == app_id_lower)
                    || a.desktop_file
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| stem.to_lowercase() == app_id_lower)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, exec: &str, startup_wm_class: Option<&str>) -> Application {
        Application {
            name: name.to_string(),
            exec: exec.to_string(),
            icon: None,
            icon_path: None,
            description: None,
            desktop_file: PathBuf::from(format!("/usr/share/applications/{name}.desktop")),
            startup_wm_class: startup_wm_class.map(str::to_string),
        }
    }

    #[test]
    fn search_indices_agrees_with_search() {
        let mut firefox = app("Firefox", "firefox %u", None);
        firefox.description = Some("Web Browser".to_string());
        let service = ApplicationsService {
            apps: vec![
                firefox,
                app("Kitty", "kitty", None),
                app("GIMP", "gimp", None),
            ],
        };

        for query in ["", "fire", "FIRE", "web browser", "nope"] {
            let by_ref: Vec<&str> = service.search(query).iter().map(|a| &*a.name).collect();
            let by_index: Vec<&str> = service
                .search_indices(query)
                .into_iter()
                .map(|ix| &*service.all()[ix].name)
                .collect();
            assert_eq!(by_ref, by_index, "query {query:?}");
        }

        assert_eq!(service.search_indices(""), vec![0, 1, 2]);
        assert_eq!(service.search_indices("fire"), vec![0]);
        // Description is searched too, not just the name.
        assert_eq!(service.search_indices("browser"), vec![0]);
        assert!(service.search_indices("nope").is_empty());
    }

    #[test]
    fn matches_exact_startup_wm_class() {
        let apps = vec![app("Firefox", "firefox %u", Some("firefox"))];
        let matched = match_app_id(&apps, "firefox").unwrap();
        assert_eq!(matched.name, "Firefox");
    }

    #[test]
    fn startup_wm_class_match_is_case_insensitive() {
        let apps = vec![app("Firefox", "firefox %u", Some("Firefox"))];
        let matched = match_app_id(&apps, "firefox").unwrap();
        assert_eq!(matched.name, "Firefox");
    }

    #[test]
    fn falls_back_to_exec_basename_when_no_startup_wm_class() {
        let apps = vec![app("Kitty", "kitty", None)];
        let matched = match_app_id(&apps, "kitty").unwrap();
        assert_eq!(matched.name, "Kitty");
    }

    #[test]
    fn falls_back_to_desktop_filename_when_other_identifiers_do_not_match() {
        let mut helium = app("Helium Browser", "helium-browser --new-window", None);
        helium.desktop_file = PathBuf::from("/usr/share/applications/helium.desktop");
        let apps = vec![helium];
        let matched = match_app_id(&apps, "HELIUM").unwrap();
        assert_eq!(matched.name, "Helium Browser");
    }

    #[test]
    fn no_match_returns_none() {
        let apps = vec![app("Firefox", "firefox %u", Some("firefox"))];
        assert!(match_app_id(&apps, "totally-unrelated-app").is_none());
    }

    #[test]
    fn parses_startup_wm_class_from_desktop_file() {
        let dir = std::env::temp_dir().join(format!("gpuishell-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-app.desktop");
        std::fs::write(
            &path,
            "[Desktop Entry]\nName=Test App\nExec=test-app\nStartupWMClass=test-app-wm\n",
        )
        .unwrap();

        let parsed = parse_desktop_file(&path).unwrap();
        assert_eq!(parsed.startup_wm_class.as_deref(), Some("test-app-wm"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
