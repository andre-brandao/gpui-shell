//! System tray widget using Zed UI colors and spacing.
//!
//! Shows StatusNotifierItem icons and opens per-item menus in a panel.

use crate::panel::{PanelConfig, toggle_panel};
use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{
    App, Context, ElementId, Hsla, MouseButton, Render, SharedString, Window, div,
    layer_shell::Anchor, prelude::*, px, rems,
};
use services::{
    MenuLayout, MenuLayoutProps, TrayCommand, TrayData, TrayIcon, TrayItem, TraySubscriber,
};
use ui::prelude::*;

/// System tray widget that displays tray icons.
pub struct Tray {
    subscriber: TraySubscriber,
    data: TrayData,
}

impl Tray {
    pub fn new(services: services::Services, cx: &mut Context<Self>) -> Self {
        let subscriber = services.tray.clone();
        let data = subscriber.get();

        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_data) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.data = new_data;
                            cx.notify();
                        })
                        .is_err()
                    {
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

        let visible_items = count_top_level_menu_items(&menu.2);
        let menu_height = (visible_items * 26).min(400) as f32 + 12.0;

        let config = PanelConfig {
            width: 260.0,
            height: menu_height,
            anchor: Anchor::TOP | Anchor::RIGHT,
            margin: (0.0, 8.0, 0.0, 0.0),
            namespace: "systray-menu".to_string(),
        };

        toggle_panel(&panel_id, config, cx, move |_cx| {
            TrayMenuPanel::new(menu, item_name, subscriber)
        });
    }
}

impl Render for Tray {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<_> = self.data.items.clone();

        let (hover_bg, active_bg, text_primary, text_muted, element_bg) = {
            let c = cx.theme().colors();
            (
                c.element_hover,
                c.element_active,
                c.text,
                c.text_muted,
                c.element_background,
            )
        };

        div()
            .id("systray")
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(items.into_iter().map(|item| {
                let item_clone = item.clone();
                let has_menu = item.menu.is_some();

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
                    .size(px(28.0))
                    .rounded(px(7.0))
                    .cursor_pointer()
                    .bg(element_bg)
                    .hover(move |s| s.bg(hover_bg))
                    .active(move |s| s.bg(active_bg))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _event, _window, cx| {
                            this.on_item_click(&item_clone, cx);
                        }),
                    )
                    .child(
                        div()
                            .text_size(rems(0.9))
                            .text_color(if has_menu { text_primary } else { text_muted })
                            .child(icon_char),
                    )
            }))
    }
}

/// Count top-level visible menu items for height calculation.
fn count_top_level_menu_items(items: &[MenuLayout]) -> usize {
    items
        .iter()
        .filter(|MenuLayout(_, props, _)| props.visible != Some(false))
        .count()
}

// ============================================================================ //
// Tray Menu Panel
// ============================================================================ //

struct TrayMenuPanel {
    menu: MenuLayout,
    item_name: String,
    subscriber: TraySubscriber,
    expanded_submenus: Vec<i32>,
}

impl TrayMenuPanel {
    fn new(menu: MenuLayout, item_name: String, subscriber: TraySubscriber) -> Self {
        Self {
            menu,
            item_name,
            subscriber,
            expanded_submenus: Vec::new(),
        }
    }

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

