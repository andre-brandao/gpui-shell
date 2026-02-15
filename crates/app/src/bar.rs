//! Status bar using layer shell for Wayland.
//!
//! This module provides a configurable shell bar that supports horizontal
//! and vertical orientations.

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, FontWeight, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px, rems,
};
use services::Services;
use ui::{ActiveTheme, spacing};

use crate::{
    config::{BarConfig, BarOrientation, Config},
    widgets::Widget,
};

/// The main bar view.
struct Bar {
    orientation: BarOrientation,
    start_widgets: Vec<Widget>,
    center_widgets: Vec<Widget>,
    end_widgets: Vec<Widget>,
}

impl Bar {
    /// Create a bar with services and configuration.
    fn new(services: Services, config: BarConfig, cx: &mut Context<Self>) -> Self {
        let orientation = config.orientation;
        Self {
            orientation,
            start_widgets: Widget::create_many(&config.start, &services, cx),
            center_widgets: Widget::create_many(&config.center, &services, cx),
            end_widgets: Widget::create_many(&config.end, &services, cx),
        }
    }

    fn render_section(
        orientation: BarOrientation,
        gap: f32,
        children: Vec<AnyElement>,
    ) -> impl IntoElement {
        let section = div();

        if orientation.is_vertical() {
            section
                .flex()
                .flex_col()
                .items_center()
                .gap(px(gap))
                .children(children)
        } else {
            section
                .flex()
                .items_center()
                .gap(px(gap))
                .children(children)
        }
    }
}

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let start_elements: Vec<AnyElement> =
            self.start_widgets.iter().map(|w| w.render()).collect();
        let center_elements: Vec<AnyElement> =
            self.center_widgets.iter().map(|w| w.render()).collect();
        let end_elements: Vec<AnyElement> = self.end_widgets.iter().map(|w| w.render()).collect();

        let root = div()
            .size_full()
            .flex()
            .justify_between()
            .text_size(rems(0.67))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.text.primary)
            .bg(theme.bg.primary)
            .border_color(theme.border.default);

        if self.orientation.is_vertical() {
            root.flex_col()
                .items_center()
                .py(px(spacing::LG))
                .border_r_1()
                .child(Self::render_section(
                    self.orientation,
                    spacing::SM,
                    start_elements,
                ))
                .child(Self::render_section(
                    self.orientation,
                    spacing::SM,
                    center_elements,
                ))
                .child(Self::render_section(
                    self.orientation,
                    spacing::MD,
                    end_elements,
                ))
        } else {
            root.items_center()
                .px(px(spacing::LG))
                .border_b_1()
                .child(Self::render_section(
                    self.orientation,
                    spacing::SM,
                    start_elements,
                ))
                .child(Self::render_section(
                    self.orientation,
                    spacing::SM,
                    center_elements,
                ))
                .child(Self::render_section(
                    self.orientation,
                    spacing::MD,
                    end_elements,
                ))
        }
    }
}

/// Returns window options for the bar.
pub fn window_options(
    config: &BarConfig,
    display_id: Option<DisplayId>,
    cx: &App,
) -> WindowOptions {
    let display_size = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| Size::new(px(1920.), px(1080.)));

    let (window_size, anchor) = if config.orientation.is_vertical() {
        (
            Size::new(px(config.size), display_size.height),
            Anchor::LEFT | Anchor::TOP | Anchor::BOTTOM,
        )
    } else {
        (
            Size::new(display_size.width, px(config.size)),
            Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
        )
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
            exclusive_zone: Some(px(config.size)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the bar using the current global config.
pub fn open(services: Services, cx: &mut App) {
    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        tracing::info!("Bar opened");

        cx.update(|cx: &mut App| {
            let bar_config = Config::global(cx).bar.clone();
            let displays = cx.displays();

            if displays.is_empty() {
                // No displays enumerated yet, open on default display
                tracing::info!("No displays found, opening bar on default display");
                open_with_config(services.clone(), bar_config, None, cx);
            } else {
                tracing::info!("Opening bar on {} displays", displays.len());
                for d in displays {
                    tracing::info!("Opening bar on display {:?}", d.id());
                    open_with_config(services.clone(), bar_config.clone(), Some(d.id()), cx);
                }
            }
        })
    })
    .detach();
}

/// Open the bar with custom configuration.
pub fn open_with_config(
    services: Services,
    config: BarConfig,
    display_id: Option<DisplayId>,
    cx: &mut App,
) {
    cx.open_window(window_options(&config, display_id, cx), move |_, cx| {
        cx.new(|cx| Bar::new(services, config, cx))
    })
    .unwrap();
}
