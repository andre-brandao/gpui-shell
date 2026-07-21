//! Dock UI: pinned + running app icons, shown as a standalone layer-shell
//! window per configured monitor.

pub mod config;
mod context_menu;
mod item;
mod picker;

pub use config::{DockConfig, DockHoverEffect, DockMonitors, DockVisibility};

use gpui::{
    AnyElement, App, Bounds, Context, DisplayId, MouseDownEvent, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, img, layer_shell::*,
    point, prelude::*, px,
};
use std::collections::HashMap;
use ui::{ActiveTheme, radius, spacing};

use crate::config::{ActiveConfig, Config};
use crate::state::{AppState, display_id_for_window, record_window_display, watch};
use item::{DockItem, build_dock_items};

const INDICATOR_ROW_HEIGHT: f32 = 5.0;
pub(super) const DOCK_CONTEXT_MENU_WIDTH: f32 = 160.0;
const DOCK_APP_PICKER_WIDTH: f32 = 240.0;
pub(super) const DOCK_CONTEXT_MENU_ROW_HEIGHT: f32 = 40.0;
pub(super) const DOCK_CONTEXT_MENU_VERTICAL_PADDING: f32 = spacing::XS;
pub(super) const DOCK_CONTEXT_MENU_GAP: f32 = 2.0;
pub(super) const DOCK_CONTEXT_MENU_BORDER_WIDTH: f32 = 1.0;
// GPUI can reserve more vertical space than the nominal row height while it
// lays out text, particularly in layer-shell windows.
const DOCK_CONTEXT_MENU_TEXT_LAYOUT_HEADROOM: f32 = 12.0;
const DOCK_HIDDEN_STRIP_SIZE: f32 = 6.0;
const DOCK_REVEAL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(750);

fn dock_context_menu_height(action_count: usize) -> f32 {
    let gaps = action_count.saturating_sub(1) as f32 * DOCK_CONTEXT_MENU_GAP;
    let content_height = action_count as f32 * DOCK_CONTEXT_MENU_ROW_HEIGHT;
    let vertical_padding = 2.0 * DOCK_CONTEXT_MENU_VERTICAL_PADDING;
    let borders = 2.0 * DOCK_CONTEXT_MENU_BORDER_WIDTH;

    content_height + gaps + vertical_padding + borders + DOCK_CONTEXT_MENU_TEXT_LAYOUT_HEADROOM
}

fn dock_context_menu_size(action_count: usize) -> Size<gpui::Pixels> {
    Size::new(
        px(DOCK_CONTEXT_MENU_WIDTH),
        px(dock_context_menu_height(action_count)),
    )
}

struct DockPanelPlacement {
    anchor: Anchor,
    margin: (f32, f32, f32, f32),
}

/// Position a dock panel beside a dock-local click while keeping the complete
/// panel within the output's usable area. Layer-shell windows do not reliably
/// expose their global position, so this derives it from the dock's known edge
/// and centered layout instead.
fn dock_panel_placement_from_dock_click(
    dock_position: crate::bar::config::BarPosition,
    dock_click: gpui::Point<gpui::Pixels>,
    dock_size: Size<gpui::Pixels>,
    menu_size: Size<gpui::Pixels>,
    display_bounds: Bounds<gpui::Pixels>,
    usable_bounds: Bounds<gpui::Pixels>,
) -> DockPanelPlacement {
    let display_width: f32 = display_bounds.size.width.into();
    let display_height: f32 = display_bounds.size.height.into();
    let dock_width: f32 = dock_size.width.into();
    let dock_height: f32 = dock_size.height.into();
    let menu_width: f32 = menu_size.width.into();
    let menu_height: f32 = menu_size.height.into();

    let usable_left: f32 = (usable_bounds.origin.x - display_bounds.origin.x).into();
    let usable_top: f32 = (usable_bounds.origin.y - display_bounds.origin.y).into();
    let usable_right: f32 =
        (usable_bounds.origin.x + usable_bounds.size.width - display_bounds.origin.x).into();
    let usable_bottom: f32 =
        (usable_bounds.origin.y + usable_bounds.size.height - display_bounds.origin.y).into();

    let min_x = usable_left;
    let max_x = (usable_right - menu_width).max(min_x);
    let min_y = usable_top;
    let max_y = (usable_bottom - menu_height).max(min_y);

    let (dock_x, dock_y) = match dock_position {
        crate::bar::config::BarPosition::Top => {
            ((display_width - dock_width).max(0.0) / 2.0, spacing::SM)
        }
        crate::bar::config::BarPosition::Bottom => (
            (display_width - dock_width).max(0.0) / 2.0,
            display_height - dock_height - spacing::SM,
        ),
        crate::bar::config::BarPosition::Left => {
            (spacing::SM, (display_height - dock_height).max(0.0) / 2.0)
        }
        crate::bar::config::BarPosition::Right => (
            display_width - dock_width - spacing::SM,
            (display_height - dock_height).max(0.0) / 2.0,
        ),
    };

    let click_x: f32 = dock_x + Into::<f32>::into(dock_click.x);
    let click_y: f32 = dock_y + Into::<f32>::into(dock_click.y);
    let (origin_x, origin_y) = match dock_position {
        crate::bar::config::BarPosition::Top => (click_x - menu_width / 2.0, dock_y + dock_height),
        crate::bar::config::BarPosition::Bottom => {
            (click_x - menu_width / 2.0, dock_y - menu_height)
        }
        crate::bar::config::BarPosition::Left => (dock_x + dock_width, click_y - menu_height / 2.0),
        crate::bar::config::BarPosition::Right => {
            (dock_x - menu_width, click_y - menu_height / 2.0)
        }
    };

    DockPanelPlacement {
        anchor: Anchor::TOP | Anchor::LEFT,
        margin: (
            clamp_dock_menu_origin(origin_y, min_y, max_y),
            0.0,
            0.0,
            clamp_dock_menu_origin(origin_x, min_x, max_x),
        ),
    }
}