        window.remove_window();
    }

    fn toggle_submenu(&mut self, menu_id: i32, cx: &mut Context<Self>) {
        if let Some(pos) = self.expanded_submenus.iter().position(|&id| id == menu_id) {
            self.expanded_submenus.remove(pos);
        } else {
            self.expanded_submenus.push(menu_id);
        }
        cx.notify();
    }

    fn is_submenu_expanded(&self, menu_id: i32) -> bool {
        self.expanded_submenus.contains(&menu_id)
    }

    fn render_menu_items(
        &self,
        items: &[MenuLayout],
        depth: usize,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let colors = cx.theme().colors();
        let hover_bg = colors.element_hover;
        let text_primary = colors.text;
        let text_muted = colors.text_muted;
        let text_disabled = colors.text_disabled;
        let accent = colors.text_accent;
        let border = colors.border;
        let mut elements = Vec::new();

        for layout in items {
            let MenuLayout(id, props, children) = layout;

            if props.visible == Some(false) {
                continue;
            }

            let label = props
                .label
                .as_ref()
                .map(|l| l.replace('_', ""))
                .unwrap_or_default();

            if props.type_.as_deref() == Some("separator")
                || (label.is_empty() && children.is_empty())
            {
                elements.push(render_menu_separator(border).into_any_element());
                continue;
            }

            let menu_id = *id;
            let is_enabled = props.enabled.unwrap_or(true);
            let has_submenu = !children.is_empty();
            let is_expanded = has_submenu && self.is_submenu_expanded(menu_id);
            let indent = depth as f32 * 12.0;

            elements.push(
                render_menu_item(
                    menu_id,
                    &label,
                    props,
                    is_enabled,
                    has_submenu,
                    is_expanded,
                    indent,
                    hover_bg,
                    text_primary,
                    text_muted,
                    text_disabled,
                    accent,
                    cx,
                )
                .into_any_element(),
            );

            if is_expanded {
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
        let colors = cx.theme().colors();

        div()
            .id("systray-menu-panel")
            .size_full()
            .bg(colors.surface_background)
            .border_1()
            .border_color(colors.border)
            .rounded(px(10.0))
            .py(px(4.0))
            .text_color(colors.text)
            .overflow_y_scroll()
            .child(div().flex().flex_col().gap(px(2.0)).children(menu_items))
    }
}

/// Render a separator line
fn render_menu_separator(border_color: Hsla) -> impl IntoElement {
    div()
        .w_full()
        .px(px(10.0))
        .py(px(4.0))
        .child(div().h(px(1.0)).w_full().bg(border_color))
}

/// Render a single menu item
fn render_menu_item(
    menu_id: i32,
    label: &str,
    props: &MenuLayoutProps,
    is_enabled: bool,
    has_submenu: bool,
    is_expanded: bool,
    indent: f32,
    hover_bg: Hsla,
    text_primary: Hsla,
    text_muted: Hsla,
    text_disabled: Hsla,
    accent: Hsla,
    cx: &mut Context<TrayMenuPanel>,
) -> impl IntoElement {
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
        .gap(px(8.0))
        .w_full()
        .pl(px(10.0 + indent))
        .pr(px(10.0))
        .py(px(6.0))
        .rounded(px(7.0))
        .mx(px(6.0))
        .when(is_enabled, |el| {
            el.cursor_pointer().hover(move |s| s.bg(hover_bg))
        })
        .when(!is_enabled, |el| el.cursor_default())
        .when(is_enabled && !has_submenu, |el| {
            el.on_click(cx.listener(move |this, _, window, _cx| {
                this.activate_menu_item(menu_id, window);
            }))
        })
        .when(has_submenu, |el| {
            el.on_click(cx.listener(move |this, _, _window, cx| {
                this.toggle_submenu(menu_id, cx);
            }))
        })
        .when_some(toggle_indicator, |el, (icon, is_checked)| {
            el.child(
                div()
                    .text_size(rems(0.82))
                    .text_color(if is_checked { accent } else { text_muted })
                    .child(icon),
            )
        })
        .child(
            div()
                .flex_1()
                .text_size(rems(0.86))
                .text_color(text_color)
                .overflow_hidden()
                .text_ellipsis()
                .child(label_owned),
        )
        .when(has_submenu, |el| {
            el.child(
                div()
                    .text_size(rems(0.8))
                    .text_color(text_muted)
                    .child(if is_expanded { "󰅀" } else { "󰅂" }),
            )
        })
}

// ============================================================================ //
// Icon Mapping
// ============================================================================ //

/// Map common icon names or app IDs to nerd font characters.
fn get_icon_char(name: &str, app_id: Option<&str>) -> &'static str {
    let icon = match name.to_lowercase().as_str() {
        "discord" => "󰙯",
        "spotify" => "󰓇",
        "steam" => "󰓓",
        "firefox" => "󰈹",
        "chrome" | "google-chrome" | "chromium" => "󰊯",
        "telegram" => "󰌾",
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

    if let Some(id) = app_id {
        let id_lower = id.to_lowercase();

        if id_lower.starts_with("systray_") {
            return "󰖂";
        }

        let icon = match id_lower.as_str() {
            "discord" | "vesktop" => "󰙯",
            "spotify" => "󰓇",
            "steam" => "󰓓",
            "firefox" => "󰈹",
            "chrome" | "google-chrome" | "chromium" => "󰊯",
            "telegram" | "telegram-desktop" => "󰌾",
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

    tracing::debug!("No icon mapping for name='{}' app_id={:?}", name, app_id);
    "󰀻"
}
