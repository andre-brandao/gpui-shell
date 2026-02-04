//! Theme repository for fetching and caching Base16 color schemes.
//!
//! Clones the schemes repository (default: tinted-theming/schemes) via `git`
//! and reads Base16 YAML files from the local clone.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, warn};

// =============================================================================
// Base16 Scheme Types
// =============================================================================

/// A parsed Base16 color scheme.
#[derive(Debug, Clone)]
pub struct Base16Scheme {
    pub name: String,
    pub author: String,
    pub variant: String,
    pub slug: String,
    pub palette: Base16Palette,
}

/// The 16-color palette from a Base16 scheme. Values are hex strings like "#1e1e2e".
#[derive(Debug, Clone)]
pub struct Base16Palette {
    pub base00: String,
    pub base01: String,
    pub base02: String,
    pub base03: String,
    pub base04: String,
    pub base05: String,
    pub base06: String,
    pub base07: String,
    pub base08: String,
    pub base09: String,
    pub base0a: String,
    pub base0b: String,
    pub base0c: String,
    pub base0d: String,
    pub base0e: String,
    pub base0f: String,
}

// Serde types for YAML parsing
#[derive(Deserialize)]
struct Base16Yaml {
    name: Option<String>,
    author: Option<String>,
    variant: Option<String>,
    palette: Base16PaletteYaml,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Base16PaletteYaml {
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    base0A: String,
    base0B: String,
    base0C: String,
    base0D: String,
    base0E: String,
    base0F: String,
}

// =============================================================================
// Theme Repository
// =============================================================================

/// Repository for fetching and caching Base16 theme schemes.
///
/// Maintains a shallow git clone of the schemes repository and reads
/// Base16 YAML files from its `base16/` subdirectory.
#[derive(Debug, Clone)]
pub struct ThemeRepository {
    repo: String,
    branch: String,
    cache_dir: PathBuf,
}

impl ThemeRepository {
    /// Create a new theme repository.
    ///
    /// - `repo`: GitHub "owner/repo" (default: "tinted-theming/schemes")
    /// - `branch`: Git branch (default: "spec-0.11")
    pub fn new(repo: Option<String>, branch: Option<String>) -> Self {
        let cache_dir = cache_directory();
        Self {
            repo: repo.unwrap_or_else(|| "tinted-theming/schemes".to_string()),
            branch: branch.unwrap_or_else(|| "spec-0.11".to_string()),
            cache_dir,
        }
    }

    /// Directory containing Base16 YAML files inside the cloned repo.
    fn schemes_dir(&self) -> PathBuf {
        self.cache_dir.join("base16")
    }

    /// Load all cached Base16 schemes from the local clone.
    pub fn load_cached(&self) -> Vec<Base16Scheme> {
        let dir = self.schemes_dir();
        if !dir.exists() {
            return Vec::new();
        }

        let mut schemes = Vec::new();
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read theme schemes dir: {}", e);
                return Vec::new();
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "yaml") {
                match parse_scheme_file(&path) {
                    Ok(scheme) => schemes.push(scheme),
                    Err(e) => {
                        warn!("Failed to parse theme {:?}: {}", path.file_name(), e);
                    }
                }
            }
        }

        schemes.sort_by_key(|s| s.name.to_lowercase());
        debug!("Loaded {} cached Base16 schemes", schemes.len());
        schemes
    }

    /// Clone or update the schemes repository, then load all Base16 schemes.
    pub fn fetch_and_cache(&self) -> Result<Vec<Base16Scheme>> {
        let clone_url = format!("https://github.com/{}.git", self.repo);

        if self.cache_dir.join(".git").exists() {
            debug!("Updating existing theme repo clone");
            let output = Command::new("git")
                .args(["pull", "--ff-only"])
                .current_dir(&self.cache_dir)
                .output()
                .context("Failed to execute git pull")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("git pull failed, continuing with cached data: {}", stderr);
            }
        } else {
            if let Some(parent) = self.cache_dir.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create theme cache parent directory")?;
            }

            debug!("Cloning theme repo: {}", clone_url);
            let output = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "--branch",
                    &self.branch,
                    &clone_url,
                    &self.cache_dir.to_string_lossy(),
                ])
                .output()
                .context("Failed to execute git clone")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git clone failed: {}", stderr);
            }
        }

        Ok(self.load_cached())
    }
}

// =============================================================================
// Stylix
// =============================================================================

const STYLIX_PALETTE_PATH: &str = "/etc/stylix/palette.json";

/// Serde type for the Stylix palette.json flat format.
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct StylixPalette {
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    base0A: String,
    base0B: String,
    base0C: String,
    base0D: String,
    base0E: String,
    base0F: String,
    scheme: Option<String>,
    author: Option<String>,
    slug: Option<String>,
}

/// Load the active Stylix theme from `/etc/stylix/palette.json`, if present.
pub fn load_stylix_scheme() -> Option<Base16Scheme> {
    let path = std::path::Path::new(STYLIX_PALETTE_PATH);
    if !path.exists() {
        return None;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read Stylix palette: {}", e);
            return None;
        }
    };

    let palette: StylixPalette = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to parse Stylix palette: {}", e);
            return None;
        }
    };

    let slug = palette.slug.unwrap_or_else(|| "stylix".to_string());
    let name = palette.scheme.unwrap_or_else(|| slug.clone());

    Some(Base16Scheme {
        name,
        author: palette.author.unwrap_or_default(),
        variant: "dark".to_string(),
        slug,
        palette: Base16Palette {
            base00: palette.base00,
            base01: palette.base01,
            base02: palette.base02,
            base03: palette.base03,
            base04: palette.base04,
            base05: palette.base05,
            base06: palette.base06,
            base07: palette.base07,
            base08: palette.base08,
            base09: palette.base09,
            base0a: palette.base0A,
            base0b: palette.base0B,
            base0c: palette.base0C,
            base0d: palette.base0D,
            base0e: palette.base0E,
            base0f: palette.base0F,
        },
    })
}

// =============================================================================
// Helpers
// =============================================================================

fn cache_directory() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".cache")
        });
    base.join("gpuishell").join("themes")
}

fn parse_scheme_file(path: &std::path::Path) -> Result<Base16Scheme> {
    let content = fs::read_to_string(path)?;
    let yaml: Base16Yaml = serde_yaml::from_str(&content)?;

    let slug = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let name = yaml.name.unwrap_or_else(|| slug.clone());

    Ok(Base16Scheme {
        name,
        author: yaml.author.unwrap_or_default(),
        variant: yaml.variant.unwrap_or_else(|| "dark".to_string()),
        slug,
        palette: Base16Palette {
            base00: yaml.palette.base00,
            base01: yaml.palette.base01,
            base02: yaml.palette.base02,
            base03: yaml.palette.base03,
            base04: yaml.palette.base04,
            base05: yaml.palette.base05,
            base06: yaml.palette.base06,
            base07: yaml.palette.base07,
            base08: yaml.palette.base08,
            base09: yaml.palette.base09,
            base0a: yaml.palette.base0A,
            base0b: yaml.palette.base0B,
            base0c: yaml.palette.base0C,
            base0d: yaml.palette.base0D,
            base0e: yaml.palette.base0E,
            base0f: yaml.palette.base0F,
        },
    })
}
