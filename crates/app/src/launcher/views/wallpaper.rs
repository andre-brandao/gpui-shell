//! Wallpaper view for browsing and applying wallpapers.

use std::path::PathBuf;

use gpui::{AnyElement, App, div, img, prelude::*, px};
use services::WallpaperCommand;
use ui::{ActiveTheme, Color, Label, LabelCommon, LabelSize, ListItem, ListItemSpacing};

use crate::launcher::view::{LauncherView, ViewContext};

const WALLPAPER_DIR: &str = "Pictures/Wallpapers";
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// Wallpaper view - browse and set wallpapers.
pub struct WallpaperView;

struct WallpaperEntry {
    path: PathBuf,
    name: String,
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

fn filtered_entries(query: &str) -> Vec<WallpaperEntry> {
    let query_lower = query.to_lowercase();
    scan_wallpapers()
        .into_iter()
        .filter(|e| query_lower.is_empty() || e.name.to_lowercase().contains(&query_lower))
        .collect()
}

impl LauncherView for WallpaperView {
    fn prefix(&self) -> &'static str {
        ";wp"
    }

    fn name(&self) -> &'static str {
        "Wallpaper"
    }

    fn icon(&self) -> &'static str {
        "󰸉"
    }

    fn description(&self) -> &'static str {
        "Browse and set wallpapers"
    }

    fn match_count(&self, vx: &ViewContext, _cx: &App) -> usize {
        filtered_entries(vx.query).len()
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let entries = filtered_entries(vx.query);
        let Some(entry) = entries.get(index) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let path = entry.path.clone();
        let preview_path = entry.path.clone();
        let wallpaper_service = vx.services.wallpaper.clone();
        let interactive_default = theme.interactive.default;

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
                    .w(px(40.))
                    .h(px(28.))
                    .rounded(px(4.))
                    .bg(interactive_default)
                    .overflow_hidden()
                    .child(img(preview_path).size_full()),
            )
            .on_click(move |_, _, _cx| {
                wallpaper_service.dispatch(WallpaperCommand::SetWallpaper(path.clone()));
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

    fn on_select(&self, index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let entries = filtered_entries(vx.query);
        if let Some(entry) = entries.get(index) {
            vx.services
                .wallpaper
                .dispatch(WallpaperCommand::SetWallpaper(entry.path.clone()));
            true
        } else {
            false
        }
    }

    fn footer_actions(&self, _vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Apply", "Enter"), ("Close", "Esc")]
    }
}
