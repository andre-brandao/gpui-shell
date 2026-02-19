//! System tray widget displaying StatusNotifierItem icons.

mod config;
pub use config::TrayConfig;

use crate::panel::{PanelConfig, toggle_panel};
use gpui::{
    App, Context, ElementId, MouseButton, Render, SharedString, Window, div, prelude::*, px,
};
use services::{MenuLayout, MenuLayoutProps, TrayCommand, TrayData, TrayIcon, TrayItem};
use ui::{ActiveTheme, font_size, radius, spacing};

use super::style;
use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::panel_placement;
use crate::state::AppState;
use crate::state::watch;

/// System tray widget that displays tray icons.
pub struct Tray {
    slot: WidgetSlot,
    subscriber: services::TraySubscriber,
    data: TrayData,
}

impl Tray {
    /// Create a new system tray widget.
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::tray(cx).clone();
        let data = subscriber.get();

        // Subscribe to tray data changes
        watch(cx, subscriber.subscribe(), |this, new_data, cx| {
            this.data = new_data;
            cx.notify();
        });

        Self {
            slot,
            subscriber,
            data,
        }
    }

    /// Handle left-clicking on a tray item.
    /// Opens menu panel if the item has a menu, otherwise calls Activate.
    fn on_item_click(&self, item: &TrayItem, cx: &mut App) {
        if let Some(menu) = item.menu.clone() {
            let panel_id = format!("systray-{}", item.name);
            let subscriber = self.subscriber.clone();
            let item_name = item.name.clone();
            let config = Config::global(cx);
            let (anchor, margin) = panel_placement(config.bar.position, self.slot);

            let config = PanelConfig {
                width: 250.0,
                height: 400.0,
                anchor,
                margin,
                namespace: "systray-menu".to_string(),
            };

            toggle_panel(&panel_id, config, cx, move |cx| {
                TrayMenuPanel::new(menu, item_name, subscriber, cx)
            });
        } else {
            // No menu — activate the item directly (e.g. show window)
            let subscriber = self.subscriber.clone();
            let item_name = item.name.clone();
            cx.spawn(async move |_| {
                let _ = subscriber
                    .dispatch(TrayCommand::Activate { item_name })
                    .await;
            })
            .detach();
        }
    }

    /// Dispatch a tray command asynchronously.
    fn dispatch_command(&self, command: TrayCommand, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        cx.spawn(async move |_| {
            let _ = subscriber.dispatch(command).await;
        })
        .detach();
    }
}

impl Render for Tray {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let items: Vec<_> = self.data.items.clone();
        let config = &cx.config().bar.modules.tray;
        let icon_size = config.icon_size;
        let item_size = icon_size.max(style::TRAY_ITEM_SIZE);

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;

        div()
            .id("systray")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .children(items.into_iter().map(|item| {
                let item_for_left = item.clone();
                let item_for_right = item.clone();
                let item_for_middle = item.clone();

                // Get the best icon representation - prefer icon name for nerd font rendering
                let icon_char = match &item.icon {
                    Some(TrayIcon::Name(name)) => get_icon_char(name, item.id.as_deref()),
                    Some(TrayIcon::Pixmap { .. }) => get_icon_char("", item.id.as_deref()),
                    None => get_icon_char("", item.id.as_deref()),
                };

                div()
                    .id(ElementId::Name(SharedString::from(format!(
                        "tray-item-{}",
                        item.name
                    ))))
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(item_size))
                    .rounded(px(radius::SM))
                    .cursor_pointer()
                    .bg(interactive_default)
                    .hover(move |s| s.bg(interactive_hover))
                    .active(move |s| s.bg(interactive_active))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.on_item_click(&item_for_left, cx);
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _event, _window, cx| {
                            this.dispatch_command(
                                TrayCommand::ContextMenu {
                                    item_name: item_for_right.name.clone(),
                                },
                                cx,
                            );
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(move |this, _event, _window, cx| {
                            this.dispatch_command(
                                TrayCommand::SecondaryActivate {
                                    item_name: item_for_middle.name.clone(),
                                },
                                cx,
                            );
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(icon_size))
                            .text_color(text_primary)
                            .child(icon_char),
                    )
            }))
    }
}

// ============================================================================
// Tray Menu Panel
// ============================================================================

/// Panel content for displaying a tray item's menu.
struct TrayMenuPanel {
    menu: MenuLayout,
    item_name: String,
    subscriber: services::TraySubscriber,
    /// Track which submenus are expanded (by menu ID)
    expanded_submenus: Vec<i32>,
}

