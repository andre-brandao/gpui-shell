//! Status bar using layer shell for Wayland.
//!
//! This module provides a configurable shell bar for any screen edge.

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, layer_shell::*, point, prelude::*, px,
};
use ui::ActiveTheme;
use ui::patterns::BarSurface;

use super::config::BarPosition;
use super::modules::Widget;
use crate::config::{ActiveConfig, Config};

/// The main bar view.
struct Bar {
    position: BarPosition,
    start_widgets: Vec<Widget>,
    center_widgets: Vec<Widget>,
    end_widgets: Vec<Widget>,
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
}

impl Render for Bar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(cx.theme().font_size);
        let bar = &cx.config().bar;

        let start: Vec<AnyElement> = self.start_widgets.iter().map(|w| w.render()).collect();
        let center: Vec<AnyElement> = self.center_widgets.iter().map(|w| w.render()).collect();
        let end: Vec<AnyElement> = self.end_widgets.iter().map(|w| w.render()).collect();

        BarSurface::new(self.position.edge())
            .border(bar.show_border)
            .padding(px(bar.padding))
            .start(start)
            .center(center)
            .end(end)
    }
}

/// Returns window options for the bar.
pub fn window_options(
    // config: &BarConfig,
    display_id: Option<DisplayId>,
    cx: &App,
) -> WindowOptions {
    // The bar spans the screen on its long axis, so it needs the real
    // logical screen extent. `PlatformDisplay::bounds()` does not give it:
    // gpui derives those from the integer `wl_output.scale`, and Wayland's
    // core output protocol carries no fractional scale, so a screen at 1.5
    // advertises 2 and gpui reports two thirds of the true size - which
    // left the bar covering 75% of the screen.
    //
    // Requesting 0 on the spanned axis and letting the compositor stretch
    // between the anchors would be the protocol-blessed alternative, but
    // gpui sets a `wp_viewport` destination from these bounds before the
    // configure arrives, and a zero there is a protocol error.
    let display_size = crate::state::logical_display_size(display_id, cx)
        .or_else(|| {
            display_id
                .and_then(|id| cx.find_display(id))
                .or_else(|| cx.primary_display())
                .map(|display| display.bounds().size)
        })
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
        crate::state::record_window_display(window.window_handle(), display_id);
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
