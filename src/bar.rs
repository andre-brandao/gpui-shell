use gpui::{
    AnyElement, App, AppContext, Bounds, Context, FontWeight, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px, rems, rgba, white,
};

use crate::services::Services;
use crate::widgets::Widget;

pub const BAR_HEIGHT: f32 = 32.0;

/// Bar layout configuration.
#[derive(Clone)]
pub struct BarConfig {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

impl Default for BarConfig {
    fn default() -> Self {
        BarConfig {
            left: vec![
                "LauncherBtn".to_string(),
                "Workspaces".to_string(),
                "SysInfo".to_string(),
            ],
            center: vec!["Clock".to_string()],
            right: vec!["Systray".to_string(), "Info".to_string()],
        }
    }
}

struct LayerShellBar {
    left_widgets: Vec<Widget>,
    center_widgets: Vec<Widget>,
    right_widgets: Vec<Widget>,
}

impl LayerShellBar {
    /// Create a bar with services and configuration.
    fn new(services: Services, config: BarConfig, cx: &mut Context<Self>) -> Self {
        LayerShellBar {
            left_widgets: Widget::create_many(&config.left, &services, cx),
            center_widgets: Widget::create_many(&config.center, &services, cx),
            right_widgets: Widget::create_many(&config.right, &services, cx),
        }
    }
}

impl Render for LayerShellBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let left_elements: Vec<AnyElement> = self.left_widgets.iter().map(|w| w.render()).collect();
        let center_elements: Vec<AnyElement> =
            self.center_widgets.iter().map(|w| w.render()).collect();
        let right_elements: Vec<AnyElement> =
            self.right_widgets.iter().map(|w| w.render()).collect();

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(16.))
            .text_size(rems(0.67))
            .font_weight(FontWeight::MEDIUM)
            .text_color(white())
            .bg(rgba(0x1a1a1aff))
            // Left section
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .children(left_elements),
            )
            // Center section
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .children(center_elements),
            )
            // Right section
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .children(right_elements),
            )
    }
}

/// Returns the window options for the bar.
pub fn window_options() -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(1920.), px(BAR_HEIGHT)),
        })),
        app_id: Some("gpui-topbar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "topbar".to_string(),
            layer: Layer::Top,
            anchor: Anchor::LEFT | Anchor::RIGHT | Anchor::TOP,
            exclusive_zone: Some(px(BAR_HEIGHT)),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open the bar with default configuration.
pub fn open(services: Services, cx: &mut App) {
    open_with_config(services, BarConfig::default(), cx);
}

/// Open the bar with custom configuration.
pub fn open_with_config(services: Services, config: BarConfig, cx: &mut App) {
    cx.open_window(window_options(), move |_, cx| {
        cx.new(|cx| LayerShellBar::new(services, config, cx))
    })
    .unwrap();
}
