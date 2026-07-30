//! System tray widget displaying StatusNotifierItem icons.

mod config;
pub use config::TrayConfig;

use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use gpui::{
    AnyElement, App, Context, ElementId, MouseButton, Render, RenderImage, SharedString, Size,
    Window, div, img, prelude::*, px,
};
use image::{Frame, RgbaImage};
use services::{MenuLayout, MenuLayoutProps, TrayCommand, TrayData, TrayIcon, TrayItem};
use std::sync::Arc;
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use super::{BarWidget, BarWidgetShell, style};
use crate::config::{ActiveConfig, Config};
use crate::state::AppState;
use crate::state::watch;

/// System tray widget that displays tray icons.
pub struct Tray {
    subscriber: services::TraySubscriber,
    data: TrayData,
}

impl Tray {
    /// Create a new system tray widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::tray(cx).clone();
        let data = subscriber.get();

        // Subscribe to tray data changes
        watch(cx, subscriber.subscribe(), |this, new_data, cx| {
            this.data = new_data;
            cx.notify();
        });

        Self { subscriber, data }
    }

    /// Handle left-clicking on a tray item.
    /// Opens menu panel if the item has a menu, otherwise calls Activate.
    fn on_item_click(
        &self,
        item: &TrayItem,
        event: &gpui::MouseDownEvent,
        window: &Window,
        cx: &mut App,
    ) {
        if let Some(menu) = item.menu.clone() {
            let panel_id = format!("systray-{}", item.name);
            let subscriber = self.subscriber.clone();
            let item_name = item.name.clone();
            let config = Config::global(cx);
            let panel_size = Size::new(px(250.0), px(400.0));
            let (anchor, margin) =
                panel_placement_from_event(config.bar.position, event, window, cx, panel_size);

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

    fn render_tray_item(
        &self,
        item: TrayItem,
        icon_size: f32,
        item_size: f32,
        theme: &ui::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let item_name_right = item.name.clone();
        let item_name_middle = item.name.clone();

        let icon_element: AnyElement = if let Some((w, h, data)) = item.icon_pixmap() {
            let mut bgra = data.to_vec();
            for pixel in bgra.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            if let Some(buffer) = RgbaImage::from_raw(w, h, bgra) {
                let frame = Frame::new(buffer);
                let render_img = Arc::new(RenderImage::new(vec![frame]));
                img(render_img).size(px(icon_size)).into_any_element()
            } else {
                render_icon_name(get_icon_name("", item.id.as_deref()), icon_size)
            }
        } else {
            let name = match &item.icon {
                Some(TrayIcon::Name(name)) => get_icon_name(name, item.id.as_deref()),
                None => get_icon_name("", item.id.as_deref()),
                Some(TrayIcon::Pixmap { .. }) => unreachable!(),
            };
            render_icon_name(name, icon_size)
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
            .rounded(Radius::Small.pixels())
            .cursor_pointer()
            .bg(theme.colors.border_transparent)
            .hover(move |s| s.bg(theme.colors.elevated_surface_background))
            .active(move |s| s.bg(theme.colors.element_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.on_item_click(&item, event, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _event, _window, cx| {
                    this.dispatch_command(
                        TrayCommand::ContextMenu {
                            item_name: item_name_right.clone(),
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
                            item_name: item_name_middle.clone(),
                        },
                        cx,
                    );
                }),
            )
            .child(icon_element)
            .into_any_element()
    }

    fn render_tray_strip(
        &self,
        items: Vec<TrayItem>,
        icon_size: f32,
        item_size: f32,
        theme: &ui::Theme,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("systray")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::group_gap(is_vertical)))
            .children(
                items
                    .into_iter()
                    .map(|item| self.render_tray_item(item, icon_size, item_size, theme, cx)),
            )
            .into_any_element()
    }
}

impl BarWidget for Tray {
    fn shell(&self) -> BarWidgetShell {
        BarWidgetShell::Group
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items: Vec<_> = self.data.items.clone();
        let config = &cx.config().bar.modules.tray;
        let icon_size = config.icon_size;
        let item_size = icon_size.max(style::TRAY_ITEM_SIZE);
        let theme = cx.theme().clone();
        self.render_tray_strip(items, icon_size, item_size, &theme, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items: Vec<_> = self.data.items.clone();
        let config = &cx.config().bar.modules.tray;
        let icon_size = config.icon_size;
        let item_size = icon_size.max(style::TRAY_ITEM_SIZE);
        let theme = cx.theme().clone();
        self.render_tray_strip(items, icon_size, item_size, &theme, false, cx)
    }
}

impl Render for Tray {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
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
        let border_subtle = theme.colors.border_variant;
        let interactive_hover = theme.colors.element_hover;
        let text_primary = theme.colors.text;
        let text_muted = theme.colors.text_muted;
        let text_disabled = theme.colors.text_disabled;
        let accent_primary = theme.colors.accent;

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
            let indent = depth as f32 * Spacing::Large.value();

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
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
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
    let _theme = cx.theme();

    // Checkbox/radio indicator
    let toggle_indicator = props.toggle_type.as_ref().and_then(|toggle_type| {
        let is_checked = props.toggle_state == Some(1);
        let icon = match (toggle_type.as_str(), is_checked) {
            ("checkmark", true) => IconName::Check,
            ("radio", true) => IconName::CheckCircle,
            ("checkmark" | "radio", false) => IconName::Square,
            _ => return None,
        };
        Some((icon, is_checked))
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
        .gap(Spacing::Medium.pixels())
        .w_full()
        .pl(px(Spacing::Medium.value() + indent))
        .pr(Spacing::Medium.pixels())
        .py(px(Spacing::XSmall.value() + 2.0))
        .rounded(Radius::Small.pixels())
        .mx(Spacing::XSmall.pixels())
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
                Icon::new(icon)
                    .size(IconSize::Small)
                    .color(Color::Custom(if is_checked {
                        accent_primary
                    } else {
                        text_muted
                    })),
            )
        })
        // Label
        .child(
            div()
                .flex_1()
                .text_size(TextSize::Small.rems())
                .text_color(text_color)
                .overflow_hidden()
                .text_ellipsis()
                .child(label_owned),
        )
        // Submenu indicator with rotation animation
        .when(has_submenu, |el| {
            el.child(
                Icon::new(if is_expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(IconSize::XSmall)
                .color(Color::Muted),
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
            .panel_surface(cx)
            .text_color(theme.colors.text)
            .overflow_hidden()
            .child(
                div()
                    .id("systray-menu-scroll")
                    .size_full()
                    .py(Spacing::XSmall.pixels())
                    .overflow_y_scroll()
                    .child(div().flex().flex_col().gap(px(1.)).children(menu_items)),
            )
    }
}

// ============================================================================
// Icon Mapping
// ============================================================================

/// Render a tray item that gave us no pixmap, using the closest icon from
/// the embedded set.
fn render_icon_name(name: IconName, icon_size: f32) -> AnyElement {
    // Sized in device pixels rather than rems: these icons sit inline with
    // raster pixmaps drawn at `px(icon_size)`, so a rem-scaled fallback
    // would be the odd one out at any font scale but 1.0.
    Icon::new(name)
        .size(IconSize::Exact(icon_size))
        .color(Color::Default)
        .into_any_element()
}

/// Map a single identifier to an icon.
///
/// Apps we ship a brand mark for get it. The rest still map onto what they
/// *do* - a screenshot tool is a camera, a volume applet is a speaker - which
/// is also where an app whose mark we don't have lands (Vesktop borrows
/// Discord's, KDE Connect borrows KDE's).
fn lookup_icon(key: &str) -> Option<IconName> {
    match key {
        "discord" | "vesktop" => Some(IconName::Discord),
        "slack" => Some(IconName::Slack),
        "telegram" | "telegram-desktop" => Some(IconName::Telegram),
        "spotify" => Some(IconName::Spotify),
        "firefox" => Some(IconName::Firefox),
        // Chromium's mark is Chrome's shape in a single colour, and the alpha
        // mask gpui renders keeps only the shape - so one file serves both.
        "chrome" | "google-chrome" | "chromium" | "chromium-browser" => Some(IconName::Chrome),
        "thunderbird" => Some(IconName::Thunderbird),
        "1password" => Some(IconName::OnePassword),
        "bitwarden" => Some(IconName::Bitwarden),
        "dropbox" => Some(IconName::Dropbox),
        "nextcloud" => Some(IconName::Nextcloud),
        "syncthing" | "syncthingtray" => Some(IconName::Syncthing),
        "nm-applet" | "network-manager" | "network-manager-applet" => Some(IconName::Wifi),
        "blueman" | "blueman-applet" | "blueman-tray" => Some(IconName::Bluetooth),
        "pasystray" | "pavucontrol" => Some(IconName::Volume),
        "udiskie" => Some(IconName::HardDrive),
        "flameshot" => Some(IconName::Camera),
        "kdeconnect" | "kdeconnectd" | "kde connect indicator" => Some(IconName::Kde),
        "tailscale" | "tailscale-systray" => Some(IconName::Tailscale),
        "remmina" | "org.remmina.remmina" | "org.remmina.remmina-status" | "remmina-icon" => {
            Some(IconName::ScreenShare)
        }
        "network" | "network-wireless" => Some(IconName::Wifi),
        "bluetooth" | "bluetooth-active" => Some(IconName::Bluetooth),
        "audio" | "audio-volume-high" => Some(IconName::Volume),
        "battery" | "battery-full" => Some(IconName::BatteryFull),
        _ => None,
    }
}

/// Map common icon names or app IDs to an icon.
fn get_icon_name(name: &str, app_id: Option<&str>) -> IconName {
    if let Some(icon) = lookup_icon(&name.to_lowercase()) {
        return icon;
    }

    if let Some(id) = app_id {
        let id_lower = id.to_lowercase();

        // Handle generic systray_XXXX pattern (often used by Go apps like Tailscale)
        if id_lower.starts_with("systray_") {
            return IconName::Network;
        }

        if let Some(icon) = lookup_icon(&id_lower) {
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
    IconName::Circle
}

/// Substring to icon, in match order.
///
/// Brand marks come first on purpose: "nextcloud" also contains "cloud" and
/// "syncthing" also contains "sync", and where we ship the mark it beats the
/// generic bucket. The generic entries below stay as the catch-all for the
/// applets we have no logo for.
const ICON_HINTS: &[(&str, IconName)] = &[
    ("firefox", IconName::Firefox),
    // "chromium" does not contain "chrome", so both spellings are needed.
    ("chrome", IconName::Chrome),
    ("chromium", IconName::Chrome),
    ("telegram", IconName::Telegram),
    ("discord", IconName::Discord),
    ("vesktop", IconName::Discord),
    ("slack", IconName::Slack),
    ("spotify", IconName::Spotify),
    ("thunderbird", IconName::Thunderbird),
    ("1password", IconName::OnePassword),
    ("bitwarden", IconName::Bitwarden),
    ("dropbox", IconName::Dropbox),
    ("nextcloud", IconName::Nextcloud),
    ("syncthing", IconName::Syncthing),
    ("tailscale", IconName::Tailscale),
    ("kdeconnect", IconName::Kde),
    ("kde connect", IconName::Kde),
    ("network", IconName::Wifi),
    ("wifi", IconName::Wifi),
    ("nm-", IconName::Wifi),
    ("bluetooth", IconName::Bluetooth),
    ("blue", IconName::Bluetooth),
    ("audio", IconName::Volume),
    ("volume", IconName::Volume),
    ("pulse", IconName::Volume),
    ("battery", IconName::BatteryFull),
    ("power", IconName::BatteryFull),
    ("vpn", IconName::Network),
    ("cloud", IconName::Cloud),
    ("sync", IconName::Cloud),
    ("remote", IconName::ScreenShare),
    ("remmina", IconName::ScreenShare),
];

fn infer_icon_from_hint(hint: &str) -> Option<IconName> {
    ICON_HINTS
        .iter()
        .find(|(needle, _)| hint.contains(needle))
        .map(|&(_, icon)| icon)
}