impl TrayMenuPanel {
    fn new(
        menu: MenuLayout,
        item_name: String,
        subscriber: services::TraySubscriber,
        cx: &mut Context<Self>,
    ) -> Self {
        // Subscribe to tray data updates so the menu refreshes live
        // (e.g. after about_to_show triggers a layout_updated signal)
        let name = item_name.clone();
        watch(cx, subscriber.subscribe(), move |this, data, cx| {
            if let Some(item) = data.items.iter().find(|i| i.name == name)
                && let Some(menu) = &item.menu
            {
                this.menu = menu.clone();
                cx.notify();
            }
        });

        Self {
            menu,
            item_name,
            subscriber,
            expanded_submenus: Vec::new(),
        }
    }

    /// Handle clicking on a menu item.
    fn activate_menu_item(&self, menu_id: i32, window: &mut Window, cx: &mut Context<Self>) {
        let subscriber = self.subscriber.clone();
        let item_name = self.item_name.clone();
        cx.spawn(async move |_, _| {
            let _ = subscriber
                .dispatch(TrayCommand::MenuItemClicked { item_name, menu_id })
                .await;
        })
        .detach();

        // Close the menu panel
        window.remove_window();
    }

    /// Toggle submenu expansion state.
    /// Calls about_to_show when expanding to trigger lazy menu population.
    fn toggle_submenu(&mut self, menu_id: i32, cx: &mut Context<Self>) {
        if let Some(pos) = self.expanded_submenus.iter().position(|&id| id == menu_id) {
            self.expanded_submenus.remove(pos);
        } else {
            // Notify the app to populate the submenu before expanding
            let subscriber = self.subscriber.clone();
            let item_name = self.item_name.clone();
            cx.spawn(async move |_, _| {
                let _ = subscriber
                    .dispatch(TrayCommand::AboutToShow { item_name, menu_id })
                    .await;
            })
            .detach();
            self.expanded_submenus.push(menu_id);
        }
        cx.notify();
    }

    /// Check if a submenu is expanded
    fn is_submenu_expanded(&self, menu_id: i32) -> bool {
        self.expanded_submenus.contains(&menu_id)
    }

    /// Render menu items recursively with collapsible submenus.
    fn render_menu_items(
        &self,
        items: &[MenuLayout],
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let theme = cx.theme();
        let mut elements = Vec::new();

        // Pre-compute colors for closures
        let border_subtle = theme.border.subtle;
        let interactive_hover = theme.interactive.hover;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_disabled = theme.text.disabled;
        let accent_primary = theme.accent.primary;

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
            if props.type_.as_deref() == Some("separator")
                || (label.is_empty() && children.is_empty())
            {
                elements.push(render_menu_separator(border_subtle).into_any_element());
                continue;
            }

            let menu_id = *id;
            let is_enabled = props.enabled.unwrap_or(true);
            let has_submenu = !children.is_empty();
            let is_expanded = has_submenu && self.is_submenu_expanded(menu_id);
            let indent = depth as f32 * spacing::MD;

            elements.push(
                render_menu_item(
                    menu_id,
                    &label,
                    props,
                    is_enabled,
                    has_submenu,
                    is_expanded,
                    indent,
                    interactive_hover,
                    text_primary,
                    text_muted,
                    text_disabled,
                    accent_primary,
                    cx,
                )
                .into_any_element(),
            );

            // Render submenu items if expanded
            if is_expanded {
                let submenu_elements = self.render_menu_items(children, depth + 1, cx);
                elements.extend(submenu_elements);
            }
        }

        elements
    }
}

/// Render a separator line
fn render_menu_separator(border_color: gpui::Hsla) -> impl IntoElement {
    div()
        .w_full()
        .px(px(spacing::SM))
        .py(px(spacing::XS))
        .child(div().h(px(1.)).w_full().bg(border_color))
}

