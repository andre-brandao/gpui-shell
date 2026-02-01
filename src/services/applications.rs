use gpui::Context;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Represents a desktop application entry.
#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub desktop_file: PathBuf,
}

impl Application {
    /// Launch the application.
    pub fn launch(&self) {
        let exec = self.exec.clone();
        std::thread::spawn(move || {
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

            let _ = std::process::Command::new("sh")
                .args(["-c", &exec_cleaned])
                .spawn();
        });
    }
}

/// Service for managing installed applications.
#[derive(Debug, Clone, Default)]
pub struct Applications {
    pub apps: Vec<Application>,
}

impl Applications {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let apps = scan_applications();

        // Could add file watching for updates in the future
        let _ = cx;

        Applications { apps }
    }

    /// Filter applications by search query.
    pub fn search(&self, query: &str) -> Vec<&Application> {
        if query.is_empty() {
            return self.apps.iter().collect();
        }

        let query_lower = query.to_lowercase();
        self.apps
            .iter()
            .filter(|app| {
                app.name.to_lowercase().contains(&query_lower)
                    || app
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }
}

fn scan_applications() -> Vec<Application> {
    let mut apps = Vec::new();
    let mut seen = HashMap::new();

    // Standard XDG directories for desktop entries
    let dirs = get_application_dirs();

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Some(app) = parse_desktop_file(&path) {
                        // Use name as key to deduplicate (user entries override system)
                        seen.insert(app.name.clone(), app);
                    }
                }
            }
        }
    }

    apps.extend(seen.into_values());
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

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

fn parse_desktop_file(path: &PathBuf) -> Option<Application> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut description = None;
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

    Some(Application {
        name,
        exec,
        icon,
        description,
        desktop_file: path.clone(),
    })
}
