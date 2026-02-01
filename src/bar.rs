fn get_battery_percentage() -> Option<u8> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        let battery_path = "/sys/class/power_supply/BAT0/capacity";
        if let Ok(contents) = fs::read_to_string(battery_path) {
            if let Ok(percentage) = contents.trim().parse::<u8>() {
                return Some(percentage);
            }
        }
    }
    None
}
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    App, Application, Bounds, Context, FontWeight, Pixels, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    point, prelude::*, px, rems, rgba, white,
};

struct LayerShellBar;

impl LayerShellBar {
    fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                let _ = this.update(cx, |_, cx| cx.notify());
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
            }
        })
        .detach();

        LayerShellBar
    }
}

impl Render for LayerShellBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let hours = (now / 3600) % 24;
        let minutes = (now / 60) % 60;
        let seconds = now % 60;

        let bat = get_battery_percentage().map_or("N/A".to_string(), |p| format!("{}%", p));

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
            .child("TopBar")
            .child(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
            .child(format!("Battery: {}", bat))
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
