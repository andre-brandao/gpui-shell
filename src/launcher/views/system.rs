use crate::launcher::view::{
    LauncherView, ViewAction, ViewContext, execute_action, render_list_item,
};
use gpui::{AnyElement, App, div, prelude::*, px};

pub struct SystemView;

struct SystemItem {
    id: &'static str,
    title: String,
    subtitle: String,
    icon: &'static str,
    action: ViewAction,
}

impl SystemView {
    fn get_items(&self, vx: &ViewContext, cx: &App) -> Vec<SystemItem> {
        let audio = vx.services.audio.read(cx);
        let network = vx.services.network.read(cx);

        let items = vec![
            SystemItem {
                id: "toggle-wifi",
                title: "Toggle WiFi".to_string(),
                subtitle: if network.wifi_enabled {
                    "Currently enabled".to_string()
                } else {
                    "Currently disabled".to_string()
                },
                icon: "󰤨",
                action: ViewAction::ToggleWifi,
            },
            SystemItem {
                id: "toggle-mute",
                title: "Toggle Mute".to_string(),
                subtitle: if audio.sink_muted {
                    "Currently muted".to_string()
                } else {
                    format!("Volume: {}%", audio.sink_volume)
                },
                icon: "󰝟",
                action: ViewAction::ToggleMute,
            },
            SystemItem {
                id: "volume-up",
                title: "Volume Up".to_string(),
                subtitle: "+5%".to_string(),
                icon: "󰕾",
                action: ViewAction::AdjustVolume(5),
            },
            SystemItem {
                id: "volume-down",
                title: "Volume Down".to_string(),
                subtitle: "-5%".to_string(),
                icon: "󰖀",
                action: ViewAction::AdjustVolume(-5),
            },
        ];

        let query_lower = vx.query.to_lowercase();
        items
            .into_iter()
            .filter(|item| {
                if vx.query.is_empty() {
                    return true;
                }
                item.title.to_lowercase().contains(&query_lower)
                    || item.subtitle.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

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

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let items = self.get_items(vx, cx);
        let count = items.len();
        let services = vx.services.clone();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(items.into_iter().enumerate().map(|(i, item)| {
                let action = item.action.clone();
                let services_clone = services.clone();

                render_list_item(
                    item.id,
                    item.icon,
                    &item.title,
                    Some(&item.subtitle),
                    i == vx.selected_index,
                    move |cx| {
                        execute_action(&action, &services_clone, cx);
                    },
                )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let items = self.get_items(vx, cx);

        if let Some(item) = items.get(index) {
            execute_action(&item.action, vx.services, cx);
            // Don't close for volume adjustments
            !matches!(item.action, ViewAction::AdjustVolume(_))
        } else {
            false
        }
    }
}