fn dock_panel_placement_from_event(
    dock_position: crate::bar::config::BarPosition,
    event: &MouseDownEvent,
    window: &Window,
    cx: &App,
    dock_size: Size<gpui::Pixels>,
    menu_size: Size<gpui::Pixels>,
) -> DockPanelPlacement {
    let (display_bounds, usable_bounds) = display_id_for_window(window)
        .and_then(|display_id| cx.find_display(display_id))
        .map(|display| (display.bounds(), display.visible_bounds()))
        .unwrap_or_else(|| {
            let bounds = Bounds::new(point(px(0.0), px(0.0)), window.viewport_size());
            (bounds, bounds)
        });

    dock_panel_placement_from_dock_click(
        dock_position,
        event.position,
        dock_size,
        menu_size,
        display_bounds,
        usable_bounds,
    )
}

fn clamp_dock_menu_origin(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// Retain only the windows on a resolved compositor monitor. When the monitor
/// cannot be resolved, retain every window so the dock remains useful.
fn windows_for_monitor(
    windows: &[services::Window],
    monitor_name: Option<&str>,
) -> Vec<services::Window> {
    match monitor_name {
        Some(monitor_name) => windows
            .iter()
            .filter(|window| window.monitor == monitor_name)
            .cloned()
            .collect(),
        None => windows.to_vec(),
    }
}

fn dock_items(
    windows: &[services::Window],
    apps: &[services::Application],
    pinned: &[String],
    monitor_name: Option<&str>,
) -> Vec<DockItem> {
    let scoped_windows = windows_for_monitor(windows, monitor_name);
    build_dock_items(&scoped_windows, apps, pinned)
}

fn dock_item_count(
    windows: &[services::Window],
    apps: &[services::Application],
    pinned: &[String],
    monitor_name: Option<&str>,
) -> usize {
    dock_items(windows, apps, pinned, monitor_name).len().max(1)
}

fn next_cycle_index(previous: Option<usize>, window_count: usize) -> Option<usize> {
    (window_count > 0).then(|| previous.map_or(0, |index| (index + 1) % window_count))
}

fn toggled_pins(pinned: &mut Vec<String>, item_key: &str) {
    if let Some(pos) = pinned.iter().position(|pin| pin == item_key) {
        pinned.remove(pos);
    } else {
        pinned.push(item_key.to_string());
    }
}

fn dock_window_size(
    position: crate::bar::config::BarPosition,
    icon_size: f32,
    item_count: usize,
    hover_effect: config::DockHoverEffect,
) -> Size<gpui::Pixels> {
    let item_count = item_count.max(1) as f32;
    let primary_extent = item_count * (icon_size + spacing::SM) + spacing::SM;
    let cross_extent = icon_size
        + if position.is_vertical() {
            0.0
        } else {
            INDICATOR_ROW_HEIGHT
        }
        + spacing::SM * 2.0;
    let magnify_reserve = match hover_effect {
        config::DockHoverEffect::Magnify | config::DockHoverEffect::MagnifyLift => icon_size * 0.3,
        _ => 0.0,
    };
    let glow_reserve = match hover_effect {
        config::DockHoverEffect::Glow => 4.0,
        _ => 0.0,
    };
    let lift_reserve = match hover_effect {
        config::DockHoverEffect::Lift | config::DockHoverEffect::MagnifyLift => 8.0,
        _ => 0.0,
    };

    let primary_extent = primary_extent
        + if position.is_vertical() {
            item_count * INDICATOR_ROW_HEIGHT
        } else {
            0.0
        }
        + magnify_reserve
        + glow_reserve
        + if position.is_vertical() {
            lift_reserve
        } else {
            0.0
        };
    let cross_extent = cross_extent
        + magnify_reserve
        + glow_reserve
        + if position.is_vertical() {
            0.0
        } else {
            lift_reserve
        };

    if position.is_vertical() {
        Size::new(px(cross_extent), px(primary_extent))
    } else {
        Size::new(px(primary_extent), px(cross_extent))
    }
}

fn monitor_name_for_display(
    display_id: Option<DisplayId>,
    state: &services::CompositorState,
    cx: &App,
) -> Option<String> {
    let display_uuid = cx.find_display(display_id?)?.uuid().ok()?;

    state
        .monitors
        .iter()
        .find(|monitor| {
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, monitor.name.as_bytes()) == display_uuid
        })
        .map(|monitor| monitor.name.clone())
}

