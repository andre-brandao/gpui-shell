//! Applications service for managing installed desktop applications.
//!
//! This module provides functionality for scanning and launching desktop
//! applications from standard XDG directories.

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
    /// Description or comment.
    pub description: Option<String>,
    /// Path to the desktop file.
    pub desktop_file: PathBuf,
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
                if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Some(app) = parse_desktop_file(&path) {
                        // Use name as key to deduplicate (user entries override system)
                        seen.insert(app.name.clone(), app);
                    }
                }
            }
        }
    }

    let mut apps: Vec<_> = seen.into_values().collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
