//! Services view - inspect and control the shell's background services.
//!
//! Two levels, both driven by the query:
//!
//! - `;s` (optionally with a filter) lists every service with its status,
//!   startup mode and retained state.
//! - `;s <service name>` opens that service: restart, stop/start, and switch
//!   it between eager, lazy and off.

pub mod config;

use std::time::Duration;

use gpui::{AnyElement, App, Context, EventEmitter, Window, div, prelude::*, px};
use services::{ServiceMode, ServiceStatus};
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, Label, LabelCommon, ListItem, ListItemSpacing,
    Spacing, TextSize, Toggleable,
};

use self::config::ServicesConfig;
use crate::config::State;
use crate::icons;
use crate::launcher::item::Item;
use crate::launcher::view::{
    Items, LauncherView, Matching, Outcome, ViewDescriptor, ViewEvent, ViewMeta,
};
use crate::state::{AppState, watch};

/// Icon for the services view itself, and for any service without a more
/// specific one.
const SERVICES_ICON: IconName = IconName::Gauge;

/// How often the resident-memory figure in the header is refreshed.
const RSS_INTERVAL: Duration = Duration::from_secs(1);

pub fn descriptor(config: &ServicesConfig) -> ViewDescriptor {
    let prefix = config.prefix.clone();
    ViewDescriptor::new(
        ViewMeta {
            id: "services",
            prefix: config.prefix.clone().into(),
            name: "Services",
            icon: SERVICES_ICON,
            description: "Inspect, restart and configure services",
            default: false,
            in_help: true,
        },
        move |cx| ServicesView::new(prefix.clone(), cx),
    )
}

/// Services status view.
pub struct ServicesView {
    prefix: String,
    /// Every service as of the last query; the header summarises all of
    /// them, including those the query hides.
    services: Vec<ServiceInfo>,
    /// Set when the query names one service exactly.
    detail: Option<(ServiceInfo, Vec<Action>)>,
    rss: Option<usize>,
}

/// Snapshot of one service.
#[derive(Clone)]
struct ServiceInfo {
    name: &'static str,
    icon: IconName,
    status: ServiceStatus,
    mode: ServiceMode,
    memory: usize,
    controllable: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Start,
    Stop,
    Restart,
    SetMode(ServiceMode),
}

impl Action {
    fn label(&self) -> String {
        match self {
            Action::Start => "Start service".to_string(),
            Action::Stop => "Stop service".to_string(),
            Action::Restart => "Restart service".to_string(),
            Action::SetMode(mode) => format!("Start mode: {}", mode.label()),
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Action::Start => "Bring the service up now",
            Action::Stop => "Tear down the background work",
            Action::Restart => "Stop the current run and start a fresh one",
            Action::SetMode(ServiceMode::Eager) => "Start with the shell",
            Action::SetMode(ServiceMode::Lazy) => "Start on first use",
            Action::SetMode(ServiceMode::Off) => "Never start",
        }
    }

    fn icon(&self) -> IconName {
        match self {
            Action::Start => IconName::Play,
            Action::Stop => IconName::Stop,
            Action::Restart => IconName::Refresh,
            Action::SetMode(_) => IconName::Settings,
        }
    }
}

