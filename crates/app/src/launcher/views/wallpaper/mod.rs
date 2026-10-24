//! Wallpaper view: a grid of thumbnails. Applying one generates a matching
//! theme with matugen in the background; it waits above the grid until
//! picked.

pub mod config;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use gpui::{
    AnyElement, App, Context, EntityId, EventEmitter, Global, Hsla, ObjectFit, Pixels,
    SharedString, Window, div, img, prelude::*, px, size,
};
use services::WallpaperCommand;
use ui::{
    ActiveTheme, Base16Palette, Color, Icon, IconName, IconSize, Label, LabelCommon, Radius,
    Spacing, TextSize, WallpaperPalettes,
};

use self::config::WallpaperConfig;
use crate::launcher::item::{Item, ItemIcon};
use crate::launcher::row::thumbnail_image;
use crate::launcher::view::{
    Body, Items, LauncherView, Outcome, ViewDescriptor, ViewEvent, ViewMeta, list_row_height,
};
use crate::launcher::views::theme::{apply_palette, palette_is_active};
use crate::state::AppState;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp"];

/// Thumbnail height in a card; the width follows the column.
const THUMB_HEIGHT: Pixels = px(100.);
/// Widest a card gets in the default window, for sizing the thumbnail.
const THUMB_WIDTH: Pixels = px(190.);

pub fn descriptor(config: &WallpaperConfig) -> ViewDescriptor {
    let config = config.clone();
    ViewDescriptor::new(
        ViewMeta {
            id: "wallpaper",
            prefix: config.prefix.clone().into(),
            name: "Wallpaper",
            icon: IconName::Image,
            description: "Browse and set wallpapers",
            default: false,
            in_help: true,
        },
        move |cx| WallpaperView::new(&config, cx),
    )
}

/// The theme generated from the last applied wallpaper. A global, so it
/// outlives the launcher window.
#[derive(Default)]
pub(crate) struct GeneratedTheme {
    state: Generation,
    /// Bumped per wallpaper, so a slower earlier run can't land last.
    id: u64,
}

#[derive(Default)]
enum Generation {
    #[default]
    Idle,
    Running(SharedString),
    Ready(SharedString, Box<WallpaperPalettes>),
    Failed(SharedString),
}

impl Global for GeneratedTheme {}

/// The wallpaper the last generated theme came from, with both its palettes.
pub(crate) fn generated_palettes(cx: &App) -> Option<(SharedString, WallpaperPalettes)> {
    match &cx.try_global::<GeneratedTheme>()?.state {
        Generation::Ready(name, palettes) => Some((name.clone(), **palettes)),
        _ => None,
    }
}

const GENERATED_PREFIX: &str = "Wallpaper: ";

/// The theme name `apply_palette` gets for a wallpaper's palette.
pub(crate) fn generated_theme_name(wallpaper: &str) -> String {
    format!("{GENERATED_PREFIX}{wallpaper}")
}

/// The wallpaper a theme name made by [`generated_theme_name`] came from.
pub(crate) fn wallpaper_of_theme_name(name: &str) -> Option<&str> {
    name.strip_prefix(GENERATED_PREFIX)
}

struct WallpaperEntry {
    path: PathBuf,
    name: String,
    extension: String,
    /// Part of the thumbnail key, so a file overwritten in place re-renders.
    mtime: Option<SystemTime>,
}

pub struct WallpaperView {
    directory: PathBuf,
    entries: Vec<WallpaperEntry>,
    scanning: bool,
    columns: usize,
    matugen: bool,
    matugen_scheme: String,
    /// Thumbnail loads notify this view, which redraws the launcher.
    owner: EntityId,
}