/// Render a single menu item
#[allow(clippy::too_many_arguments)]
fn render_menu_item(
    menu_id: i32,
    label: &str,
    props: &MenuLayoutProps,
    is_enabled: bool,
    has_submenu: bool,
    is_expanded: bool,
    indent: f32,
    interactive_hover: gpui::Hsla,
    text_primary: gpui::Hsla,
    text_muted: gpui::Hsla,
    text_disabled: gpui::Hsla,
    accent_primary: gpui::Hsla,
    cx: &mut Context<TrayMenuPanel>,
) -> impl IntoElement {
    // Checkbox/radio indicator
    let toggle_indicator = props.toggle_type.as_ref().map(|toggle_type| {
        let is_checked = props.toggle_state == Some(1);
        match toggle_type.as_str() {
            "checkmark" => {
                if is_checked {
                    ("󰄬", true)
                } else {
                    ("󰄱", false)
                }
            }
            "radio" => {
                if is_checked {
                    ("󰄴", true)
                } else {
                    ("󰄱", false)
                }
            }
            _ => ("", false),
        }
    });

    let label_owned = label.to_string();
    let text_color = if !is_enabled {
        text_disabled
    } else {
        text_primary
    };

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "menu-item-{}",
            menu_id
        ))))
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .w_full()
        .pl(px(spacing::SM + indent))
        .pr(px(spacing::SM))
        .py(px(spacing::XS + 2.0))
        .rounded(px(radius::SM))
        .mx(px(spacing::XS))
        .when(is_enabled, |el| {
            el.cursor_pointer().hover(move |s| s.bg(interactive_hover))
        })
        .when(!is_enabled, |el| el.cursor_default())
        .when(is_enabled && !has_submenu, |el| {
            el.on_click(cx.listener(move |this, _, window, cx| {
                this.activate_menu_item(menu_id, window, cx);
            }))
        })
        .when(has_submenu, |el| {
            el.on_click(cx.listener(move |this, _, _window, cx| {
                this.toggle_submenu(menu_id, cx);
            }))
        })
        // Toggle indicator (checkbox/radio)
        .when_some(toggle_indicator, |el, (icon, is_checked)| {
            el.child(
                div()
                    .text_size(px(font_size::SM))
                    .text_color(if is_checked {
                        accent_primary
                    } else {
                        text_muted
                    })
                    .child(icon),
            )
        })
        // Label
        .child(
            div()
                .flex_1()
                .text_size(px(font_size::SM))
                .text_color(text_color)
                .overflow_hidden()
                .text_ellipsis()
                .child(label_owned),
        )
        // Submenu indicator with rotation animation
        .when(has_submenu, |el| {
            el.child(
                div()
                    .text_size(px(font_size::XS))
                    .text_color(text_muted)
                    .child(if is_expanded { "󰅀" } else { "󰅂" }),
            )
        })
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
            .text_color(theme.text.primary)
            .overflow_hidden()
            .child(
                div()
                    .id("systray-menu-scroll")
                    .size_full()
                    .py(px(spacing::XS))
                    .overflow_y_scroll()
                    .child(div().flex().flex_col().gap(px(1.)).children(menu_items)),
            )
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
        "chrome" | "google-chrome" | "chromium" | "chromium-browser" => "",
        "telegram" | "telegram-desktop" => "",
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
            "chrome" | "google-chrome" | "chromium" | "chromium-browser" => "",
            "telegram" | "telegram-desktop" => "",
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
            "kdeconnect" | "kdeconnectd" | "kde connect indicator" => "󰄜",
            "tailscale" | "tailscale-systray" => "󰖂",
            "remmina" | "org.remmina.remmina" | "remmina-icon" => "󰢹",
            _ => "",
        };

        if !icon.is_empty() {
            return icon;
        }
    }

    // Heuristic fallback by substring (covers many variant ids/names).
    if let Some(icon) = infer_icon_from_hint(&name.to_lowercase()) {
        return icon;
    }
    if let Some(id) = app_id
        && let Some(icon) = infer_icon_from_hint(&id.to_lowercase())
    {
        return icon;
    }

    // Fallback icon - log for easier icon mapping
    tracing::debug!("No icon mapping for name='{}' app_id={:?}", name, app_id);
    "󰀻"
}

fn infer_icon_from_hint(hint: &str) -> Option<&'static str> {
    if hint.contains("chrome") || hint.contains("chromium") {
        Some("")
    } else if hint.contains("telegram") {
        Some("")
    } else if hint.contains("discord") || hint.contains("vesktop") {
        Some("󰙯")
    } else if hint.contains("spotify") {
        Some("󰓇")
    } else if hint.contains("steam") {
        Some("󰓓")
    } else if hint.contains("network") || hint.contains("wifi") || hint.contains("nm-") {
        Some("󰖩")
    } else if hint.contains("bluetooth") || hint.contains("blue") {
        Some("󰂯")
    } else if hint.contains("audio") || hint.contains("volume") || hint.contains("pulse") {
        Some("󰕾")
    } else if hint.contains("battery") || hint.contains("power") {
        Some("󰁹")
    } else if hint.contains("kdeconnect") || hint.contains("kde connect") {
        Some("󰄜")
    } else if hint.contains("vpn") {
        Some("󰕥")
    } else if hint.contains("cloud") || hint.contains("dropbox") || hint.contains("sync") {
        Some("󰇣")
    } else if hint.contains("remote") || hint.contains("remmina") {
        Some("󰢹")
    } else {
        None
    }
}
