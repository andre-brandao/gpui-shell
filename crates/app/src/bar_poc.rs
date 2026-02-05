//! Bar2 - Proof of concept using Zed's UI module.
//!
//! This demonstrates using Zed UI components (Button, Label, etc.) with
//! proper theme initialization.

use chrono::Local;
use gpui::{
    App, Bounds, Context, DisplayId, Entity, FontWeight, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems,
};
use std::time::Duration;
use ui::{
    ActiveTheme, Button, ButtonStyle, Color, IconName, IconSize, Label, LabelCommon, LabelSize,
    Tooltip, prelude::*,
};

/// Height of the bar in pixels.
pub const BAR_HEIGHT: f32 = 32.0;

/// Simple clock widget using Zed UI components.
struct Clock2 {
    time_str: String,
}

impl Clock2 {
    fn new(cx: &mut Context<Self>) -> Self {
        // Spawn a timer to update the clock every second
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |this, cx| {
                    this.time_str = Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
                    cx.notify();
                });
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        })
        .detach();

        Self {
            time_str: Local::now().format("%d/%m/%Y %H:%M:%S").to_string(),
        }
    }
}

impl Render for Clock2 {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Using Zed's Label component
        Label::new(self.time_str.clone())
            .size(LabelSize::Small)
            .color(Color::Default)
    }
}

/// The main bar2 view - using Zed UI components.
struct Bar2 {
    clock: Entity<Clock2>,
}

impl Bar2 {
    fn new(cx: &mut Context<Self>) -> Self {
        let clock = cx.new(Clock2::new);
        Self { clock }
    }
}

impl Render for Bar2 {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .text_size(rems(0.67))
            .font_weight(FontWeight::MEDIUM)
            .text_color(colors.text)
            .bg(colors.background)
            .border_b_1()
            .border_color(colors.border)
            // Left section - using Zed's Button component
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("launcher", "Apps")
                            .style(ButtonStyle::Subtle)
                            .icon(IconName::Menu)
                            .icon_size(IconSize::Small)
                            .tooltip(Tooltip::text("Open Launcher"))
                            .on_click(|_, _window, _cx| {
                                tracing::info!("Launcher button clicked!");
                            }),
                    )
                    .child(
                        Label::new("GPUi Shell")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            // Center section
            .child(
                div().flex().items_center().child(
                    Label::new("Center")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
            // Right section - clock using Zed UI
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.clock.clone()),
            )
    }
}

/// Returns the window options for bar2.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let width = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display())
        .map(|display| display.bounds().size.width)
        .unwrap_or_else(|| px(1920.));

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(width, px(BAR_HEIGHT)),
        })),
        app_id: Some("gpuishell-bar2".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "bar2".to_string(),
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

/// Open bar2 on all displays.
pub fn open(cx: &mut App) {
    cx.spawn(async move |cx| {
        // Small delay to allow Wayland to enumerate displays
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;

        tracing::info!("Bar2 (PoC) opened");

        cx.update(|cx: &mut App| {
            let displays = cx.displays();
            if displays.is_empty() {
                tracing::info!("No displays found, opening bar2 on default display");
                open_on_display(None, cx);
            } else {
                tracing::info!("Opening bar2 on {} displays", displays.len());
                for d in displays {
                    tracing::info!("Opening bar2 on display {:?}", d.id());
                    open_on_display(Some(d.id()), cx);
                }
            }
        })
    })
    .detach();
}

/// Open bar2 on a specific display.
fn open_on_display(display_id: Option<DisplayId>, cx: &mut App) {
    cx.open_window(window_options(display_id, cx), move |_, cx| {
        cx.new(Bar2::new)
    })
    .unwrap();
}
