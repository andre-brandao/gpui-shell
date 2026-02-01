use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    App, Application, Bounds, Context, FontWeight, Size, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, layer_shell::*, point, prelude::*, px, rems,
    rgba, white,
};

use crate::widgets::{self, WorkspaceInfo};

struct LayerShellBar {
    workspaces: Vec<WorkspaceInfo>,
}

impl LayerShellBar {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                let workspaces = widgets::fetch_workspaces();
                let _ = this.update(cx, |this, cx| {
                    this.workspaces = workspaces;
                    cx.notify();
                });
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
            }
        })
        .detach();

        LayerShellBar {
            workspaces: widgets::fetch_workspaces(),
        }
    }

    fn get_time(&self) -> (u64, u64, u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;

        (hours, minutes, seconds)
    }
}

impl Render for LayerShellBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (hours, minutes, seconds) = self.get_time();
        let battery = widgets::get_battery_percentage();

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
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(widgets::workspaces(&self.workspaces)),
            )
            // Center section
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(widgets::clock(hours, minutes, seconds)),
            )
            // End section
            .child(div().flex().items_center().child(widgets::battery(battery)))
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