impl ServicesView {
    fn new(prefix: String, cx: &mut Context<Self>) -> Self {
        // Keeps the view live while a service starts, errors or is stopped
        // from the view itself.
        let signals: Vec<_> = AppState::managed_services(cx)
            .into_iter()
            .map(|service| service.status_signal())
            .collect();
        for signal in signals {
            watch(cx, signal, |_, _, cx| cx.emit(ViewEvent::ItemsChanged));
        }

        cx.spawn(async move |this, cx| {
            loop {
                let rss = cx.background_spawn(async { process_rss_bytes() }).await;
                let alive = this.update(cx, |this, cx| {
                    this.rss = rss;
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
                cx.background_executor().timer(RSS_INTERVAL).await;
            }
        })
        .detach();

        Self {
            prefix,
            services: Vec::new(),
            detail: None,
            rss: None,
        }
    }

    fn snapshot(cx: &App) -> Vec<ServiceInfo> {
        AppState::managed_services(cx)
            .into_iter()
            .map(|service| ServiceInfo {
                name: service.name(),
                icon: icon_for(service.name()),
                status: service.status(),
                mode: service.mode(),
                memory: service.memory_bytes(),
                controllable: service.controllable(),
            })
            .collect()
    }

    fn actions_for(service: &ServiceInfo) -> Vec<Action> {
        if !service.controllable {
            return Vec::new();
        }

        let mut actions = if service.status.is_operational() {
            vec![Action::Restart, Action::Stop]
        } else {
            vec![Action::Start]
        };
        actions.extend(ServiceMode::ALL.map(Action::SetMode));
        actions
    }

    fn run(&self, action: Action, cx: &mut Context<Self>) {
        let Some((info, _)) = &self.detail else {
            return;
        };
        let name = info.name;
        let Some(service) = AppState::managed_services(cx)
            .into_iter()
            .find(|service| service.name() == name)
        else {
            return;
        };

        match action {
            Action::Start => service.start(),
            Action::Stop => service.stop(),
            Action::Restart => service.restart(),
            Action::SetMode(mode) => {
                service.lifecycle().set_mode(mode);
                // Turning a running service off stops it now; the other modes
                // only decide what happens at the next startup.
                if mode == ServiceMode::Off {
                    service.stop();
                }
                State::set_service_mode(name, mode, cx);
            }
        }
        // A mode change fires no status signal.
        cx.emit(ViewEvent::ItemsChanged);
    }
}

impl EventEmitter<ViewEvent> for ServicesView {}

impl LauncherView for ServicesView {
    fn matching(&self) -> Matching {
        if self.detail.is_some() {
            Matching::View
        } else {
            Matching::Shell
        }
    }

    fn items(&mut self, query: &str, cx: &mut Context<Self>) -> Items {
        let query = query.trim().to_lowercase();
        self.services = Self::snapshot(cx);

        // An exact name match opens that service rather than filtering to it.
        self.detail = self
            .services
            .iter()
            .find(|service| service.name.to_lowercase() == query)
            .map(|service| (service.clone(), Self::actions_for(service)));

        let items = match &self.detail {
            Some((_, actions)) => actions
                .iter()
                .enumerate()
                .map(|(ix, action)| {
                    Item::new(format!("action:{ix}"), action.label())
                        .subtitle(action.description())
                        .action("run", "Run")
                })
                .collect(),
            None => self
                .services
                .iter()
                .map(|service| {
                    Item::new(service.name, service.name)
                        .subtitle(service.status.label())
                        .keywords(service.mode.label())
                        .action("open", "Open")
                })
                .collect(),
        };
        Items::Ready(items)
    }

    fn activate(&mut self, item: &Item, _action: &str, cx: &mut Context<Self>) -> Outcome {
        match &self.detail {
            Some((_, actions)) => {
                let action = item
                    .id
                    .strip_prefix("action:")
                    .and_then(|ix| ix.parse::<usize>().ok())
                    .and_then(|ix| actions.get(ix).copied());
                if let Some(action) = action {
                    self.run(action, cx);
                }
                Outcome::Stay
            }
            None => Outcome::SetQuery(format!("{} {}", self.prefix, item.id.to_lowercase())),
        }
    }

    fn render_row(
        &self,
        item: &Item,
        selected: bool,
        _window: &Window,
        cx: &App,
    ) -> Option<AnyElement> {
        match &self.detail {
            Some((service, actions)) => {
                let ix: usize = item.id.strip_prefix("action:")?.parse().ok()?;
                Some(render_action_row(actions.get(ix)?, service, selected))
            }
            None => {
                let service = self.services.iter().find(|s| s.name == item.id.as_ref())?;
                Some(render_service_row(service, selected, cx))
            }
        }
    }

    fn render_header(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let theme = cx.theme();

        let (summary, color, section) = match &self.detail {
            Some((service, _)) => (
                format!(
                    "{} · {} · {} · {}",
                    service.name,
                    service.status.label(),
                    service.mode.label(),
                    format_bytes(service.memory)
                ),
                status_color(&service.status, cx),
                "ACTIONS",
            ),
            None => {
                let services = &self.services;
                let running = services
                    .iter()
                    .filter(|service| service.status.is_operational())
                    .count();
                let errors = services
                    .iter()
                    .filter(|service| matches!(service.status, ServiceStatus::Error(_)))
                    .count();
                let retained: usize = services.iter().map(|service| service.memory).sum();

                let mut summary = format!(
                    "{}/{} running · {} retained",
                    running,
                    services.len(),
                    format_bytes(retained)
                );
                if let Some(rss) = self.rss {
                    summary.push_str(&format!(" · {} resident", format_bytes(rss)));
                }

                let color = if errors > 0 {
                    theme.colors.status.error
                } else if running == services.len() {
                    theme.colors.status.success
                } else {
                    theme.colors.status.warning
                };

                (summary, color, "SERVICES")
            }
        };

        let error = self
            .detail
            .as_ref()
            .and_then(|(service, _)| service.status.error_message().map(String::from));

        Some(
            div()
                .flex()
                .flex_col()
                .gap(Spacing::Medium.pixels())
                .p(Spacing::Medium.pixels())
                .child(
                    div()
                        .px(Spacing::Large.pixels())
                        .py(Spacing::Medium.pixels())
                        .bg(theme.colors.surface_background)
                        .rounded(px(8.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(Spacing::Medium.pixels())
                                .child(
                                    Icon::new(SERVICES_ICON)
                                        .size(IconSize::Small)
                                        .color(Color::Custom(color)),
                                )
                                .child(
                                    div()
                                        .text_size(TextSize::Small.rems())
                                        .text_color(color)
                                        .child(summary),
                                ),
                        )
                        .children(error.map(|error| {
                            Label::new(error).size(TextSize::XSmall).color(Color::Error)
                        })),
                )
                .child(
                    div().px(Spacing::Medium.pixels()).child(
                        Label::new(section)
                            .size(TextSize::XSmall)
                            .color(Color::Disabled),
                    ),
                )
                .into_any_element(),
        )
    }
}

fn render_service_row(service: &ServiceInfo, selected: bool, cx: &App) -> AnyElement {
    ListItem::new(format!("service-{}", service.name))
        .spacing(ListItemSpacing::Sparse)
        .toggle_state(selected)
        .start_slot(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(
                    Icon::new(service.icon)
                        .size(IconSize::Medium)
                        .color(Color::Default),
                )
                .child(
                    Icon::new(icons::service_status_icon(&service.status))
                        .size(IconSize::XSmall)
                        .color(Color::Custom(status_color(&service.status, cx))),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(Label::new(service.name).size(TextSize::Default))
                .child(
                    Label::new(service.status.label())
                        .size(TextSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(service.mode.label())
                        .size(TextSize::XSmall)
                        .color(Color::Disabled),
                ),
        )
        .end_slot(
            Label::new(format_bytes(service.memory))
                .size(TextSize::Small)
                .color(Color::Muted),
        )
        .into_any_element()
}

fn render_action_row(action: &Action, service: &ServiceInfo, selected: bool) -> AnyElement {
    let current = *action == Action::SetMode(service.mode);

    ListItem::new(format!("service-action-{}", action.label()))
        .spacing(ListItemSpacing::Sparse)
        .toggle_state(selected)
        .start_slot(
            Icon::new(action.icon())
                .size(IconSize::Medium)
                .color(Color::Default),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(Label::new(action.label()).size(TextSize::Default))
                .child(
                    Label::new(action.description())
                        .size(TextSize::Small)
                        .color(Color::Muted),
                ),
        )
        .end_slot(if current {
            Label::new("current")
                .size(TextSize::XSmall)
                .color(Color::Accent)
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .into_any_element()
}

fn status_color(status: &ServiceStatus, cx: &App) -> gpui::Hsla {
    let theme = cx.theme();
    match status {
        ServiceStatus::Active => theme.colors.status.success,
        ServiceStatus::Initializing => theme.colors.status.info,
        ServiceStatus::Error(_) => theme.colors.status.error,
        ServiceStatus::Stopped => theme.colors.text_muted,
        ServiceStatus::Unavailable => theme.colors.text_disabled,
    }
}

fn icon_for(name: &str) -> IconName {
    match name {
        "Audio" => IconName::Volume,
        "Network" => IconName::Globe,
        "Bluetooth" => IconName::Bluetooth,
        "UPower" => IconName::BatteryFull,
        "MPRIS" => IconName::Music,
        "Notifications" => IconName::Bell,
        "Tray" => IconName::Menu,
        "Sysinfo" => IconName::Cpu,
        "Privacy" => IconName::Eye,
        "Wallpaper" => IconName::Image,
        "Brightness" => IconName::Sun,
        "Compositor" => IconName::Layout,
        _ => SERVICES_ICON,
    }
}

fn format_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let bytes = bytes as f64;

    if bytes >= KB * KB {
        format!("{:.1} MB", bytes / (KB * KB))
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}

/// Resident set size of the whole shell process.
///
/// Services share one heap, so this is the only true memory figure available;
/// per-service numbers are the state each one retains.
fn process_rss_bytes() -> Option<usize> {
    // ponytail: 4 KiB pages, true for every Linux target this shell runs on.
    // Read `sysconf(_SC_PAGESIZE)` if that ever stops holding.
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let resident: usize = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident * 4096)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_formatted_by_magnitude() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(3 * 1024 * 1024), "3.0 MB");
    }
}
