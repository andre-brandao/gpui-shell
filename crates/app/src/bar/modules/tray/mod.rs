//! System tray widget displaying StatusNotifierItem icons.

mod config;
pub use config::TrayConfig;

use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, Pixels, Point, Render, RenderImage,
    SharedString, Size, Window, div, img, prelude::*, px,
};
use image::{Frame, RgbaImage};
use services::{MenuLayout, MenuLayoutProps, TrayCommand, TrayData, TrayIcon, TrayItem};
use std::collections::HashMap;
use std::sync::Arc;
use ui::patterns::PanelSurface;
use ui::{
    ActiveTheme, ButtonCommon, ButtonLike, ButtonStyle, Clickable, Color, Disableable, Divider,
    Icon, IconName, IconSize, Label, LabelCommon, List, ListItem, Spacing, TextSize,
};

use super::{BarWidget, BarWidgetShell, style};
use crate::config::{ActiveConfig, Config};
use crate::state::AppState;
use crate::state::watch;

/// A tray pixmap turned into an image GPUI can draw, kept across renders.
struct CachedIcon {
    /// The pixel buffer this was built from, to notice when the icon changes.
    source: Arc<Vec<u8>>,
    image: Arc<RenderImage>,
}

/// System tray widget that displays tray icons.
pub struct Tray {
    subscriber: services::TraySubscriber,
    data: TrayData,
    /// One image per item. GPUI uploads a `RenderImage` into the window's
    /// sprite atlas keyed by the image's id, and every `RenderImage::new` takes
    /// a fresh id - so building one per render (the bar repaints every time the
    /// clock ticks) grew the atlas by a page every few minutes. Entries leave
    /// the atlas only through `App::drop_image`; see `release_stale_icons`.
    icons: HashMap<String, CachedIcon>,
}

impl Tray {
    /// Create a new system tray widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::tray(cx).clone();
        let data = subscriber.get();

        // Subscribe to tray data changes
        watch(cx, subscriber.subscribe(), |this, new_data, cx| {
            this.release_stale_icons(&new_data, cx);
            this.data = new_data;
            cx.notify();
        });

        // The bar rebuilds its modules on a config reload, so hand back what
        // this one is still holding rather than stranding it in the atlas.
        cx.on_release(|this, cx| {
            for cached in std::mem::take(&mut this.icons).into_values() {
                cx.drop_image(cached.image, None);
            }
        })
        .detach();

