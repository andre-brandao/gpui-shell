//! Services status view - shows health status of all services.

pub mod config;

use gpui::{AnyElement, App, div, prelude::*, px};
use services::ServiceStatus;
use ui::{
    ActiveTheme, Color, Label, LabelCommon, LabelSize, ListItem, ListItemSpacing, icon_size,
    spacing,
};

use self::config::ServicesConfig;
use crate::launcher::view::{LauncherView, ViewContext};
use crate::state::AppState;

/// Services status view - shows health of all system services.
pub struct ServicesView {
    prefix: String,
}

#[derive(Clone)]
struct ServiceInfo {
    name: &'static str,
    icon: &'static str,
    status: ServiceStatus,
}

impl ServicesView {
    pub fn new(config: &ServicesConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
        }
    }

    /// Build the full list of services to display.
    ///
    /// NOTE: This must be kept in sync with `AppState` — there is no compile-time
    /// guarantee that all services are represented here.
    fn get_services(&self, cx: &App) -> Vec<ServiceInfo> {
        vec![
            ServiceInfo {
                name: "Audio",
                icon: "󰕾",
                status: AppState::audio(cx).status(),
            },
            ServiceInfo {
                name: "Network",
                icon: "󰖟",
                status: AppState::network(cx).status(),
            },
            ServiceInfo {
                name: "Bluetooth",
                icon: "󰂯",
                status: AppState::bluetooth(cx).status(),
            },
            ServiceInfo {
                name: "UPower",
                icon: "󰁹",
                status: AppState::upower(cx).status(),
            },
            ServiceInfo {
                name: "MPRIS",
                icon: "󰝚",
                status: AppState::mpris(cx).status(),
            },
            ServiceInfo {
                name: "Notifications",
                icon: "󰂚",
                status: AppState::notification(cx).status(),
            },
            ServiceInfo {
                name: "Tray",
                icon: "󰍜",
                status: AppState::tray(cx).status(),
            },
            ServiceInfo {
                name: "Sysinfo",
                icon: "󰻠",
                status: AppState::sysinfo(cx).status(),
            },
            ServiceInfo {
                name: "Privacy",
                icon: "󰒃",
                status: AppState::privacy(cx).status(),
            },
            ServiceInfo {
                name: "Wallpaper",
                icon: "󰸉",
                status: AppState::wallpaper(cx).status(),
            },
            ServiceInfo {
                name: "Brightness",
                icon: "󰃟",
                status: AppState::brightness(cx).status(),
            },
        ]
    }

    fn filtered_services(&self, query: &str, cx: &App) -> Vec<ServiceInfo> {
        let query_lower = query.to_lowercase();
        self.get_services(cx)
            .into_iter()
            .filter(|service| {
                if query.is_empty() {
                    return true;
                }
                service.name.to_lowercase().contains(&query_lower)
                    || service.status.label().to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

impl LauncherView for ServicesView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Services"
    }

    fn icon(&self) -> &'static str {
        "󰓅"
    }

    fn description(&self) -> &'static str {
        "View service health status"
    }

    fn match_count(&self, vx: &ViewContext, cx: &App) -> usize {
        self.filtered_services(vx.query, cx).len()
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let services = self.filtered_services(vx.query, cx);
        let Some(service) = services.get(index) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let status_color = match &service.status {
            ServiceStatus::Active => theme.status.success,
            ServiceStatus::Initializing => theme.status.info,
            ServiceStatus::Error(_) => theme.status.error,
            ServiceStatus::Unavailable => theme.text.disabled,
        };

        let mut item = ListItem::new(format!("service-{}", service.name))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::SM))
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(theme.text.primary)
                            .child(service.icon),
                    )
                    .child(
                        div()
                            .text_size(px(icon_size::SM))
                            .text_color(status_color)
                            .child(service.status.icon()),
                    ),
            )
            .child(
                div().flex().flex_col().gap(px(1.)).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(spacing::SM))
                        .child(Label::new(service.name).size(LabelSize::Default))
                        .child(
                            Label::new(service.status.label())
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                ),
            );

        // Add error message if present
        if let Some(error_msg) = service.status.error_message() {
            item = item.child(
                Label::new(format!("Error: {}", error_msg))
                    .size(LabelSize::Small)
                    .color(Color::Error),
            );
        }

        item.into_any_element()
    }

    fn render_header(&self, _vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        // Always uses the full unfiltered list so the summary reflects overall health,
        // not just what's visible for the current query.
        let services = self.get_services(cx);
        let active = services
            .iter()
            .filter(|s| s.status == ServiceStatus::Active)
            .count();
        let total = services.len();
        let errors = services
            .iter()
            .filter(|s| matches!(s.status, ServiceStatus::Error(_)))
            .count();

        let theme = cx.theme();
        let status_text = if errors > 0 {
            format!("{} services active, {} with errors", active, errors)
        } else {
            format!("{}/{} services active", active, total)
        };

        let status_color = if errors > 0 {
            theme.status.error
        } else if active == total {
            theme.status.success
        } else {
            theme.status.warning
        };

        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(spacing::SM))
                .p(px(spacing::SM))
                .child(
                    div()
                        .px(px(spacing::MD))
                        .py(px(spacing::SM))
                        .bg(theme.bg.secondary)
                        .rounded(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(spacing::SM))
                        .child(
                            div()
                                .text_size(px(icon_size::MD))
                                .text_color(status_color)
                                .child("󰓅"),
                        )
                        .child(
                            div()
                                .text_size(theme.font_sizes.sm)
                                .text_color(status_color)
                                .child(status_text),
                        ),
                )
                .child(
                    div().px(px(spacing::SM)).child(
                        Label::new("SERVICES")
                            .size(LabelSize::XSmall)
                            .color(Color::Disabled),
                    ),
                )
                .into_any_element(),
        )
    }

    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        // Services are read-only, just keep launcher open
        false
    }
}