impl WallpaperView {
    fn new(config: &WallpaperConfig, cx: &mut Context<Self>) -> Self {
        let directory = expand_tilde(&config.directory);
        let scan_dir = directory.clone();
        cx.spawn(async move |this, cx| {
            let entries = cx
                .background_spawn(async move { scan_wallpapers(&scan_dir) })
                .await;
            this.update(cx, |this, cx| {
                this.entries = entries;
                this.scanning = false;
                cx.emit(ViewEvent::ItemsChanged);
            })
            .ok();
        })
        .detach();

        cx.default_global::<GeneratedTheme>();
        cx.observe_global::<GeneratedTheme>(|_, cx| cx.notify())
            .detach();

        Self {
            directory,
            entries: Vec::new(),
            scanning: true,
            columns: config.columns.max(1),
            matugen: config.matugen,
            matugen_scheme: config.matugen_scheme.clone(),
            owner: cx.entity_id(),
        }
    }

    fn apply(&self, path: PathBuf, name: SharedString, cx: &mut Context<Self>) {
        AppState::wallpaper(cx).dispatch(WallpaperCommand::SetWallpaper(path.clone()));
        if self.matugen {
            generate_theme(path, name, self.matugen_scheme.clone(), cx);
        }
    }
}

/// Run matugen off the UI thread; the result is offered, not applied.
fn generate_theme(path: PathBuf, name: SharedString, scheme: String, cx: &mut App) {
    let global = cx.global_mut::<GeneratedTheme>();
    global.id += 1;
    global.state = Generation::Running(name.clone());
    let id = global.id;

    cx.spawn(async move |cx| {
        let result = cx
            .background_spawn(async move { Base16Palette::generate_from_wallpaper(&path, &scheme) })
            .await;
        cx.update(|cx| {
            let global = cx.global_mut::<GeneratedTheme>();
            if global.id != id {
                return;
            }
            global.state = match result {
                Ok(palettes) => Generation::Ready(name, Box::new(palettes)),
                Err(err) => {
                    tracing::warn!("Failed to generate matugen theme: {err}");
                    Generation::Failed(err.to_string().into())
                }
            };
        });
    })
    .detach();
}

/// A clickable chip previewing one generated palette.
fn palette_chip(
    id: &'static str,
    label: &'static str,
    palette: Base16Palette,
    name: SharedString,
    cx: &App,
) -> impl IntoElement {
    let colors = cx.theme().colors();
    let active = palette_is_active(&palette, cx.theme());
    let swatch = |color: Hsla| div().size(px(10.)).rounded_full().bg(color);

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(Spacing::Small.pixels())
        .px(Spacing::Medium.pixels())
        .py(px(4.))
        .rounded(Radius::Small.pixels())
        .bg(palette.base00)
        .border_1()
        .border_color(if active { colors.accent } else { colors.border })
        .cursor_pointer()
        .hover(|s| s.border_color(colors.accent))
        .child(
            div().flex().gap(px(2.)).children(
                [
                    palette.base05,
                    palette.base08,
                    palette.base0b,
                    palette.base0d,
                    palette.base0e,
                ]
                .map(swatch),
            ),
        )
        .child(
            div()
                .text_size(TextSize::XSmall.rems())
                .text_color(palette.base05)
                .child(label),
        )
        .on_click(move |_, _, cx| {
            apply_palette(generated_theme_name(&name).into(), palette, cx);
        })
}

impl EventEmitter<ViewEvent> for WallpaperView {}

impl LauncherView for WallpaperView {
    fn items(&mut self, _query: &str, _cx: &mut Context<Self>) -> Items {
        Items::Ready(
            self.entries
                .iter()
                .map(|entry| {
                    Item::new(entry.path.to_string_lossy().to_string(), entry.name.clone())
                        .subtitle(entry.extension.clone())
                        .icon(ItemIcon::Thumbnail {
                            path: entry.path.clone(),
                            mtime: entry.mtime,
                        })
                        .action("apply", "Apply")
                })
                .collect(),
        )
    }

    fn activate(&mut self, item: &Item, _action: &str, cx: &mut Context<Self>) -> Outcome {
        self.apply(PathBuf::from(item.id.as_ref()), item.title.clone(), cx);
        Outcome::Stay
    }

    fn body(&mut self, _cx: &mut Context<Self>) -> Body {
        Body::Grid {
            columns: self.columns,
        }
    }

    fn row_height(&self, cx: &App) -> Pixels {
        THUMB_HEIGHT + list_row_height(cx, &[TextSize::Small], px(0.)) + Spacing::Medium.pixels()
    }

