use gpui::{
    App, Application, Bounds, Context, Entity, FontWeight, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px, rems, rgba, white,
};

use crate::widgets::{Battery, Clock, HyprlandWorkspaces};

struct LayerShellBar {
    workspaces: Entity<HyprlandWorkspaces>,
    clock: Entity<Clock>,
    battery: Entity<Battery>,
}

impl LayerShellBar {
    fn new(cx: &mut Context<Self>) -> Self {
        LayerShellBar {
            workspaces: cx.new(HyprlandWorkspaces::new),
            clock: cx.new(Clock::new),
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
            .text_size(rems(0.875))
            .font_weight(FontWeight::MEDIUM)
            .text_color(white())
            .bg(rgba(0x1a1a1aff))
            // Start section
            .child(div().flex().items_center().child(self.workspaces.clone()))
            // Center section
            .child(div().flex().items_center().child(self.clock.clone()))
            // End section
            .child(div().flex().items_center().child(self.battery.clone()))
    }
}

pub fn init() {
    const BAR_HEIGHT: f32 = 32.0;

    Application::new().run(|cx: &mut App| {
        cx.open_window(
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
            },
            |_, cx| cx.new(LayerShellBar::new),
        )
        .unwrap();
    });
}
