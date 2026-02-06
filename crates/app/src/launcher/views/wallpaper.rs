//! Wallpaper view — browse and apply wallpapers using swww.
//!
//! Scans `~/Pictures/Wallpapers/` for image files and sets them via `swww img`.

use std::path::PathBuf;

use gpui::{AnyElement, App, Context, EventEmitter};
use ui::{ActiveTheme, ListItem, ListItemSpacing, prelude::*};

use crate::launcher::view::{FooterAction, LauncherView, ViewEvent};

const WALLPAPER_DIR: &str = "Pictures/Wallpapers";
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

struct WallpaperEntry {
    path: PathBuf,
    name: String,
}

pub struct WallpaperView {
    entries: Vec<WallpaperEntry>,
    filtered: Vec<usize>,
    query: String,
}

impl EventEmitter<ViewEvent> for WallpaperView {}

impl WallpaperView {
    pub fn new() -> Self {
        let entries = scan_wallpapers();
        let filtered = (0..entries.len()).collect();
        Self {
            entries,
            filtered,
            query: String::new(),
        }
    }

    fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
    }
}

impl LauncherView for WallpaperView {
    fn id(&self) -> &'static str {
        "wallpaper"
    }

    fn prefix(&self) -> &'static str {
        ";wp"
    }

    fn name(&self) -> &'static str {
        "Wallpaper"
    }

    fn icon(&self) -> IconName {
        IconName::Image
    }

    fn description(&self) -> &'static str {
        "Browse and set wallpapers"
    }

    fn match_count(&self) -> usize {
        self.filtered.len()
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_string();
        self.refilter();
    }

    fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement {
        let Some(&entry_idx) = self.filtered.get(index) else {
            return gpui::Empty.into_any_element();
        };
        let entry = &self.entries[entry_idx];
        let colors = cx.theme().colors();
        let path = entry.path.clone();

        let extension = entry
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_uppercase();

        ListItem::new(format!("wp-{index}"))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(32.))
                    .h(px(32.))
                    .rounded(px(6.))
                    .bg(colors.element_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(IconName::Image).color(Color::Muted)),
            )
            .on_click(move |_, _, _| {
                set_wallpaper(&path);
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(Label::new(entry.name.clone()).size(LabelSize::Default))
                    .child(
                        Label::new(extension)
                            .size(LabelSize::XSmall)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn confirm(&mut self, index: usize, _cx: &mut Context<Self>) {
        if let Some(&entry_idx) = self.filtered.get(index) {
            set_wallpaper(&self.entries[entry_idx].path);
        }
    }

    fn footer_actions(&self) -> Vec<FooterAction> {
        vec![
            FooterAction {
                label: "Apply",
                key: "Enter",
            },
            FooterAction {
                label: "Close",
                key: "Esc",
            },
        ]
    }
}

fn scan_wallpapers() -> Vec<WallpaperEntry> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let wallpaper_dir = PathBuf::from(home).join(WALLPAPER_DIR);

    let Ok(read_dir) = std::fs::read_dir(&wallpaper_dir) else {
        tracing::warn!(
            "Could not read wallpaper directory: {}",
            wallpaper_dir.display()
        );
        return Vec::new();
    };

    let mut entries: Vec<WallpaperEntry> = read_dir
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        })
        .map(|entry| {
            let path = entry.path();
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            WallpaperEntry { path, name }
        })
        .collect();

    entries.sort_by_key(|e| e.name.to_lowercase());
    entries
}

fn set_wallpaper(path: &std::path::Path) {
    let path = path.to_path_buf();
    std::thread::spawn(move || {
        let result = std::process::Command::new("swww")
            .args([
                "img",
                &path.to_string_lossy(),
                "--transition-type",
                "fade",
                "--transition-duration",
                "1",
            ])
            .spawn();

        if let Err(error) = result {
            tracing::error!("Failed to set wallpaper via swww: {}", error);
        }
    });
}
