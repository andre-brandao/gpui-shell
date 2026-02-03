//! System tray widget displaying StatusNotifierItem icons.

use crate::panel::{PanelConfig, toggle_panel};
use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{App, Context, MouseButton, Render, Window, div, layer_shell::Anchor, prelude::*, px};
use services::{MenuLayout, TrayCommand, TrayData, TrayIcon, TrayItem, TraySubscriber};
use ui::{ActiveTheme, icon_size, radius, spacing};

/// System tray widget that displays tray icons.
pub struct Tray {
    subscriber: TraySubscriber,
    data: TrayData,
}

impl Tray {
    /// Create a new system tray widget.
    pub fn new(subscriber: TraySubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();

        // Subscribe to tray data changes
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_data) = signal.next().await {
                    let result = this.update(cx, |this, cx| {
                        this.data = new_data;
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { subscriber, data }
    }

    /// Handle clicking on a tray item - opens a panel with the menu.
    fn on_item_click(&self, item: &TrayItem, cx: &mut App) {
        let Some(menu) = item.menu.clone() else {
            return;
        };

        let panel_id = format!("systray-{}", item.name);
        let subscriber = self.subscriber.clone();
        let item_name = item.name.clone();

        // Calculate menu height based on visible items
        let visible_items = count_visible_menu_items(&menu.2);
        let menu_height = (visible_items * 32).min(500) as f32 + 16.0;

        let config = PanelConfig {
            width: 250.0,
            height: menu_height,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0), // Compositor handles top margin
            namespace: "systray-menu".to_string(),
        };

        toggle_panel(&panel_id, config, cx, move |_cx| TrayMenuPanel {
            menu,
            item_name,
            subscriber,
        });
    }
}

impl Render for Tray {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let items: Vec<_> = self.data.items.clone();

        // Pre-compute colors for closures
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;

        div()
            .id("systray")
            .flex()
            .items_center()
            .gap(px(spacing::XS))
            .children(items.into_iter().map(|item| {
                let item_clone = item.clone();

                // Get the best icon representation - prefer icon name for nerd font rendering
                let icon_char = match &item.icon {
                    Some(TrayIcon::Name(name)) => get_icon_char(name, item.id.as_deref()),
                    Some(TrayIcon::Pixmap { .. }) => get_icon_char("", item.id.as_deref()),
                    None => get_icon_char("", item.id.as_deref()),
                };

                div()
                    .id(format!("tray-item-{}", item.name))
                    .p(px(spacing::XS))
                    .rounded(px(radius::SM))
                    .cursor_pointer()
                    .hover(move |s| s.bg(interactive_hover))
                    .active(move |s| s.bg(interactive_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.on_item_click(&item_clone, cx);
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(icon_size::MD))
                            .text_color(text_primary)
                            .child(icon_char),
                    )
            }))
    }
}

/// Count visible menu items for height calculation.
fn count_visible_menu_items(items: &[MenuLayout]) -> usize {
    items
        .iter()
        .filter(|MenuLayout(_, props, _)| props.visible != Some(false))
        .map(|MenuLayout(_, _, children)| 1 + count_visible_menu_items(children))
        .sum()
}

// ============================================================================
// Tray Menu Panel
// ============================================================================

/// Panel content for displaying a tray item's menu.
struct TrayMenuPanel {
    menu: MenuLayout,
    item_name: String,
    subscriber: TraySubscriber,
}

impl TrayMenuPanel {
    /// Handle clicking on a menu item.
    fn activate_menu_item(&self, menu_id: i32, window: &mut Window) {
        let subscriber = self.subscriber.clone();
        let item_name = self.item_name.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime");

            rt.block_on(async {
                if let Err(e) = subscriber
                    .dispatch(TrayCommand::MenuItemClicked { item_name, menu_id })
                    .await
                {
                    tracing::error!("Failed to click menu item: {}", e);
                }
            });
        });

        // Close the menu panel
        window.remove_window();
    }

    /// Render menu items recursively.
    fn render_menu_items(
        &self,
        items: &[MenuLayout],
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let theme = cx.theme();
        let mut elements = Vec::new();

        // Pre-compute colors for closures
        let border_default = theme.border.default;
        let interactive_hover = theme.interactive.hover;

        for layout in items {
            let MenuLayout(id, props, children) = layout;

            // Skip invisible items
            if props.visible == Some(false) {
                continue;
            }

            let label = props
                .label
                .as_ref()
                .map(|l| l.replace('_', ""))
                .unwrap_or_default();

            // Handle separator
            if label.is_empty() && children.is_empty() {
                elements.push(
                    div()
                        .h(px(1.))
                        .w_full()
                        .bg(border_default)
                        .my(px(spacing::XS))
                        .into_any_element(),
                );
                continue;
            }

            let menu_id = *id;
            let is_enabled = props.enabled.unwrap_or(true);
            let has_submenu = !children.is_empty();
            let indent = depth * 16;

            // Checkbox/radio state
            let toggle_indicator = props.toggle_type.as_ref().map(|toggle_type| {
                let is_checked = props.toggle_state == Some(1);
                match toggle_type.as_str() {
                    "checkmark" => {
                        if is_checked {
                            "󰄬 "
                        } else {
                            "  "
                        }
                    }
                    "radio" => {
                        if is_checked {
                            "󰄴 "
                        } else {
                            "󰄱 "
                        }
                    }
                    _ => "",
                }
            });

            elements.push(
                div()
                    .id(format!("menu-item-{}", menu_id))
                    .w_full()
                    .pl(px(spacing::MD + indent as f32))
                    .pr(px(spacing::MD))
                    .py(px(spacing::SM - 2.0))
                    .cursor_pointer()
                    .when(!is_enabled, |s| s.opacity(0.5))
                    .hover(move |s| {
                        if is_enabled {
                            s.bg(interactive_hover)
                        } else {
                            s
                        }
                    })
                    .when(is_enabled && !has_submenu, |el| {
                        el.on_click(cx.listener(move |this, _, window, _cx| {
                            this.activate_menu_item(menu_id, window);
                        }))
                    })
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .when_some(toggle_indicator, |this, indicator| {
                                        this.child(indicator)
                                    })
                                    .child(label),
                            )
                            .when(has_submenu, |el| el.child("▸")),
                    )
                    .into_any_element(),
            );

            // Render submenu items inline (expanded)
            if has_submenu {
                let submenu_elements = self.render_menu_items(children, depth + 1, cx);
                elements.extend(submenu_elements);
            }
        }

        elements
    }
}

