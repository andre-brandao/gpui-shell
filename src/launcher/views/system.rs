use crate::launcher::view::{LauncherView, ViewAction, ViewItem};
use crate::services::Services;
use gpui::App;

pub struct SystemView;

impl LauncherView for SystemView {
    fn prefix(&self) -> &'static str {
        "sys"
    }

    fn name(&self) -> &'static str {
        "System"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "System actions and settings"
    }

    fn items(&self, query: &str, services: &Services, cx: &App) -> Vec<ViewItem> {
        let audio = services.audio.read(cx);
        let network = services.network.read(cx);

        let mut items = vec![
            ViewItem::new("toggle-wifi", "Toggle WiFi", "󰤨")
                .with_subtitle(if network.wifi_enabled {
                    "Currently enabled"
                } else {
                    "Currently disabled"
                })
                .with_action(ViewAction::ToggleWifi),
            ViewItem::new("toggle-mute", "Toggle Mute", "󰝟")
                .with_subtitle(if audio.sink_muted {
                    "Currently muted".to_string()
                } else {
                    format!("Volume: {}%", audio.sink_volume)
                })
                .with_action(ViewAction::ToggleMute),
            ViewItem::new("volume-up", "Volume Up", "󰕾")
                .with_subtitle("+5%")
                .with_action(ViewAction::AdjustVolume(5)),
            ViewItem::new("volume-down", "Volume Down", "󰖀")
                .with_subtitle("-5%")
                .with_action(ViewAction::AdjustVolume(-5)),
        ];

        items.retain(|item| item.matches(query));
        items
    }
}