fn focused_window_is_on_other_monitor(
    focused_window: Option<&services::Window>,
    monitor_name: Option<&str>,
) -> bool {
    focused_window.is_some_and(|window| Some(window.monitor.as_str()) != monitor_name)
}

fn geometry_overlaps_dock(
    geometry: services::WindowGeometry,
    dock_bounds: Bounds<gpui::Pixels>,
) -> bool {
    let dock_left: f32 = dock_bounds.origin.x.into();
    let dock_top: f32 = dock_bounds.origin.y.into();
    let dock_right = dock_left + f32::from(dock_bounds.size.width);
    let dock_bottom = dock_top + f32::from(dock_bounds.size.height);

    let window_left = geometry.x as f32;
    let window_top = geometry.y as f32;
    let window_right = window_left + geometry.width as f32;
    let window_bottom = window_top + geometry.height as f32;

    window_left < dock_right
        && window_right > dock_left
        && window_top < dock_bottom
        && window_bottom > dock_top
}

fn dock_bounds_in_compositor_space(
    position: crate::bar::config::BarPosition,
    dock_size: Size<gpui::Pixels>,
    monitor: &services::Monitor,
) -> Bounds<gpui::Pixels> {
    let monitor_width = monitor.width as f32;
    let monitor_height = monitor.height as f32;
    let dock_width: f32 = dock_size.width.into();
    let dock_height: f32 = dock_size.height.into();
    let (x, y) = match position {
        crate::bar::config::BarPosition::Top => (
            monitor.x as f32 + (monitor_width - dock_width).max(0.0) / 2.0,
            monitor.y as f32 + spacing::SM,
        ),
        crate::bar::config::BarPosition::Bottom => (
            monitor.x as f32 + (monitor_width - dock_width).max(0.0) / 2.0,
            monitor.y as f32 + monitor_height - dock_height - spacing::SM,
        ),
        crate::bar::config::BarPosition::Left => (
            monitor.x as f32 + spacing::SM,
            monitor.y as f32 + (monitor_height - dock_height).max(0.0) / 2.0,
        ),
        crate::bar::config::BarPosition::Right => (
            monitor.x as f32 + monitor_width - dock_width - spacing::SM,
            monitor.y as f32 + (monitor_height - dock_height).max(0.0) / 2.0,
        ),
    };

    Bounds::new(point(px(x), px(y)), dock_size)
}

fn hidden_strip_size(
    position: crate::bar::config::BarPosition,
    dock_size: Size<gpui::Pixels>,
) -> Size<gpui::Pixels> {
    if position.is_vertical() {
        Size::new(px(DOCK_HIDDEN_STRIP_SIZE), dock_size.height)
    } else {
        Size::new(dock_size.width, px(DOCK_HIDDEN_STRIP_SIZE))
    }
}

fn revealed_after_visibility_update(
    was_hidden_by_rule: bool,
    was_revealed: bool,
    is_hidden_by_rule: bool,
) -> bool {
    if !is_hidden_by_rule {
        true
    } else if !was_hidden_by_rule {
        false
    } else {
        was_revealed
    }
}

fn dodge_windows_should_hide(
    focused_window: Option<&services::Window>,
    monitor_name: Option<&str>,
    dock_bounds: Option<Bounds<gpui::Pixels>>,
) -> bool {
    let intelligent_hide = focused_window_is_on_other_monitor(focused_window, monitor_name);
    let Some(geometry) = focused_window.and_then(|window| window.geometry) else {
        return intelligent_hide;
    };
    let Some(dock_bounds) = dock_bounds else {
        return intelligent_hide;
    };

    geometry_overlaps_dock(geometry, dock_bounds)
}

/// The dock's own view, rendered in a standalone layer-shell window.
struct Dock {
    compositor: services::CompositorSubscriber,
    state: services::CompositorState,
    cycle_index: HashMap<String, usize>,
    revealed: bool,
    hidden_by_rule: bool,
    reveal_generation: u64,
}