impl Render for TrayMenuPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_items = self.render_menu_items(&self.menu.2, 0, cx);
        let theme = cx.theme();

        div()
            .id("systray-menu-panel")
            .size_full()
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(radius::LG))
            .py(px(spacing::SM))
            .text_color(theme.text.primary)
            .overflow_hidden()
            .children(menu_items)
    }
}

// ============================================================================
// Icon Mapping
// ============================================================================

/// Map common icon names or app IDs to nerd font characters.
fn get_icon_char(name: &str, app_id: Option<&str>) -> &'static str {
    // First try the icon name
    let icon = match name.to_lowercase().as_str() {
        "discord" => "󰙯",
        "spotify" => "󰓇",
        "steam" => "󰓓",
        "firefox" => "󰈹",
        "chrome" | "google-chrome" | "chromium" => "",
        "telegram" => "",
        "slack" => "󰒱",
        "thunderbird" => "󰴃",
        "vesktop" => "󰙯",
        "1password" => "󰢁",
        "bitwarden" => "󰞀",
        "dropbox" => "󰇣",
        "nextcloud" => "󰀸",
        "syncthing" => "󰓦",
        "nm-applet" | "network-manager" => "󰖩",
        "blueman" | "blueman-applet" => "󰂯",
        "pasystray" | "pavucontrol" => "󰕾",
        "udiskie" => "󰋊",
        "flameshot" => "󰹑",
        "kdeconnect" => "󰄜",
        "tailscale" => "󰖂",
        "remmina" | "org.remmina.remmina" | "org.remmina.remmina-status" | "remmina-icon" => "󰢹",
        "network" | "network-wireless" => "󰖩",
        "bluetooth" | "bluetooth-active" => "󰂯",
        "audio" | "audio-volume-high" => "󰕾",
        "battery" | "battery-full" => "󰁹",
        _ => "",
    };

    if !icon.is_empty() {
        return icon;
    }

    // Try the app id as fallback
    if let Some(id) = app_id {
        let id_lower = id.to_lowercase();

        // Handle generic systray_XXXX pattern (often used by Go apps like Tailscale)
        if id_lower.starts_with("systray_") {
            return "󰖂"; // Assume Tailscale for now
        }

        let icon = match id_lower.as_str() {
            "discord" | "vesktop" => "󰙯",
            "spotify" => "󰓇",
            "steam" => "󰓓",
            "firefox" => "󰈹",
            "chrome" | "google-chrome" | "chromium" => "",
            "telegram" | "telegram-desktop" => "",
            "slack" => "󰒱",
            "thunderbird" => "󰴃",
            "1password" => "󰢁",
            "bitwarden" => "󰞀",
            "dropbox" => "󰇣",
            "nextcloud" => "󰀸",
            "syncthing" | "syncthingtray" => "󰓦",
            "nm-applet" | "network-manager-applet" => "󰖩",
            "blueman" | "blueman-applet" | "blueman-tray" => "󰂯",
            "pasystray" | "pavucontrol" => "󰕾",
            "udiskie" => "󰋊",
            "flameshot" => "󰹑",
            "kdeconnect" | "kdeconnectd" => "󰄜",
            "tailscale" | "tailscale-systray" => "󰖂",
            "remmina" | "org.remmina.remmina" | "remmina-icon" => "󰢹",
            _ => "",
        };

        if !icon.is_empty() {
            return icon;
        }
    }

    // Fallback icon - log for easier icon mapping
    tracing::debug!("No icon mapping for name='{}' app_id={:?}", name, app_id);
    "󰀻"
}
