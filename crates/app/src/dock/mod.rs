//! Dock UI: pinned + running app icons, shown as a standalone layer-shell
//! window per configured monitor.

pub mod config;
mod context_menu;
mod item;
mod picker;
mod placement;
mod visibility;
mod window;

pub use config::{DockConfig, DockHoverEffect, DockMonitors, DockVisibility};
pub use window::{init, reload};

use gpui::{AnyElement, App, Context, Render, Size, Window, div, img, prelude::*, px};
use std::collections::HashMap;
use ui::{ActiveTheme, Radius, Spacing, TextSize};

use crate::config::{ActiveConfig, ConfigSignals, State};
use crate::state::{AppState, display_id_for_window, watch};
use item::{DockItem, build_dock_items};
use placement::*;
use visibility::*;

const INDICATOR_ROW_HEIGHT: f32 = 5.0;
pub(super) const DOCK_CONTEXT_MENU_WIDTH: f32 = 160.0;
const DOCK_APP_PICKER_WIDTH: f32 = 240.0;
pub(super) const DOCK_CONTEXT_MENU_ROW_HEIGHT: f32 = 40.0;
pub(super) const DOCK_CONTEXT_MENU_VERTICAL_PADDING: f32 = Spacing::XSmall.value();
pub(super) const DOCK_CONTEXT_MENU_GAP: f32 = 2.0;
pub(super) const DOCK_CONTEXT_MENU_BORDER_WIDTH: f32 = 1.0;
// GPUI can reserve more vertical space than the nominal row height while it
// lays out text, particularly in layer-shell windows.
const DOCK_CONTEXT_MENU_TEXT_LAYOUT_HEADROOM: f32 = 12.0;
const DOCK_HIDDEN_STRIP_SIZE: f32 = 6.0;
const DOCK_REVEAL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(750);
const DOCK_INITIAL_HIDE_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

