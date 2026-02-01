use gpui::{
    App, AppContext, Bounds, Context, Entity, FontWeight, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems,
    rgba, white,
};

use crate::services::compositor::Compositor;
use crate::widgets::{Battery, Clock, Systray, Workspaces};

pub const BAR_HEIGHT: f32 = 32.0;

struct LayerShellBar {
    workspaces: Entity<Workspaces>,
    clock: Entity<Clock>,
    systray: Entity<Systray>,
    battery: Entity<Battery>,
}

impl LayerShellBar {
    fn new(cx: &mut Context<Self>) -> Self {
        LayerShellBar {
            workspaces: cx.new(Workspaces::new),
            clock: cx.new(Clock::new),
            systray: cx.new(Systray::new),
            battery: cx.new(Battery::new),
        }
    }

    /// Create a bar with a shared compositor entity.
    /// Use this when opening multiple bars (e.g., one per monitor).
    fn with_compositor(compositor: Entity<Compositor>, cx: &mut Context<Self>) -> Self {
        LayerShellBar {
            workspaces: cx.new(|cx| Workspaces::with_compositor(compositor, cx)),
            clock: cx.new(Clock::new),
            systray: cx.new(Systray::new),
            battery: cx.new(Battery::new),
        }
    }
}

impl Render for LayerShellBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
            // Start section
            .child(div().flex().items_center().child(self.workspaces.clone()))
            // Center section
            .child(div().flex().items_center().child(self.clock.clone()))
            // End section
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(self.systray.clone())
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

/// Opens the bar window (creates its own compositor entity).
pub fn open(cx: &mut App) {
    cx.open_window(window_options(), |_, cx| cx.new(LayerShellBar::new))
        .unwrap();
}

/// Opens a bar window with a shared compositor entity.
/// Use this when you want multiple windows to share compositor state.
///
/// Example for multi-monitor setup:
/// ```ignore
/// let compositor = cx.new(Compositor::new);
/// bar::open_with_compositor(compositor.clone(), cx);
/// bar::open_with_compositor(compositor.clone(), cx);
/// ```
///
/// Note: GPUI's LayerShellOptions doesn't currently support targeting
/// specific outputs. The compositor will place each window on the
/// focused output when created.
pub fn open_with_compositor(compositor: Entity<Compositor>, cx: &mut App) {
    cx.open_window(window_options(), move |_, cx| {
        cx.new(|cx| LayerShellBar::with_compositor(compositor, cx))
    })
    .unwrap();
}
