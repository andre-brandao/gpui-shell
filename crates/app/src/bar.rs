//! Status bar using layer shell for Wayland.
//!
//! This module provides a top bar that displays widgets like clock and battery
//! status, anchored to the top of the screen using the layer shell protocol.

use gpui::{
    App, Bounds, Context, Entity, FontWeight, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems,
};
use services::UPowerSubscriber;
use ui::{bg, border, spacing, text};

use crate::widgets::{Battery, Clock};

/// Height of the bar in pixels.
pub const BAR_HEIGHT: f32 = 32.0;

/// Shared services for widgets.
#[derive(Clone)]
pub struct Services {
    pub upower: UPowerSubscriber,
}

impl Services {
    /// Create new services. Must be called from an async context.
    pub async fn new() -> anyhow::Result<Self> {
        let upower = UPowerSubscriber::new().await?;
        Ok(Self { upower })
    }
}

/// The main bar view.
struct Bar {
    clock: Entity<Clock>,
    battery: Entity<Battery>,
}

impl Bar {
    /// Create a new bar with the given services.
    fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let clock = cx.new(Clock::new);
        let battery = cx.new(|cx| Battery::new(services.upower.clone(), cx));

        Bar { clock, battery }
    }
}

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(spacing::LG))
            .text_size(rems(0.67))
            .font_weight(FontWeight::MEDIUM)
            .text_color(text::primary())
            .bg(bg::primary())
            .border_b_1()
            .border_color(border::default())
            // Left section (placeholder for future widgets)
            .child(
                div().flex().items_center().gap(px(spacing::SM)), // Add left widgets here
            )
            // Center section - Clock
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SM))
                    .child(self.clock.clone()),
            )
            // Right section - Battery
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    .child(self.battery.clone()),
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
        app_id: Some("gpuishell-bar".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar".to_string(),
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

/// Open the bar window with the given services.
pub fn open(services: Services, cx: &mut App) {
    cx.open_window(window_options(), move |_, cx| {
        cx.new(|cx| Bar::new(services, cx))
    })
    .unwrap();
}