/// The right-click menu or the app picker is open.
fn dock_menu_is_open(cx: &App) -> bool {
    crate::windows::ShellWindows::active_panel_id(cx).is_some_and(|id| id.starts_with("dock-"))
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

fn activation_index(windows: &[services::Window], previous: Option<usize>) -> Option<usize> {
    match windows {
        [] => None,
        [window] if window.is_focused => None,
        [_] => Some(0),
        windows => Some(previous.map_or_else(
            || {
                windows
                    .iter()
                    .position(|window| window.is_focused)
                    .map_or(0, |index| (index + 1) % windows.len())
            },
            |index| (index + 1) % windows.len(),
        )),
    }
}

fn dock_item_can_be_pinned(item: &DockItem) -> bool {
    item.exec.is_some()
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
    let primary_extent =
        item_count * (icon_size + Spacing::Medium.value()) + Spacing::Medium.value();
    let cross_extent = icon_size
        + if position.is_vertical() {
            0.0
        } else {
            INDICATOR_ROW_HEIGHT
        }
        + Spacing::Medium.value() * 2.0;
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

/// The dock's own view, rendered in a standalone layer-shell window.
pub(super) struct Dock {
    compositor: services::CompositorSubscriber,
    state: services::CompositorState,
    cycle_index: HashMap<String, usize>,
    revealed: bool,
    hidden_by_rule: bool,
    pointer_inside: bool,
    /// A rehide timer is waiting for the dock's menu to close.
    menu_hold: bool,
    reveal_generation: u64,
    hide_generation: u64,
}

impl Dock {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            cx.notify();
        });
        let signals = ConfigSignals::global(cx);
        let (dock, pinned) = (signals.dock.signal_cloned(), signals.pinned.signal_cloned());
        watch(cx, dock, |_, _, cx| cx.notify());
        watch(cx, pinned, |_, _, cx| cx.notify());

        Self {
            compositor,
            state,
            cycle_index: HashMap::new(),
            revealed: true,
            hidden_by_rule: false,
            pointer_inside: false,
            menu_hold: false,
            reveal_generation: 0,
            hide_generation: 0,
        }
    }

    fn current_monitor_name(&self, window: &Window, cx: &App) -> Option<String> {
        monitor_name_for_display(display_id_for_window(window, cx), &self.state, cx)
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
    fn hide_behavior(
        &self,
        window: &Window,
        dock_size: Size<gpui::Pixels>,
        cx: &App,
    ) -> DockHideBehavior {
        let monitor_name = self.current_monitor_name(window, cx);
        let focused_window = self.focused_window();

        // Our own window holds the keyboard; the windows underneath did not move.
        if focused_window.is_none() && crate::windows::ShellWindows::holds_keyboard_focus(cx) {
            return if self.hidden_by_rule {
                DockHideBehavior::Immediate
            } else {
                DockHideBehavior::Visible
            };
        }

        let intelligent_hide =
            || focused_window_is_on_other_monitor(focused_window, monitor_name.as_deref());

        match cx.config().dock.visibility {
            config::DockVisibility::AlwaysVisible => DockHideBehavior::Visible,
            config::DockVisibility::IntelligentHide => {
                if intelligent_hide() {
                    DockHideBehavior::Delayed
                } else {
                    DockHideBehavior::Visible
                }
            }
            config::DockVisibility::DodgeWindows => dodge_windows_hide_behavior(
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
        self.hide_generation += 1;
        self.reveal_generation += 1;
        let generation = self.reveal_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DOCK_REVEAL_TIMEOUT).await;
            let _ = this.update(cx, |this, cx| {
                if this.reveal_generation != generation {
                    return;
                }
                if this.pointer_inside {
                    this.menu_hold = false;
                    return;
                }
                if dock_menu_is_open(cx) {
                    this.schedule_rehide(cx);
                    return;
                }
                this.menu_hold = false;
                if this.hidden_by_rule {
                    this.revealed = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn schedule_initial_hide(&mut self, cx: &mut Context<Self>) {
        self.hide_generation += 1;
        let generation = self.hide_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(DOCK_INITIAL_HIDE_DELAY)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.hide_generation != generation {
                    return;
                }
                if this.pointer_inside {
                    return;
                }
                if dock_menu_is_open(cx) {
                    this.schedule_initial_hide(cx);
                    return;
                }
                if this.hidden_by_rule {
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
            State::pinned(cx),
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

        let Some(index) = activation_index(&item.windows, self.cycle_index.get(&item.key).copied())
        else {
            return;
        };
        if item.windows.len() > 1 {
            self.cycle_index.insert(item.key.clone(), index);
        }

        let window_id = item.windows[index].id.clone();
        self.compositor
            .dispatch(services::CompositorCommand::FocusWindow(window_id));
    }

    fn render_item(&self, item: &DockItem, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().dock;
        let icon_size = config.icon_size;
        let hover_effect = config.hover_effect;
        let accent = theme.colors.accent;

        let mut element = div()
            .relative()
            .size(px(icon_size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(Radius::Medium.pixels());

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

        let window_count = item.windows.len();
        let indicator_width = if window_count > 1 { 20.0 } else { 6.0 };
        let inactive_indicator = theme.colors.text_muted;
        let indicators = item
            .windows
            .iter()
            .map(|window| {
                div()
                    .flex_1()
                    .h(px(3.0))
                    .rounded(px(1.5))
                    .bg(if window.is_focused {
                        accent
                    } else {
                        inactive_indicator
                    })
            })
            .collect::<Vec<_>>();

        let icon_element = element
            .when_some(item.icon_path.clone(), |element, path| {
                element.child(img(path).size(px(icon_size * 0.75)))
            })
            .when(item.icon_path.is_none(), |element| {
                element.child(
                    div()
                        .text_size(TextSize::Medium.rems())
                        .text_color(theme.colors.text)
                        .child(item.name.chars().take(1).collect::<String>()),
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
                    if !dock_item_can_be_pinned(&item) {
                        return;
                    }
                    let config = cx.config().dock.clone();
                    let action_count = 2;
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
                    let panel_config = crate::windows::PanelConfig {
                        size: crate::windows::PanelSize::FitHeight {
                            width: DOCK_CONTEXT_MENU_WIDTH,
                            max_height: panel_height * 2.0,
                        },
                        anchor: placement.anchor,
                        margin: placement.margin,
                        namespace: "dock-context-menu".to_string(),
                    };
                    let item_key = item.key.clone();
                    let panel_id = format!("dock-context-{item_key}");
                    let is_pinned = item.is_pinned;
                    let exec = item.exec.clone();
                    let app_name = item.name.clone();
                    let icon_path = item.icon_path.clone();
                    let display_id = display_id_for_window(window, cx);
                    crate::windows::ShellWindows::toggle_panel_on_display(
                        cx,
                        &panel_id,
                        panel_config,
                        display_id,
                        move |cx| {
                            context_menu::DockContextMenu::new(
                                item_key, is_pinned, exec, app_name, icon_path, cx,
                            )
                        },
                    );
                })
            })
            .child(icon_element)
            .when(item.is_running(), |element| {
                element.child(
                    div()
                        .w(px(indicator_width))
                        .h(px(3.0))
                        .flex()
                        .gap(px(2.0))
                        .children(indicators),
                )
            })
            .into_any_element()
    }
}

impl Render for Dock {
    #[allow(refining_impl_trait)]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        window.set_rem_size(cx.theme().font_size);
        let position = cx.config().dock.position;
        let items = self.items(window, cx);
        let dock_size = dock_window_size(
            position,
            cx.config().dock.icon_size,
            items.len() + 1,
            cx.config().dock.hover_effect,
        );
        let hide_behavior = self.hide_behavior(window, dock_size, cx);
        let is_hidden_by_rule = hide_behavior.hides();
        match initial_hide_action(self.hidden_by_rule, hide_behavior) {
            InitialHideAction::Reveal => {
                self.hide_generation += 1;
                self.revealed = true;
            }
            InitialHideAction::Keep => {
                self.revealed = revealed_after_visibility_update(
                    self.hidden_by_rule,
                    self.revealed,
                    is_hidden_by_rule,
                );
            }
            InitialHideAction::HideNow => self.revealed = false,
            InitialHideAction::ScheduleDelayedHide => self.schedule_initial_hide(cx),
        }
        self.hidden_by_rule = is_hidden_by_rule;

        if self.pointer_inside {
            self.revealed = true;
        }

        // Stay revealed while a dock menu is open; the timer re-arms until it closes.
        if dock_menu_is_open(cx) {
            self.revealed = true;
            if !self.menu_hold {
                self.menu_hold = true;
                self.schedule_rehide(cx);
            }
        }

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
                    this.pointer_inside = true;
                    this.hide_generation += 1;
                    this.reveal_generation += 1;
                    this.revealed = true;
                    cx.notify();
                }))
                .into_any_element();
        }

        let background = cx.theme().colors.background;
        let border = cx.theme().colors.border_variant;
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
            .gap(Spacing::Medium.pixels())
            .px(Spacing::Medium.pixels())
            .py(Spacing::Medium.pixels())
            .rounded(Radius::Large.pixels())
            .bg(background)
            .border_1()
            .border_color(border)
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                this.pointer_inside = *hovered;
                if *hovered {
                    this.hide_generation += 1;
                    this.reveal_generation += 1;
                } else if this.hidden_by_rule {
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
                    .rounded(Radius::Medium.pixels())
                    .cursor_pointer()
                    .text_color(cx.theme().colors.text_muted)
                    .hover(|style| style.text_color(cx.theme().colors.text))
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
                            let panel_config = crate::windows::PanelConfig {
                                size: crate::windows::PanelSize::Fixed {
                                    width: DOCK_APP_PICKER_WIDTH,
                                    height: picker::DOCK_APP_PICKER_HEIGHT,
                                },
                                anchor: placement.anchor,
                                margin: placement.margin,
                                namespace: "dock-app-picker".to_string(),
                            };
                            let display_id = display_id_for_window(window, cx);
                            crate::windows::ShellWindows::toggle_panel_on_display(
                                cx,
                                "dock-app-picker",
                                panel_config,
                                display_id,
                                picker::DockAppPicker::new,
                            );
                        }),
                    )
                    .child("+"),
            )
            .into_any_element()
    }
}

