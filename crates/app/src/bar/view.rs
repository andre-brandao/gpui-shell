//! Status bar using layer shell for Wayland.
//!
//! This module provides a configurable shell bar for any screen edge.

use std::collections::HashMap;
use std::sync::Mutex;

use gpui::{
    AnyElement, AnyWindowHandle, App, Bounds, Context, DisplayId, FontWeight, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px,
};
use ui::{ActiveTheme, spacing};

use super::config::BarPosition;
use super::modules::{Widget, style};
use crate::config::{ActiveConfig, Config};

/// Maps each bar window to the display it was explicitly opened on.
///
/// We can't correlate a window to a compositor monitor by position: gpui's
/// `Display::bounds()` doesn't reflect real global monitor placement in this
/// fork (every display reports origin (0, 0) regardless of its actual
/// position), and `Window::display()` is separately unreliable for
/// layer-shell windows (gpui_linux derives it from `primary_output_scale()`,
/// which just picks whichever output has the highest scale factor). We also
/// can't assume `cx.displays()` and the compositor's own monitor list
/// enumerate outputs in the same order - on at least one real dual-monitor
/// setup they didn't, which silently swapped which bar controlled which
/// monitor. So instead we record the specific `DisplayId` each window was
/// opened with, and match it against a compositor monitor by name via
/// `PlatformDisplay::uuid()` (see `Workspaces::current_monitor_name`), which
/// gpui_linux derives deterministically from the Wayland output's name.
static WINDOW_DISPLAYS: Mutex<Option<HashMap<AnyWindowHandle, DisplayId>>> = Mutex::new(None);

/// Look up the display a bar window was opened on.
pub fn display_id_for_window(window: &Window) -> Option<DisplayId> {
    WINDOW_DISPLAYS
        .lock()
        .ok()?
        .as_ref()?
        .get(&window.window_handle())
        .copied()
}

fn record_window_display(handle: AnyWindowHandle, display_id: Option<DisplayId>) {
    let Some(display_id) = display_id else {
        return;
    };
    let mut guard = WINDOW_DISPLAYS.lock().unwrap();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(handle, display_id);
}

/// The main bar view.
struct Bar {
    position: BarPosition,
    start_widgets: Vec<Widget>,
    center_widgets: Vec<Widget>,
    end_widgets: Vec<Widget>,
}

#[derive(Clone, Copy)]
enum SectionAlign {
    Start,
    Center,
    End,
}

impl Bar {
    /// Create a bar with configuration.
    fn new(cx: &mut Context<Self>) -> Self {
        let config = cx.config().bar.clone();
        let position = config.position;
        Self {
            position,
            start_widgets: Widget::create_many(&config.start, cx),
            center_widgets: Widget::create_many(&config.center, cx),
            end_widgets: Widget::create_many(&config.end, cx),
        }
    }

