//! Wallpaper view for browsing and applying wallpapers.

pub mod config;

use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use gpui::{AnyElement, App, div, img, prelude::*, px};
use services::WallpaperCommand;
use ui::{
    ActiveTheme, Color, Label, LabelCommon, LabelSize, ListItem, ListItemSpacing, Switch,
    SwitchSize, spacing,
};

use self::config::WallpaperConfig;
use crate::launcher::view::{LauncherView, ViewContext, render_footer_hints};
use crate::state::AppState;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// Wallpaper view - browse and set wallpapers.
///
/// State Management:
/// - `directory`: Parsed from config, immutable
/// - `matugen_enabled`: Arc<AtomicBool> for toggle state
///   - Arc: Shared ownership so closures can own a reference
///   - AtomicBool: Thread-safe mutation without &mut self (trait requires &self)
/// - `matugen_dark_mode`: Arc<AtomicBool> for dark/light mode toggle
pub struct WallpaperView {
    prefix: String,
    directory: PathBuf,
    matugen_type: String,
    matugen_source_color_index: usize,
    matugen_enabled: Arc<AtomicBool>,
    matugen_dark_mode: Arc<AtomicBool>,
}

impl WallpaperView {
    pub fn new(config: &WallpaperConfig) -> Self {
        let directory = expand_tilde(&config.directory);
        Self {
            prefix: config.prefix.clone(),
            directory,
            matugen_type: "scheme-tonal-spot".into(),
            matugen_source_color_index: 0,
            matugen_enabled: Arc::new(AtomicBool::new(true)),
            matugen_dark_mode: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Apply wallpaper, optionally generating a theme with matugen.
    ///
    /// This is a static helper that doesn't use `&self` so it can be called
    /// from closures that need 'static lifetime.
    fn apply_wallpaper(
        path: PathBuf,
        matugen_enabled: &Arc<AtomicBool>,
        matugen_dark_mode: &Arc<AtomicBool>,
        matugen_type: &str,
        matugen_source_color_index: usize,
        cx: &mut App,
    ) {
        use ui::{Base16Colors, Theme};

        // Always set the wallpaper first
        AppState::wallpaper(cx).dispatch(WallpaperCommand::SetWallpaper(path.clone()));

        // If matugen is enabled, generate theme in background
        if matugen_enabled.load(Ordering::Relaxed) {
            let dark_mode = matugen_dark_mode.load(Ordering::Relaxed);
            let mode = if dark_mode { "dark" } else { "light" };
            let scheme_type = matugen_type.to_string();
            let source_index = matugen_source_color_index;

            cx.spawn(async move |cx| {
                let path_display = path.display().to_string();

                // Run matugen in background executor (blocking operation)
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        Base16Colors::generate_from_wallpaper(
                            &path,
                            &mode,
                            &scheme_type,
                            source_index,
                        )
                    })
                    .await;

                match result {
                    Ok(theme) => {
                        tracing::debug!("Matugen theme generated for: {}", path_display);
                        // Apply the generated theme
                        let _ = cx.update(|cx| {
                            Theme::set(theme, cx);
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to generate matugen theme: {}", e);
                    }
                }
            })
            .detach();
        }
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
}

struct WallpaperEntry {
    path: PathBuf,
    name: String,
}

fn scan_wallpapers(wallpaper_dir: &PathBuf) -> Vec<WallpaperEntry> {
    let Ok(read_dir) = std::fs::read_dir(wallpaper_dir) else {
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

fn filtered_entries(dir: &PathBuf, query: &str) -> Vec<WallpaperEntry> {
    let query_lower = query.to_lowercase();
    scan_wallpapers(dir)
        .into_iter()
        .filter(|e| query_lower.is_empty() || e.name.to_lowercase().contains(&query_lower))
        .collect()
}

impl LauncherView for WallpaperView {
    fn prefix(&self) -> &str {
        &self.prefix
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
        filtered_entries(&self.directory, vx.query).len()
    }

    fn render_header(&self, _vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let accent_primary = theme.accent.primary;
        let bg_secondary = theme.bg.secondary;
        let border = theme.border.default;

        let enabled = self.matugen_enabled.load(Ordering::Relaxed);
        let dark_mode = self.matugen_dark_mode.load(Ordering::Relaxed);

        let matugen_enabled_atomic = Arc::clone(&self.matugen_enabled);
        let matugen_dark_mode_atomic = Arc::clone(&self.matugen_dark_mode);

        Some(
            div()
                .flex()
                .flex_col()
                .bg(bg_secondary)
                .border_b_1()
                .border_color(border)
                // Main toggle section
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(spacing::LG))
                        .py(px(spacing::MD))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(spacing::MD))
                                // Icon indicator
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(32.))
                                        .h(px(32.))
                                        .rounded(px(6.))
                                        .bg(if enabled {
                                            accent_primary
                                        } else {
                                            theme.interactive.default
                                        })
                                        .text_color(if enabled {
                                            theme.bg.primary
                                        } else {
                                            text_muted
                                        })
                                        .text_base()
                                        .child("󱥚"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .text_color(text_primary)
                                                .text_sm()
                                                .child("Material You Theming"),
                                        )
                                        .child(
                                            div()
                                                .text_color(text_muted)
                                                .text_xs()
                                                .child("Auto-generate theme colors from wallpaper"),
                                        ),
                                ),
                        )
                        .child(
                            Switch::new("matugen-toggle")
                                .checked(enabled)
                                .size(SwitchSize::Medium)
                                .on_click(move |checked, _, _cx| {
                                    matugen_enabled_atomic.store(*checked, Ordering::Relaxed);
                                }),
                        ),
                )
                // Dark/Light mode selector (animated collapse)
                .when(enabled, |this| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(spacing::LG))
                            .pb(px(spacing::MD))
                            .pt(px(spacing::XS))
                            .child(
                                div().flex().items_center().gap(px(spacing::SM)).child(
                                    div().text_color(text_muted).text_xs().child("Theme Mode"),
                                ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .child(
                                        div()
                                            .text_color(if dark_mode {
                                                text_muted
                                            } else {
                                                accent_primary
                                            })
                                            .text_xs()
                                            .child("󰖨"),
                                    )
                                    .child(
                                        Switch::new("matugen-dark-mode-toggle")
                                            .checked(dark_mode)
                                            .size(SwitchSize::Small)
                                            .on_click(move |checked, _, _cx| {
                                                matugen_dark_mode_atomic
                                                    .store(*checked, Ordering::Relaxed);
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_color(if dark_mode {
                                                accent_primary
                                            } else {
                                                text_muted
                                            })
                                            .text_xs()
                                            .child("󰖔"),
                                    ),
                            ),
                    )
                })
                .into_any_element(),
        )
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let entries = filtered_entries(&self.directory, vx.query);
        let Some(entry) = entries.get(index) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let path_for_click = entry.path.clone();
        let preview_path = entry.path.clone();
        let interactive_default = theme.interactive.default;

        // Clone Arc fields for closure
        let matugen_enabled = Arc::clone(&self.matugen_enabled);
        let matugen_dark_mode = Arc::clone(&self.matugen_dark_mode);
        let matugen_type = self.matugen_type.clone();
        let matugen_source_color_index = self.matugen_source_color_index;

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
            .on_click(move |_, _, cx| {
                WallpaperView::apply_wallpaper(
                    path_for_click.clone(),
                    &matugen_enabled,
                    &matugen_dark_mode,
                    &matugen_type,
                    matugen_source_color_index,
                    cx,
                );
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

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let entries = filtered_entries(&self.directory, vx.query);
        if let Some(entry) = entries.get(index) {
            Self::apply_wallpaper(
                entry.path.clone(),
                &self.matugen_enabled,
                &self.matugen_dark_mode,
                &self.matugen_type,
                self.matugen_source_color_index,
                cx,
            );
            false
        } else {
            false
        }
    }

    fn render_footer_bar(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        render_footer_hints(vec![("Apply", "Enter"), ("Close", "Esc")], cx)
    }
}