    fn render_row(
        &self,
        item: &Item,
        selected: bool,
        window: &Window,
        cx: &App,
    ) -> Option<AnyElement> {
        let ItemIcon::Thumbnail { path, mtime } = &item.icon else {
            return None;
        };
        let colors = cx.theme().colors();
        let image = thumbnail_image(
            path,
            *mtime,
            size(THUMB_WIDTH, THUMB_HEIGHT),
            self.owner,
            window,
            cx,
        );

        Some(
            div()
                .size_full()
                .p(Spacing::XSmall.pixels())
                .rounded(Radius::Medium.pixels())
                .border_1()
                .border_color(if selected {
                    colors.accent
                } else {
                    gpui::transparent_black()
                })
                .when(selected, |el| el.bg(colors.element_selected))
                .when(!selected, |el| el.hover(|s| s.bg(colors.element_hover)))
                .cursor_pointer()
                .flex()
                .flex_col()
                .gap(Spacing::XSmall.pixels())
                .child(
                    div()
                        .w_full()
                        .h(THUMB_HEIGHT)
                        .rounded(Radius::Small.pixels())
                        .overflow_hidden()
                        .bg(colors.element_background)
                        .children(
                            image.map(|image| img(image).size_full().object_fit(ObjectFit::Cover)),
                        ),
                )
                .child(
                    Label::new(item.title.clone())
                        .size(TextSize::Small)
                        .truncate(),
                )
                .into_any_element(),
        )
    }

    fn render_header(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.matugen {
            return None;
        }
        let colors = cx.theme().colors();
        let (status, status_color): (SharedString, Color) = match &cx
            .global::<GeneratedTheme>()
            .state
        {
            Generation::Idle => ("Apply a wallpaper to generate one".into(), Color::Muted),
            Generation::Running(name) => (format!("Generating from {name}…").into(), Color::Muted),
            Generation::Ready(name, _) => (format!("From {name}").into(), Color::Muted),
            Generation::Failed(err) => (err.clone(), Color::Error),
        };
        let chips = match &cx.global::<GeneratedTheme>().state {
            Generation::Ready(name, palettes) => Some(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.pixels())
                    .child(palette_chip(
                        "wallpaper-theme-dark",
                        "Dark",
                        palettes.dark,
                        name.clone(),
                        cx,
                    ))
                    .child(palette_chip(
                        "wallpaper-theme-light",
                        "Light",
                        palettes.light,
                        name.clone(),
                        cx,
                    )),
            ),
            _ => None,
        };

        Some(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(Spacing::Large.pixels())
                .px(Spacing::XLarge.pixels())
                .py(Spacing::Medium.pixels())
                .bg(colors.surface_background)
                .border_b_1()
                .border_color(colors.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(Spacing::Large.pixels())
                        .min_w(px(0.))
                        .child(
                            Icon::new(IconName::Palette)
                                .size(IconSize::Small)
                                .color(Color::Accent),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .min_w(px(0.))
                                .child(Label::new("Theme from wallpaper").size(TextSize::Small))
                                .child(
                                    Label::new(status)
                                        .size(TextSize::XSmall)
                                        .color(status_color)
                                        .truncate(),
                                ),
                        ),
                )
                .children(chips)
                .into_any_element(),
        )
    }

    fn empty_text(&self) -> Option<SharedString> {
        let dir = self.directory.display();
        Some(if self.scanning {
            format!("Scanning {dir}…").into()
        } else {
            format!("No images in {dir}").into()
        })
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

fn scan_wallpapers(dir: &Path) -> Vec<WallpaperEntry> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        tracing::warn!("Could not read wallpaper directory: {}", dir.display());
        return Vec::new();
    };

    let mut entries: Vec<WallpaperEntry> = read_dir
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?.to_lowercase();
            if !path.is_file() || !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_string();
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            Some(WallpaperEntry {
                path,
                name,
                extension: extension.to_uppercase(),
                mtime,
            })
        })
        .collect();

    entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
    entries
}