/// Toggle whether `item_key` (a desktop file id) is in the pinned list,
/// persisting the change.
pub(crate) fn toggle_pin(item_key: &str, cx: &mut App) {
    let mut pinned = State::pinned(cx).to_vec();
    toggled_pins(&mut pinned, item_key);
    State::set_pinned(pinned, cx);
}

#[cfg(test)]
mod tests {
    use super::picker;
    use super::{
        DOCK_APP_PICKER_WIDTH, DOCK_CONTEXT_MENU_WIDTH, DockHideBehavior, DockHoverEffect,
        InitialHideAction, activation_index, dock_bounds_in_compositor_space,
        dock_context_menu_height, dock_context_menu_size, dock_item_can_be_pinned, dock_item_count,
        dock_panel_placement_from_dock_click, dock_window_size, dodge_windows_hide_behavior,
        focused_window_is_on_other_monitor, geometry_overlaps_dock, hidden_strip_size,
        initial_hide_action, revealed_after_visibility_update, toggled_pins, windows_for_monitor,
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
            ..Default::default()
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
    fn dodge_windows_falls_back_to_delayed_intelligent_hide_without_geometry_or_bounds() {
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

        assert_eq!(
            dodge_windows_hide_behavior(Some(&other_monitor), Some("eDP-1"), None,),
            DockHideBehavior::Delayed
        );
        assert_eq!(
            dodge_windows_hide_behavior(Some(&same_monitor), Some("eDP-1"), None,),
            DockHideBehavior::Visible
        );
        assert_eq!(
            dodge_windows_hide_behavior(Some(&same_monitor_with_geometry), Some("eDP-1"), None,),
            DockHideBehavior::Visible
        );
    }

    #[test]
    fn visibility_transition_keeps_a_pointer_revealed_dock_open_until_rehide() {
        assert!(!revealed_after_visibility_update(false, true, true));
        assert!(revealed_after_visibility_update(true, true, true));
        assert!(!revealed_after_visibility_update(true, false, true));
        assert!(revealed_after_visibility_update(true, false, false));
    }

    #[test]
    fn initial_intelligent_hide_is_delayed_but_dodge_overlap_hides_immediately() {
        assert_eq!(
            initial_hide_action(false, DockHideBehavior::Delayed),
            InitialHideAction::ScheduleDelayedHide
        );
        assert_eq!(
            initial_hide_action(false, DockHideBehavior::Immediate),
            InitialHideAction::HideNow
        );
        assert_eq!(
            initial_hide_action(true, DockHideBehavior::Delayed),
            InitialHideAction::Keep
        );
        assert_eq!(
            initial_hide_action(true, DockHideBehavior::Visible),
            InitialHideAction::Reveal
        );
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
    fn activation_noops_for_the_already_focused_single_window() {
        let focused = services::Window {
            is_focused: true,
            ..window("1", "eDP-1")
        };

        assert_eq!(activation_index(&[focused], None), None);
    }

    #[test]
    fn activation_starts_after_the_focused_group_window_then_cycles() {
        let windows = vec![
            window("1", "eDP-1"),
            services::Window {
                is_focused: true,
                ..window("2", "eDP-1")
            },
            window("3", "eDP-1"),
        ];

        assert_eq!(activation_index(&windows, None), Some(2));
        assert_eq!(activation_index(&windows, Some(2)), Some(0));
        assert_eq!(activation_index(&windows, Some(0)), Some(1));
    }

    #[test]
    fn only_launchable_items_can_be_pinned() {
        let launchable = super::DockItem {
            key: "kitty.desktop".to_string(),
            name: "Kitty".to_string(),
            icon_path: None,
            is_pinned: false,
            windows: Vec::new(),
            exec: Some("kitty".to_string()),
        };
        let unmatched = super::DockItem {
            exec: None,
            ..launchable.clone()
        };

        assert!(dock_item_can_be_pinned(&launchable));
        assert!(!dock_item_can_be_pinned(&unmatched));
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

        assert_eq!(placement.anchor, Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(placement.margin, (0.0, 0.0, 69.0, 446.0));
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

        assert_eq!(placement.anchor, Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(placement.margin, (0.0, 0.0, 69.0, 120.0));
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

        assert_eq!(placement.anchor, Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(placement.margin, (0.0, 0.0, 69.0, 406.0));
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