impl Dock {
    fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            cx.notify();
        });

        Self {
            compositor,
            state,
            cycle_index: HashMap::new(),
            revealed: true,
            hidden_by_rule: false,
            reveal_generation: 0,
        }
    }

    fn current_monitor_name(&self, window: &Window, cx: &App) -> Option<String> {
        monitor_name_for_display(display_id_for_window(window), &self.state, cx)
    }

    /// The focused window's full record, cross-referenced from the active
    /// window address because `ActiveWindow` has no monitor or geometry.
    fn focused_window(&self) -> Option<&services::Window> {
        let address = &self.state.active_window.as_ref()?.address;
        self.state
            .windows
            .iter()
            .find(|window| &window.id == address)
    }

    /// Whether the dock should be hidden for the configured visibility mode.
    /// `DodgeWindows` uses the intelligent-hide behavior when the compositor
    /// cannot report window geometry, as with Niri.
    fn should_hide(&self, window: &Window, dock_size: Size<gpui::Pixels>, cx: &App) -> bool {
        let monitor_name = self.current_monitor_name(window, cx);
        let focused_window = self.focused_window();
        let intelligent_hide =
            || focused_window_is_on_other_monitor(focused_window, monitor_name.as_deref());

        match cx.config().dock.visibility {
            config::DockVisibility::AlwaysVisible => false,
            config::DockVisibility::IntelligentHide => intelligent_hide(),
            config::DockVisibility::DodgeWindows => dodge_windows_should_hide(
                focused_window,
                monitor_name.as_deref(),
                monitor_name.as_deref().and_then(|monitor_name| {
                    self.state
                        .monitors
                        .iter()
                        .find(|monitor| monitor.name == monitor_name)
                        .map(|monitor| {
                            dock_bounds_in_compositor_space(
                                cx.config().dock.position,
                                dock_size,
                                monitor,
                            )
                        })
                }),
            ),
        }
    }

    fn schedule_rehide(&mut self, cx: &mut Context<Self>) {
        self.reveal_generation += 1;
        let generation = self.reveal_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DOCK_REVEAL_TIMEOUT).await;
            let _ = this.update(cx, |this, cx| {
                if this.hidden_by_rule && this.reveal_generation == generation {
                    this.revealed = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn items(&self, window: &Window, cx: &App) -> Vec<DockItem> {
        let apps = AppState::applications(cx).all();
        dock_items(
            &self.state.windows,
            apps,
            &cx.config().dock.pinned,
            self.current_monitor_name(window, cx).as_deref(),
        )
    }

    /// Launch a pinned app, focus a single window, or cycle a window group.
    fn activate(&mut self, item: &DockItem, _cx: &mut Context<Self>) {
        if item.windows.is_empty() {
            if let Some(exec) = &item.exec {
                services::Application {
                    name: item.name.clone(),
                    exec: exec.clone(),
                    icon: None,
                    icon_path: item.icon_path.clone(),
                    description: None,
                    desktop_file: std::path::PathBuf::from(&item.key),
                    startup_wm_class: None,
                }
                .launch();
            }
            return;
        }

        let index = next_cycle_index(self.cycle_index.get(&item.key).copied(), item.windows.len())
            .expect("non-empty dock items have a selectable window");
        if item.windows.len() > 1 {
            self.cycle_index.insert(item.key.clone(), index);
        }

        let window_id = item.windows[index].id.clone();
        if let Err(error) = self
            .compositor
            .dispatch(services::CompositorCommand::FocusWindow(window_id))
        {
            tracing::error!("Failed to focus window for '{}': {error}", item.name);
        }
    }

    fn render_item(&self, item: &DockItem, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().dock;
        let icon_size = config.icon_size;
        let hover_effect = config.hover_effect;
        let accent = theme.accent.primary;

        let mut element = div()
            .relative()
            .size(px(icon_size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius::MD));

        element = match hover_effect {
            config::DockHoverEffect::None => element,
            config::DockHoverEffect::Lift => element.hover(move |style| style.mb(px(8.0))),
            config::DockHoverEffect::Magnify => {
                element.hover(move |style| style.size(px(icon_size * 1.3)))
            }
            config::DockHoverEffect::Glow => {
                element.hover(move |style| style.border_2().border_color(accent))
            }
            config::DockHoverEffect::MagnifyLift => {
                element.hover(move |style| style.size(px(icon_size * 1.3)).mb(px(8.0)))
            }
        };

        let is_focused = item.windows.iter().any(|window| window.is_focused);
        let window_count = item.windows.len();
        let bar_color = if is_focused { accent } else { theme.text.muted };
        let bar_width = if window_count > 1 { 20.0 } else { 6.0 };

        let icon_element = element
            .when_some(item.icon_path.clone(), |element, path| {
                element.child(img(path).size(px(icon_size * 0.75)))
            })
            .when(item.icon_path.is_none(), |element| {
                element.child(
                    div()
                        .text_size(theme.font_sizes.md)
                        .text_color(theme.text.primary)
                        .child(item.name.chars().take(1).collect::<String>()),
                )
            })
            .when(window_count > 1, |element| {
                element.child(
                    div()
                        .absolute()
                        .top(px(-2.0))
                        .right(px(-2.0))
                        .min_w(px(14.0))
                        .h(px(14.0))
                        .px(px(3.0))
                        .rounded(px(7.0))
                        .bg(accent)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(theme.font_sizes.xs)
                        .text_color(theme.bg.primary)
                        .child(window_count.to_string()),
                )
            });

        let item_key = item.key.clone();
        div()
            .id(item.key.clone())
            .cursor_pointer()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, window, cx| {
                    if let Some(item) = this
                        .items(window, cx)
                        .into_iter()
                        .find(|item| item.key == item_key)
                    {
                        this.activate(&item, cx);
                    }
                }),
            )
            .on_mouse_down(gpui::MouseButton::Right, {
                let item = item.clone();
                cx.listener(move |_this, event, window, cx| {
                    let config = cx.config().dock.clone();
                    let action_count = 1 + usize::from(item.exec.is_some());
                    let panel_height = dock_context_menu_height(action_count);
                    let panel_size = dock_context_menu_size(action_count);
                    let placement = dock_panel_placement_from_event(
                        config.position,
                        event,
                        window,
                        cx,
                        window.viewport_size(),
                        panel_size,
                    );
                    let panel_config = crate::panel::PanelConfig {
                        width: DOCK_CONTEXT_MENU_WIDTH,
                        height: panel_height,
                        anchor: placement.anchor,
                        margin: placement.margin,
                        namespace: "dock-context-menu".to_string(),
                    };
                    let item_key = item.key.clone();
                    let panel_id = format!("dock-context-{item_key}");
                    let menu_panel_id = panel_id.clone();
                    let is_pinned = item.is_pinned;
                    let exec = item.exec.clone();
                    let app_name = item.name.clone();
                    let icon_path = item.icon_path.clone();
                    crate::panel::toggle_panel_on_display(
                        &panel_id,
                        panel_config,
                        display_id_for_window(window),
                        cx,
                        move |cx| {
                            context_menu::DockContextMenu::new(
                                menu_panel_id,
                                item_key,
                                is_pinned,
                                exec,
                                app_name,
                                icon_path,
                                cx,
                            )
                        },
                    );
                })
            })
            .child(icon_element)
            .when(item.is_running(), |element| {
                element.child(
                    div()
                        .w(px(bar_width))
                        .h(px(3.0))
                        .rounded(px(1.5))
                        .bg(bar_color),
                )
            })
            .into_any_element()
    }
}

