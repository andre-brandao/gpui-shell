use gpui::{
    App, AppContext, Bounds, Context, Entity, FontWeight, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems,
    rgba, white,
};

use crate::services::Services;
use crate::widgets::{Clock, Info, Systray, Workspaces};

pub const BAR_HEIGHT: f32 = 32.0;

struct LayerShellBar {
    workspaces: Entity<Workspaces>,
    clock: Entity<Clock>,
    systray: Entity<Systray>,
    info: Entity<Info>,
}

impl LayerShellBar {
    /// Create a bar with all services.
    fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        LayerShellBar {
            workspaces: cx.new(|cx| Workspaces::with_services(services.clone(), cx)),
            clock: cx.new(Clock::new),
            systray: cx.new(Systray::new),
            info: cx.new(|cx| Info::with_services(services, cx)),
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
                    .child(self.info.clone()),
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

pub fn open(services: Services, cx: &mut App) {
    cx.open_window(window_options(), move |_, cx| {
        cx.new(|cx| LayerShellBar::with_services(services, cx))
    })
    .unwrap();
}
