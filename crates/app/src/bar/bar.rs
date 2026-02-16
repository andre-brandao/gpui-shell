//! Status bar using layer shell for Wayland.
//!
//! This module provides a configurable shell bar for any screen edge.

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, FontWeight, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px,
};
use ui::{ActiveTheme, font_size, spacing};

use super::widgets::{Widget, WidgetSlot};
use crate::config::{ActiveConfig, BarPosition};

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
            start_widgets: Widget::create_many(&config.start, WidgetSlot::Start, cx),
            center_widgets: Widget::create_many(&config.center, WidgetSlot::Center, cx),
            end_widgets: Widget::create_many(&config.end, WidgetSlot::End, cx),
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
                .gap(px(spacing::SM))
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
                .gap(px(spacing::SM))
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
        let is_vertical = self.position.is_vertical();

        let start_elements: Vec<AnyElement> =
            self.start_widgets.iter().map(|w| w.render()).collect();
        let center_elements: Vec<AnyElement> =
            self.center_widgets.iter().map(|w| w.render()).collect();
        let end_elements: Vec<AnyElement> = self.end_widgets.iter().map(|w| w.render()).collect();

        let root = div()
            .size_full()
            .flex()
            .text_size(px(font_size::SM))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.text.primary)
            .bg(theme.bg.primary)
            .border_color(theme.border.default);

        if is_vertical {
            root.flex_col()
                .items_center()
                .px(px(spacing::XS))
                .py(px(spacing::SM))
                .when(matches!(self.position, BarPosition::Left), |this| {
                    this.border_r_1()
                })
                .when(matches!(self.position, BarPosition::Right), |this| {
                    this.border_l_1()
                })
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
                .px(px(spacing::SM))
                .when(matches!(self.position, BarPosition::Top), |this| {
                    this.border_b_1()
                })
                .when(matches!(self.position, BarPosition::Bottom), |this| {
                    this.border_t_1()
                })
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
    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        tracing::info!("Bar opened");

        cx.update(|cx: &mut App| {
            let displays = cx.displays();

            if displays.is_empty() {
                // No displays enumerated yet, open on default display
                tracing::info!("No displays found, opening bar on default display");
                open_with_config(None, cx);
            } else {
                tracing::info!("Opening bar on {} displays", displays.len());
                for d in displays {
                    tracing::info!("Opening bar on display {:?}", d.id());
                    open_with_config(Some(d.id()), cx);
                }
            }
        })
    })
    .detach();
}

/// Open the bar with custom configuration.
pub fn open_with_config(display_id: Option<DisplayId>, cx: &mut App) {
    cx.open_window(window_options(display_id, cx), move |_, cx| {
        cx.new(Bar::new)
    })
    .unwrap();
}