impl Render for Dock {
    #[allow(refining_impl_trait)]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let position = cx.config().dock.position;
        let items = self.items(window, cx);
        let dock_size = dock_window_size(
            position,
            cx.config().dock.icon_size,
            items.len() + 1,
            cx.config().dock.hover_effect,
        );
        let is_hidden_by_rule = cx.config().dock.visibility
            != config::DockVisibility::AlwaysVisible
            && self.should_hide(window, dock_size, cx);
        self.revealed =
            revealed_after_visibility_update(self.hidden_by_rule, self.revealed, is_hidden_by_rule);
        self.hidden_by_rule = is_hidden_by_rule;

        if !self.revealed {
            let strip_size = hidden_strip_size(position, dock_size);
            if window.viewport_size() != strip_size {
                window.resize(strip_size);
            }
            return div()
                .id("dock-hidden-strip")
                .w(strip_size.width)
                .h(strip_size.height)
                .on_mouse_move(cx.listener(|this, _event, _window, cx| {
                    this.revealed = true;
                    this.schedule_rehide(cx);
                    cx.notify();
                }))
                .into_any_element();
        }

        let background = cx.theme().bg.primary;
        let border = cx.theme().border.subtle;
        let is_vertical = position.is_vertical();
        if window.viewport_size() != dock_size {
            window.resize(dock_size);
        }
        let elements: Vec<AnyElement> = items
            .iter()
            .map(|item| self.render_item(item, cx))
            .collect();

        div()
            .id("dock")
            .flex()
            .when(is_vertical, |element| element.flex_col())
            .items_center()
            .justify_center()
            .gap(px(spacing::SM))
            .px(px(spacing::SM))
            .py(px(spacing::SM))
            .rounded(px(radius::LG))
            .bg(background)
            .border_1()
            .border_color(border)
            .on_mouse_move(cx.listener(|this, _event, _window, cx| {
                if this.hidden_by_rule {
                    this.schedule_rehide(cx);
                }
            }))
            .children(elements)
            .child(
                div()
                    .id("dock-add")
                    .size(px(cx.config().dock.icon_size * 0.6))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(radius::MD))
                    .cursor_pointer()
                    .text_color(cx.theme().text.muted)
                    .hover(|style| style.text_color(cx.theme().text.primary))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|_this, event, window, cx| {
                            let config = cx.config().dock.clone();
                            let panel_size = gpui::Size::new(
                                px(DOCK_APP_PICKER_WIDTH),
                                px(picker::DOCK_APP_PICKER_HEIGHT),
                            );
                            let placement = dock_panel_placement_from_event(
                                config.position,
                                event,
                                window,
                                cx,
                                window.viewport_size(),
                                panel_size,
                            );
                            let panel_config = crate::panel::PanelConfig {
                                width: DOCK_APP_PICKER_WIDTH,
                                height: picker::DOCK_APP_PICKER_HEIGHT,
                                anchor: placement.anchor,
                                margin: placement.margin,
                                namespace: "dock-app-picker".to_string(),
                            };
                            crate::panel::toggle_panel_on_display(
                                "dock-app-picker",
                                panel_config,
                                display_id_for_window(window),
                                cx,
                                picker::DockAppPicker::new,
                            );
                        }),
                    )
                    .child("+"),
            )
            .into_any_element()
    }
}