    fn render_section(
        is_vertical: bool,
        align: SectionAlign,
        children: Vec<AnyElement>,
    ) -> impl IntoElement {
        let section = div();

        if is_vertical {
            section
                .flex()
                .w_full()
                .flex_col()
                .items_center()
                .gap(px(style::BAR_SECTION_GAP))
                .when(matches!(align, SectionAlign::Center), |this| {
                    this.flex_1().justify_center()
                })
                .when(matches!(align, SectionAlign::Start), |this| {
                    this.justify_start()
                })
                .when(matches!(align, SectionAlign::End), |this| {
                    this.justify_end()
                })
                .children(children)
        } else {
            section
                .flex()
                .h_full()
                .items_center()
                .gap(px(style::BAR_SECTION_GAP))
                .when(matches!(align, SectionAlign::Start), |this| {
                    this.flex_1().justify_start()
                })
                .when(matches!(align, SectionAlign::Center), |this| {
                    this.flex_1().justify_center().overflow_hidden()
                })
                .when(matches!(align, SectionAlign::End), |this| {
                    this.flex_1().justify_end()
                })
                .children(children)
        }
    }
}

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let bar = &cx.config().bar;
        let is_vertical = self.position.is_vertical();

        let start_elements: Vec<AnyElement> =
            self.start_widgets.iter().map(|w| w.render()).collect();
        let center_elements: Vec<AnyElement> =
            self.center_widgets.iter().map(|w| w.render()).collect();
        let end_elements: Vec<AnyElement> = self.end_widgets.iter().map(|w| w.render()).collect();

        let root = div()
            .size_full()
            .flex()
            .text_size(theme.font_sizes.sm)
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.text.primary)
            .bg(theme.bg.primary)
            .border_color(theme.border.default);

        if is_vertical {
            root.flex_col()
                .items_center()
                .px(px(1.0))
                .py(px(spacing::SM))
                .when(
                    bar.show_border && matches!(self.position, BarPosition::Left),
                    |this| this.border_r_1(),
                )
                .when(
                    bar.show_border && matches!(self.position, BarPosition::Right),
                    |this| this.border_l_1(),
                )
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::Start,
                    start_elements,
                ))
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::Center,
                    center_elements,
                ))
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::End,
                    end_elements,
                ))
        } else {
            root.items_center()
                .px(px(bar.padding))
                .when(
                    bar.show_border && matches!(self.position, BarPosition::Top),
                    |this| this.border_b_1(),
                )
                .when(
                    bar.show_border && matches!(self.position, BarPosition::Bottom),
                    |this| this.border_t_1(),
                )
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::Start,
                    start_elements,
                ))
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::Center,
                    center_elements,
                ))
                .child(Self::render_section(
                    is_vertical,
                    SectionAlign::End,
                    end_elements,
                ))
        }
    }
}

/// Returns window options for the bar.
pub fn window_options(
    // config: &BarConfig,
    display_id: Option<DisplayId>,
    cx: &App,
) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));
    let config = cx.config();
    let (window_size, anchor) = match config.bar.position {
        BarPosition::Left => (
            Size::new(px(config.bar.size), display_size.height),
            Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
        ),
        BarPosition::Right => (
            Size::new(px(config.bar.size), display_size.height),
            Anchor::RIGHT | Anchor::TOP | Anchor::BOTTOM,
        ),
        BarPosition::Top => (
            Size::new(display_size.width, px(config.bar.size)),
            Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
        ),
        BarPosition::Bottom => (
            Size::new(display_size.width, px(config.bar.size)),
            Anchor::LEFT | Anchor::RIGHT | Anchor::BOTTOM,
        ),
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: window_size,
        })),
        app_id: Some("gpuishell-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
            layer: Layer::Top,
            anchor,
            exclusive_zone: Some(px(config.bar.size)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Initialize the bar using the current global config.
pub fn init(cx: &mut App) {
    cx.observe_global::<Config>(|cx| {
        tracing::info!("Config changed; reloading bar windows");
        reload(cx);
    })
    .detach();

    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        tracing::info!("Bar opened");

        cx.update(open_all_bar_windows)
    })
    .detach();
}

/// Open the bar with custom configuration.
pub fn open_with_config(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |window, cx| {
        record_window_display(window.window_handle(), display_id);
        cx.new(Bar::new)
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open bar window: {}", err);
            false
        }
    }
}

fn open_all_bar_windows(cx: &mut App) -> usize {
    let mut opened = 0usize;
    let displays = cx.displays();
    if displays.is_empty() {
        tracing::info!("No displays found, opening bar on default display");
        if open_with_config(None, cx) {
            opened += 1;
        }
    } else {
        tracing::info!("Opening bar on {} displays", displays.len());
        for d in displays {
            tracing::info!("Opening bar on display {:?}", d.id());
            if open_with_config(Some(d.id()), cx) {
                opened += 1;
            }
        }
    }
    opened
}

/// Rebuild all bar windows using the latest config.
pub fn reload(cx: &mut App) {
    let old_windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter(|h| h.downcast::<Bar>().is_some())
        .collect();

    let opened = open_all_bar_windows(cx);
    if opened == 0 {
        tracing::warn!("Bar reload skipped closing old windows because no new bar window opened");
        return;
    }

    for handle in old_windows {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}