        Self {
            subscriber,
            data,
            icons: HashMap::new(),
        }
    }

    /// Give GPUI back the atlas tiles for the icons this update retires.
    ///
    /// Dropping a `RenderImage` does not free its sprite-atlas entry - only
    /// [`App::drop_image`] does - so an item that vanished and an icon the app
    /// swapped out both have to be released here, or their tile stays for the
    /// rest of the session.
    fn release_stale_icons(&mut self, new_data: &TrayData, cx: &mut App) {
        self.icons.retain(|name, cached| {
            let still_drawn = new_data.items.iter().any(|item| {
                item.name == *name
                    && matches!(
                        &item.icon,
                        Some(TrayIcon::Pixmap { data, .. }) if Arc::ptr_eq(data, &cached.source)
                    )
            });

            if !still_drawn {
                cx.drop_image(cached.image.clone(), None);
            }

            still_drawn
        });
    }

    /// The item's pixmap as a GPUI image, decoded once per icon.
    fn pixmap_image(
        &mut self,
        item_name: &str,
        width: u32,
        height: u32,
        data: &Arc<Vec<u8>>,
    ) -> Option<Arc<RenderImage>> {
        if let Some(cached) = self.icons.get(item_name)
            && Arc::ptr_eq(&cached.source, data)
        {
            return Some(cached.image.clone());
        }

        // A buffer that does not match the stated size cannot be drawn. Bail
        // before copying it, or the conversion below runs again every repaint.
        if data.len() < width as usize * height as usize * 4 {
            return None;
        }

        // Convert RGBA to BGRA
        let mut bgra = data.to_vec();
        for pixel in bgra.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }

        let buffer = RgbaImage::from_raw(width, height, bgra)?;
        let image = Arc::new(RenderImage::new(vec![Frame::new(buffer)]));
        self.icons.insert(
            item_name.to_string(),
            CachedIcon {
                source: data.clone(),
                image: image.clone(),
            },
        );

        Some(image)
    }

    /// Handle left-clicking on a tray item.
    /// Opens menu panel if the item has a menu, otherwise calls Activate.
    fn on_item_click(&self, item: &TrayItem, at: Point<Pixels>, window: &Window, cx: &mut App) {
        if let Some(menu) = item.menu.clone() {
            let panel_id = format!("systray-{}", item.name);
            let subscriber = self.subscriber.clone();
            let item_name = item.name.clone();
            let config = Config::global(cx);
            let panel_size = Size::new(px(250.0), px(400.0));
            let (anchor, margin) =
                panel_placement_from_event(config.bar.position, at, window, cx, panel_size);

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
        &mut self,
        item: TrayItem,
        icon_size: f32,
        item_size: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let aux_item_name = item.name.clone();

        let pixmap = match &item.icon {
            Some(TrayIcon::Pixmap {
                width,
                height,
                data,
            }) => self.pixmap_image(&item.name, *width, *height, data),
            _ => None,
        };

        let icon_element: AnyElement = if let Some(image) = pixmap {
            img(image).size(px(icon_size)).into_any_element()
        } else {
            let name = match &item.icon {
                Some(TrayIcon::Name(name)) => get_icon_name(name, item.id.as_deref()),
                // No name, or a pixmap that would not decode.
                _ => get_icon_name("", item.id.as_deref()),
            };
            render_icon_name(name, icon_size)
        };

        // `item_size` is the hit target; the icon is drawn at `icon_size`, so
        // the difference becomes the button's own padding.
        let padding_y = px(((item_size - icon_size) / 2.0).max(0.0));

        ButtonLike::new(ElementId::Name(SharedString::from(format!(
            "tray-item-{}",
            item.name
        ))))
        .style(ButtonStyle::Ghost)
        .width(px(item_size))
        .padding(px(0.0), padding_y)
        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.on_item_click(&item, event.position(), window, cx);
        }))
        .on_aux_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            let command = if event.is_middle_click() {
                TrayCommand::SecondaryActivate {
                    item_name: aux_item_name.clone(),
                }
            } else if event.is_right_click() {
                TrayCommand::ContextMenu {
                    item_name: aux_item_name.clone(),
                }
            } else {
                return;
            };
            this.dispatch_command(command, cx);
        }))
        .child(icon_element)
        .into_any_element()
    }

    fn render_tray_strip(
        &mut self,
        items: Vec<TrayItem>,
        icon_size: f32,
        item_size: f32,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        style::stack(is_vertical)
            .id("systray")
            .justify_center()
            .gap(px(style::group_gap(is_vertical)))
            .children(
                items
                    .into_iter()
                    .map(|item| self.render_tray_item(item, icon_size, item_size, cx)),
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
        self.render_tray_strip(items, icon_size, item_size, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let items: Vec<_> = self.data.items.clone();
        let config = &cx.config().bar.modules.tray;
        let icon_size = config.icon_size;
        let item_size = icon_size.max(style::TRAY_ITEM_SIZE);
        self.render_tray_strip(items, icon_size, item_size, false, cx)
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
    menu: Arc<MenuLayout>,
    item_name: String,
    subscriber: services::TraySubscriber,
    /// Track which submenus are expanded (by menu ID)
    expanded_submenus: Vec<i32>,
}

impl TrayMenuPanel {
    fn new(
        menu: Arc<MenuLayout>,
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
        let mut elements = Vec::new();

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
                elements.push(render_menu_separator().into_any_element());
                continue;
            }

            let menu_id = *id;
            let is_enabled = props.enabled.unwrap_or(true);
            let has_submenu = !children.is_empty();
            let is_expanded = has_submenu && self.is_submenu_expanded(menu_id);

            elements.push(
                render_menu_item(
                    menu_id,
                    &label,
                    props,
                    is_enabled,
                    has_submenu,
                    is_expanded,
                    depth,
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
fn render_menu_separator() -> impl IntoElement {
    div()
        .w_full()
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
        .child(Divider::horizontal())
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
    depth: usize,
    cx: &mut Context<TrayMenuPanel>,
) -> impl IntoElement {
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

    ListItem::new(ElementId::Name(SharedString::from(format!(
        "menu-item-{}",
        menu_id
    ))))
    // A submenu row stays clickable even when the item itself is disabled -
    // collapsing it is the panel's own affordance, not the app's action.
    .disabled(!is_enabled && !has_submenu)
    .indent_level(depth)
    .indent_step_size(Spacing::Large.pixels())
    .on_click(cx.listener(move |this, _, window, cx| {
        if has_submenu {
            this.toggle_submenu(menu_id, cx);
        } else {
            this.activate_menu_item(menu_id, window, cx);
        }
    }))
    .when_some(toggle_indicator, |el, (icon, is_checked)| {
        el.start_slot(Icon::new(icon).size(IconSize::Small).color(if is_checked {
            Color::Accent
        } else {
            Color::Muted
        }))
    })
    .child(
        Label::new(label.to_string())
            .size(TextSize::Small)
            .color(if is_enabled {
                Color::Default
            } else {
                Color::Disabled
            })
            .truncate(),
    )
    .when(has_submenu, |el| {
        el.end_slot(
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
                    .child(
                        List::new()
                            .empty_message("This item has no menu")
                            .children(menu_items),
                    ),
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