/// Window options for a dock window, sized to fit its content rather than
/// spanning the monitor like the bar does.
fn window_options(display_id: Option<DisplayId>, cx: &App) -> WindowOptions {
    let config = &cx.config().dock;
    let is_vertical = config.position.is_vertical();
    let compositor_state = AppState::compositor(cx).get();
    let item_count = dock_item_count(
        &compositor_state.windows,
        AppState::applications(cx).all(),
        &config.pinned,
        monitor_name_for_display(display_id, &compositor_state, cx).as_deref(),
    ) + 1;
    let window_size = dock_window_size(
        config.position,
        config.icon_size,
        item_count,
        config.hover_effect,
    );

    let anchor = if is_vertical {
        match config.position {
            crate::bar::config::BarPosition::Left => Anchor::LEFT,
            _ => Anchor::RIGHT,
        }
    } else {
        match config.position {
            crate::bar::config::BarPosition::Top => Anchor::TOP,
            _ => Anchor::BOTTOM,
        }
    };

    WindowOptions {
        display_id,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: window_size,
        })),
        app_id: Some("gpuishell-dock".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "dock".to_string(),
            layer: Layer::Top,
            anchor,
            exclusive_zone: None,
            margin: Some((
                px(spacing::SM),
                px(spacing::SM),
                px(spacing::SM),
                px(spacing::SM),
            )),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Open a dock window on the given display, recording its display identity
/// for monitor-scoped item selection.
fn open_with_config(display_id: Option<DisplayId>, cx: &mut App) -> bool {
    match cx.open_window(window_options(display_id, cx), move |window, cx| {
        record_window_display(window.window_handle(), display_id);
        cx.new(Dock::new)
    }) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!("Failed to open dock window: {err}");
            false
        }
    }
}

fn open_all_dock_windows(cx: &mut App) -> usize {
    let mut opened = 0;
    let monitors = cx.config().dock.monitors;
    let displays = cx.displays();

    match monitors {
        DockMonitors::PrimaryOnly => {
            if open_with_config(cx.primary_display().map(|display| display.id()), cx) {
                opened += 1;
            }
        }
        DockMonitors::All if displays.is_empty() => {
            if open_with_config(None, cx) {
                opened += 1;
            }
        }
        DockMonitors::All => {
            for display in displays {
                if open_with_config(Some(display.id()), cx) {
                    opened += 1;
                }
            }
        }
    }

    opened
}

/// Rebuild all dock windows using the latest config.
pub fn reload(cx: &mut App) {
    let old_windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter(|handle| handle.downcast::<Dock>().is_some())
        .collect();

    if open_all_dock_windows(cx) == 0 {
        tracing::warn!("Dock reload skipped closing old windows because no new dock window opened");
        return;
    }

    for handle in old_windows {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

/// Toggle whether `item_key` (a desktop file id) is in the pinned list,
/// persisting the change.
pub(crate) fn toggle_pin(item_key: &str, cx: &mut App) {
    let mut config = cx.config().clone();
    toggled_pins(&mut config.dock.pinned, item_key);
    Config::set(config, cx);
}

/// Initialize the dock using the current global config.
pub fn init(cx: &mut App) {
    cx.observe_global::<Config>(|cx| {
        tracing::info!("Config changed; reloading dock windows");
        reload(cx);
    })
    .detach();

    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(100))
            .await;

        tracing::info!("Dock opened");
        cx.update(open_all_dock_windows)
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::picker;
    use super::{
        DOCK_APP_PICKER_WIDTH, DOCK_CONTEXT_MENU_WIDTH, DockHoverEffect,
        dock_bounds_in_compositor_space, dock_context_menu_height, dock_context_menu_size,
        dock_item_count, dock_panel_placement_from_dock_click, dock_window_size,
        dodge_windows_should_hide, focused_window_is_on_other_monitor, geometry_overlaps_dock,
        hidden_strip_size, next_cycle_index, revealed_after_visibility_update, toggled_pins,
        windows_for_monitor,
    };
    use crate::bar::config::BarPosition;
    use gpui::{Bounds, Size, layer_shell::Anchor, point, px};
    use std::path::PathBuf;

    fn app(name: &str, desktop_file: &str) -> services::Application {
        services::Application {
            name: name.to_string(),
            exec: name.to_string(),
            icon: None,
            icon_path: None,
            description: None,
            desktop_file: PathBuf::from(format!("/usr/share/applications/{desktop_file}")),
            startup_wm_class: None,
        }
    }

    fn window(id: &str, monitor: &str) -> services::Window {
        services::Window {
            id: id.to_string(),
            app_id: "test-app".to_string(),
            title: "Test window".to_string(),
            monitor: monitor.to_string(),
            workspace_id: 1,
            is_focused: false,
            is_minimized: false,
            geometry: None,
        }
    }

    #[test]
    fn intelligent_hide_only_hides_for_a_focused_window_on_another_monitor() {
        let focused = window("focused", "HDMI-A-1");

        assert!(!focused_window_is_on_other_monitor(None, Some("HDMI-A-1")));
        assert!(!focused_window_is_on_other_monitor(
            Some(&focused),
            Some("HDMI-A-1")
        ));
        assert!(focused_window_is_on_other_monitor(
            Some(&focused),
            Some("eDP-1")
        ));
        assert!(focused_window_is_on_other_monitor(Some(&focused), None));
    }

    #[test]
    fn dodge_windows_hides_only_for_strictly_overlapping_geometry() {
        let dock = Bounds::new(point(px(100.0), px(900.0)), gpui::size(px(200.0), px(50.0)));

        assert!(geometry_overlaps_dock(
            services::WindowGeometry {
                x: 150,
                y: 850,
                width: 200,
                height: 100,
            },
            dock,
        ));
        assert!(!geometry_overlaps_dock(
            services::WindowGeometry {
                x: 300,
                y: 900,
                width: 100,
                height: 50,
            },
            dock,
        ));
        assert!(!geometry_overlaps_dock(
            services::WindowGeometry {
                x: 100,
                y: 950,
                width: 200,
                height: 50,
            },
            dock,
        ));
    }

    #[test]
    fn dodge_windows_falls_back_to_intelligent_hide_without_geometry_or_bounds() {
        let other_monitor = window("focused", "HDMI-A-1");
        let same_monitor = window("focused", "eDP-1");
        let geometry = services::WindowGeometry {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let same_monitor_with_geometry = services::Window {
            geometry: Some(geometry),
            ..same_monitor.clone()
        };

        assert!(dodge_windows_should_hide(
            Some(&other_monitor),
            Some("eDP-1"),
            None,
        ));
        assert!(!dodge_windows_should_hide(
            Some(&same_monitor),
            Some("eDP-1"),
            None,
        ));
        assert!(!dodge_windows_should_hide(
            Some(&same_monitor_with_geometry),
            Some("eDP-1"),
            None,
        ));
    }

    #[test]
    fn visibility_transition_keeps_a_pointer_revealed_dock_open_until_rehide() {
        assert!(!revealed_after_visibility_update(false, true, true));
        assert!(revealed_after_visibility_update(true, true, true));
        assert!(!revealed_after_visibility_update(true, false, true));
        assert!(revealed_after_visibility_update(true, false, false));
    }

    #[test]
    fn hidden_strip_resizes_along_the_dock_edge() {
        let dock_size = Size::new(px(200.0), px(61.0));

        assert_eq!(
            hidden_strip_size(BarPosition::Bottom, dock_size),
            Size::new(px(200.0), px(6.0))
        );
        assert_eq!(
            hidden_strip_size(BarPosition::Left, dock_size),
            Size::new(px(6.0), px(61.0))
        );
    }

    #[test]
    fn dodge_windows_uses_compositor_coordinates_for_nonzero_monitor_origins() {
        let monitor = services::Monitor {
            name: "HDMI-A-1".to_string(),
            x: 1920,
            y: 100,
            width: 2560,
            height: 1440,
            ..Default::default()
        };
        let dock_bounds = dock_bounds_in_compositor_space(
            BarPosition::Bottom,
            Size::new(px(200.0), px(61.0)),
            &monitor,
        );

        assert_eq!(dock_bounds.origin, point(px(3100.0), px(1471.0)));
        assert!(geometry_overlaps_dock(
            services::WindowGeometry {
                x: 3100,
                y: 1450,
                width: 200,
                height: 100,
            },
            dock_bounds,
        ));
    }

    #[test]
    fn windows_for_monitor_only_keeps_windows_on_that_monitor() {
        let windows = vec![window("1", "eDP-1"), window("2", "HDMI-A-1")];

        let scoped = windows_for_monitor(&windows, Some("HDMI-A-1"));

        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].id, "2");
    }

    #[test]
    fn windows_for_monitor_without_a_resolved_monitor_keeps_all_windows() {
        let windows = vec![window("1", "eDP-1"), window("2", "HDMI-A-1")];

        let scoped = windows_for_monitor(&windows, None);

        assert_eq!(scoped, windows);
    }

    #[test]
    fn dock_item_count_includes_pinned_and_monitor_local_unpinned_items() {
        let windows = vec![window("1", "HDMI-A-1"), window("2", "eDP-1")];
        let apps = vec![app("Pinned", "pinned.desktop")];

        let count = dock_item_count(
            &windows,
            &apps,
            &["pinned.desktop".to_string()],
            Some("HDMI-A-1"),
        );

        assert_eq!(count, 2);
    }

    #[test]
    fn dock_window_size_matches_horizontal_item_extent() {
        let size = dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::None);

        assert_eq!(size, Size::new(px(104.0), px(61.0)));
    }

    #[test]
    fn dock_window_size_reserves_indicator_space_in_vertical_primary_axis() {
        let size = dock_window_size(BarPosition::Left, 40.0, 2, DockHoverEffect::None);

        assert_eq!(size, Size::new(px(56.0), px(114.0)));
    }

    #[test]
    fn dock_window_size_reserves_lift_margin_for_each_orientation() {
        let horizontal = dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::Lift);
        let vertical = dock_window_size(BarPosition::Left, 40.0, 2, DockHoverEffect::Lift);

        assert_eq!(horizontal, Size::new(px(104.0), px(69.0)));
        assert_eq!(vertical, Size::new(px(56.0), px(122.0)));
    }

    #[test]
    fn dock_window_size_reserves_magnify_lift_for_each_orientation() {
        let horizontal =
            dock_window_size(BarPosition::Bottom, 40.0, 2, DockHoverEffect::MagnifyLift);
        let vertical = dock_window_size(BarPosition::Left, 40.0, 2, DockHoverEffect::MagnifyLift);

        assert_eq!(horizontal, Size::new(px(116.0), px(81.0)));
        assert_eq!(vertical, Size::new(px(68.0), px(134.0)));
    }

    #[test]
    fn next_cycle_index_advances_and_wraps_grouped_windows() {
        assert_eq!(next_cycle_index(None, 2), Some(0));
        assert_eq!(next_cycle_index(Some(0), 2), Some(1));
        assert_eq!(next_cycle_index(Some(1), 2), Some(0));
    }

    #[test]
    fn toggled_pins_adds_missing_items_and_removes_existing_items() {
        let mut pinned = vec!["firefox.desktop".to_string()];

        toggled_pins(&mut pinned, "kitty.desktop");
        assert_eq!(pinned, ["firefox.desktop", "kitty.desktop"]);

        toggled_pins(&mut pinned, "firefox.desktop");
        assert_eq!(pinned, ["kitty.desktop"]);
    }

    #[test]
    fn dock_context_menu_height_reserves_one_action_row() {
        assert_eq!(dock_context_menu_height(1), 62.0);
    }

    #[test]
    fn dock_context_menu_height_reserves_two_action_rows() {
        assert_eq!(dock_context_menu_height(2), 104.0);
        assert_eq!(
            dock_context_menu_size(2),
            Size::new(px(DOCK_CONTEXT_MENU_WIDTH), px(104.0))
        );
    }

    #[test]
    fn dock_context_menu_bottom_uses_the_dock_local_click_position() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Bottom,
            point(px(78.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(placement.margin, (627.0, 0.0, 0.0, 446.0));
    }

    #[test]
    fn dock_context_menu_top_opens_below_the_clicked_icon() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Top,
            point(px(26.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.margin, (69.0, 0.0, 0.0, 394.0));
    }

    #[test]
    fn dock_context_menu_left_opens_to_the_right_of_the_clicked_icon() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Left,
            point(px(25.0), px(90.0)),
            Size::new(px(56.0), px(114.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.margin, (381.0, 0.0, 0.0, 64.0));
    }

    #[test]
    fn dock_context_menu_right_opens_to_the_left_of_the_clicked_icon() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Right,
            point(px(20.0), px(30.0)),
            Size::new(px(56.0), px(114.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.margin, (321.0, 0.0, 0.0, 776.0));
    }

    #[test]
    fn dock_context_menu_clamps_the_dock_local_position_to_usable_bounds() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Bottom,
            point(px(130.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(300.0), px(200.0))),
            Bounds::new(point(px(20.0), px(10.0)), Size::new(px(260.0), px(170.0))),
        );

        assert_eq!(placement.margin, (27.0, 0.0, 0.0, 120.0));
    }

    #[test]
    fn dock_app_picker_bottom_opens_above_the_add_control() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Bottom,
            point(px(78.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            Size::new(
                px(DOCK_APP_PICKER_WIDTH),
                px(picker::DOCK_APP_PICKER_HEIGHT),
            ),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(placement.margin, (451.0, 0.0, 0.0, 406.0));
    }

    #[test]
    fn dock_app_picker_top_opens_below_the_add_control() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Top,
            point(px(26.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            Size::new(
                px(DOCK_APP_PICKER_WIDTH),
                px(picker::DOCK_APP_PICKER_HEIGHT),
            ),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(placement.margin, (69.0, 0.0, 0.0, 354.0));
    }

    #[test]
    fn dock_app_picker_left_opens_to_the_right_of_the_add_control() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Left,
            point(px(25.0), px(90.0)),
            Size::new(px(56.0), px(114.0)),
            Size::new(
                px(DOCK_APP_PICKER_WIDTH),
                px(picker::DOCK_APP_PICKER_HEIGHT),
            ),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(placement.margin, (293.0, 0.0, 0.0, 64.0));
    }

    #[test]
    fn dock_app_picker_right_opens_to_the_left_of_the_add_control() {
        let placement = dock_panel_placement_from_dock_click(
            BarPosition::Right,
            point(px(20.0), px(30.0)),
            Size::new(px(56.0), px(114.0)),
            Size::new(
                px(DOCK_APP_PICKER_WIDTH),
                px(picker::DOCK_APP_PICKER_HEIGHT),
            ),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::TOP | Anchor::LEFT);
        assert_eq!(placement.margin, (233.0, 0.0, 0.0, 696.0));
    }
}
