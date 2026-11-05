//! Dock UI: pinned + running app icons, shown as a standalone layer-shell
//! window per configured monitor.

pub mod config;
mod context_menu;
mod item;
mod placement;
mod visibility;
mod window;

pub use config::{DockConfig, DockHoverEffect, DockMonitors, DockStyle, DockVisibility};
pub use window::{init, reload};

use gpui::{
    AnimationExt, AnimationPhase, AnyElement, App, Context, DisplayId, ElementId,
    InteractiveElement, Render, Size, Styled, Window, div, img, prelude::*, px, relative,
};
use std::collections::HashMap;
use ui::patterns::surface_radius;
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, Motion, Radius, Spacing, SpringConfigExt,
    TextSize, mix, nested_radius,
};

use crate::config::{ActiveConfig, ConfigSignals, State};
use crate::state::{AppState, display_id_for_window, watch};
use item::{DockItem, build_dock_items};
use placement::*;
use visibility::*;

const INDICATOR_ROW_HEIGHT: f32 = 5.0;
pub(super) const DOCK_CONTEXT_MENU_WIDTH: f32 = 220.0;
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

/// The right-click menu is open.
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

/// The dock window's size on `display`: fitted to its `item_count` apps
/// and the launcher button, or, stretched, the whole edge less the gap it
/// keeps at both ends.
fn dock_extent(
    config: &DockConfig,
    item_count: usize,
    display: Size<gpui::Pixels>,
) -> Size<gpui::Pixels> {
    let fitted = dock_window_size(
        config.position,
        config.icon_size,
        item_count.max(1) + usize::from(config.launcher),
        config.hover_effect,
    );
    if !config.stretch {
        return fitted;
    }
    let length = |extent: gpui::Pixels| (extent - px(config.edge_gap() * 2.)).max(px(1.));
    if config.position.is_vertical() {
        Size::new(fitted.width, length(display.height))
    } else {
        Size::new(length(display.width), fitted.height)
    }
}

/// The dock's extent across its edge: what it reserves when asked to.
fn dock_thickness(config: &DockConfig, size: Size<gpui::Pixels>) -> gpui::Pixels {
    if config.position.is_vertical() {
        size.width
    } else {
        size.height
    }
}

/// The dock's extent across its edge from its config alone, for the frame
/// it can be a side of. It does not depend on how many apps are shown.
pub fn thickness(config: &DockConfig) -> f32 {
    let size = dock_window_size(config.position, config.icon_size, 1, config.hover_effect);
    f32::from(dock_thickness(config, size))
}

/// The dock's own view, rendered in a standalone layer-shell window.
pub(super) struct Dock {
    display_id: Option<DisplayId>,
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
    pub(super) fn new(display_id: Option<DisplayId>, cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            this.update_visibility(cx);
            cx.notify();
        });
        let signals = ConfigSignals::global(cx);
        let (dock, pinned) = (signals.dock.signal_cloned(), signals.pinned.signal_cloned());
        watch(cx, dock, |this, _, cx| {
            this.update_visibility(cx);
            cx.notify();
        });
        watch(cx, pinned, |this, _, cx| {
            this.update_visibility(cx);
            cx.notify();
        });

        let mut dock = Self {
            display_id,
            compositor,
            state,
            cycle_index: HashMap::new(),
            revealed: true,
            hidden_by_rule: false,
            pointer_inside: false,
            menu_hold: false,
            reveal_generation: 0,
            hide_generation: 0,
        };
        dock.update_visibility(cx);
        dock
    }

    fn current_monitor_name(&self, cx: &App) -> Option<String> {
        monitor_name_for_display(self.display_id, &self.state, cx)
    }

    fn dock_size(&self, item_count: usize, cx: &App) -> Size<gpui::Pixels> {
        dock_extent(
            &cx.config().dock,
            item_count,
            crate::state::display_extent(self.display_id, cx),
        )
    }

    /// Re-run the hide rule against the latest compositor state and config.
    fn update_visibility(&mut self, cx: &mut Context<Self>) {
        let dock_size = self.dock_size(self.items(cx).len(), cx);
        let hide_behavior = self.hide_behavior(dock_size, cx);
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
    fn hide_behavior(&self, dock_size: Size<gpui::Pixels>, cx: &App) -> DockHideBehavior {
        let monitor_name = self.current_monitor_name(cx);
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

        match cx.config().dock.effective_visibility() {
            config::DockVisibility::AlwaysVisible => DockHideBehavior::Visible,
            config::DockVisibility::IntelligentHide => {
                if intelligent_hide() {
                    DockHideBehavior::Delayed
                } else {
                    DockHideBehavior::Visible
                }
            }
            config::DockVisibility::DodgeWindows => dodge_windows_hide_behavior(
                &self.state.windows,
                monitor_name.as_deref().and_then(|monitor_name| {
                    self.state
                        .monitors
                        .iter()
                        .find(|monitor| monitor.name == monitor_name)
                }),
                cx.config().dock.geometry(),
                dock_size,
            ),
        }
    }

    fn set_pointer_inside(&mut self, inside: bool, cx: &mut Context<Self>) {
        if inside == self.pointer_inside {
            return;
        }
        self.pointer_inside = inside;
        if inside {
            self.hide_generation += 1;
            self.reveal_generation += 1;
        } else if self.hidden_by_rule {
            self.schedule_rehide(cx);
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

    fn items(&self, cx: &App) -> Vec<DockItem> {
        let apps = AppState::applications(cx).all();
        dock_items(
            &self.state.windows,
            apps,
            State::pinned(cx),
            self.current_monitor_name(cx).as_deref(),
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

    fn render_item(
        &self,
        item: &DockItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let config = &cx.config().dock;
        let icon_size = config.icon_size;
        let hover_effect = config.hover_effect;
        let position = config.position;
        let accent = theme.colors.accent;
        let text = theme.colors.text;
        let inactive_indicator = theme.colors.text_muted;

        // Hover is remembered per item so the effect can spring in and out
        // rather than switch with the pointer.
        let hovered = window.use_keyed_state(
            (ElementId::from("dock-hover"), item.key.clone()),
            cx,
            |_, _| false,
        );
        let is_hovered = *hovered.read(cx);

        let window_count = item.windows.len();
        let indicator_width = if window_count > 1 { 20.0 } else { 6.0 };
        let indicators = item
            .windows
            .iter()
            .map(|window| {
                div()
                    .flex_1()
                    .h(px(3.0))
                    .rounded(Radius::Full.pixels())
                    .with_spring(
                        (ElementId::from("dock-indicator"), window.id.clone()),
                        Motion::GENTLE.to(window.is_focused),
                        move |dot, phase| dot.bg(mix(phase, inactive_indicator, accent)),
                    )
            })
            .collect::<Vec<_>>();

        let icon_path = item.icon_path.clone();
        let initial = item.name.chars().take(1).collect::<String>();
        let icon_element = icon_cell(icon_size).with_spring(
            (ElementId::from("dock-hover-effect"), item.key.clone()),
            Motion::LIVELY.to(is_hovered),
            move |cell, phase| {
                let cell = with_hover_effect(cell, hover_effect, phase, icon_size, position);
                // Sized by the cell, so a magnified cell magnifies the icon
                // with it.
                let cell = match icon_path {
                    Some(path) => cell.child(img(path).size(relative(0.75))),
                    None => cell.child(
                        div()
                            .text_size(TextSize::Medium.rems())
                            .text_color(text)
                            .child(initial),
                    ),
                };
                cell.when(hover_effect == config::DockHoverEffect::Glow, |cell| {
                    cell.child(glow_ring(accent, phase))
                })
            },
        );

        let item_key = item.key.clone();
        div()
            .id(item.key.clone())
            .cursor_pointer()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .on_hover(track_hover(hovered))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, _event, _window, cx| {
                    if let Some(item) = this.items(cx).into_iter().find(|item| item.key == item_key)
                    {
                        this.activate(&item, cx);
                    }
                }),
            )
            .on_mouse_down(gpui::MouseButton::Right, {
                let item = item.clone();
                cx.listener(move |_this, event, window, cx| {
                    let rows = context_menu::row_count(&item);
                    let panel_height = dock_context_menu_height(rows);
                    let placement = dock_panel_placement_from_event(
                        cx.config().dock.geometry(),
                        event,
                        window,
                        cx,
                        window.viewport_size(),
                        dock_context_menu_size(rows),
                    );
                    let panel_config = crate::windows::PanelConfig {
                        size: crate::windows::PanelSize::FitHeight {
                            width: DOCK_CONTEXT_MENU_WIDTH,
                            max_height: panel_height,
                        },
                        anchor: placement.anchor,
                        margin: placement.margin,
                        namespace: "dock-context-menu".to_string(),
                    };
                    let panel_id = format!("dock-context-{}", item.key);
                    let item = item.clone();
                    let display_id = display_id_for_window(window, cx);
                    crate::windows::ShellWindows::toggle_panel_on_display(
                        cx,
                        &panel_id,
                        panel_config,
                        display_id,
                        move |cx| context_menu::DockContextMenu::new(item, cx),
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
        let items = self.items(cx);
        let dock_size = self.dock_size(items.len(), cx);

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

        let background = cx.theme().surface(cx.config().dock.opacity);
        let border = cx.theme().colors.border_variant;
        let (style, stretch, align, launcher) = {
            let dock = &cx.config().dock;
            (dock.style, dock.stretch, dock.align, dock.launcher)
        };
        let is_vertical = position.is_vertical();
        // A stretched dock's length is the compositor's to set between the
        // anchors; asking for it again every frame would only argue with it.
        let wanted = if stretch {
            let viewport = window.viewport_size();
            if is_vertical {
                Size::new(dock_size.width, viewport.height)
            } else {
                Size::new(viewport.width, dock_size.height)
            }
        } else {
            dock_size
        };
        if window.viewport_size() != wanted {
            window.resize(wanted);
        }
        let elements: Vec<AnyElement> = items
            .iter()
            .map(|item| self.render_item(item, window, cx))
            .collect();

        div()
            .id("dock")
            .flex()
            .when(is_vertical, |element| element.flex_col())
            .when(stretch && is_vertical, |element| element.h_full())
            .when(stretch && !is_vertical, |element| element.w_full())
            .items_center()
            .map(|element| match align {
                config::DockAlign::Start => element.justify_start(),
                config::DockAlign::Center => element.justify_center(),
                config::DockAlign::End => element.justify_end(),
            })
            .gap(Spacing::Medium.pixels())
            .px(Spacing::Medium.pixels())
            .py(Spacing::Medium.pixels())
            .map(|element| dock_chrome(element, position, style, stretch))
            .bg(background)
            .border_color(border)
            // gpui reports a hover at the window origin on first paint, before any
            // pointer entered; only a real pointer counts, and a move fills in the enter.
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.set_pointer_inside(*hovered && window.is_window_hovered(), cx);
            }))
            .on_mouse_move(cx.listener(|this, _event, _window, cx| {
                this.set_pointer_inside(true, cx);
            }))
            .when(launcher, |element| {
                element.child(self.render_launcher(window, cx))
            })
            .children(elements)
            .into_any_element()
    }
}

impl Dock {
    /// The launcher button: an item-sized cell with the bar's launcher icon
    /// in the accent colour, in the same column an item uses so the icons
    /// line up.
    fn render_launcher(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let dock = &cx.config().dock;
        let (icon_size, hover_effect, position) =
            (dock.icon_size, dock.hover_effect, dock.position);
        let accent = cx.theme().colors.accent;
        let icon = crate::icons::source_or(
            cx.config().bar.modules.launcher_btn.icon.as_ref(),
            IconName::Layers,
        );
        let hovered = window.use_keyed_state("dock-launcher-hover", cx, |_, _| false);
        let is_hovered = *hovered.read(cx);
        let cell = icon_cell(icon_size)
            .id("dock-launcher")
            .cursor_pointer()
            .on_hover(track_hover(hovered))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|_this, _event, _window, cx| crate::launcher::toggle(None, cx)),
            )
            .with_spring(
                "dock-launcher-hover-effect",
                Motion::LIVELY.to(is_hovered),
                move |cell, phase| {
                    with_hover_effect(cell, hover_effect, phase, icon_size, position)
                        .child(
                            div()
                                .size(relative(0.6))
                                .child(Icon::new(icon).size(IconSize::Fill).color(Color::Accent)),
                        )
                        .when(hover_effect == config::DockHoverEffect::Glow, |cell| {
                            cell.child(glow_ring(accent, phase))
                        })
                },
            );
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.0))
            .child(cell)
            .into_any_element()
    }
}

/// The square an icon sits in.
fn icon_cell(icon_size: f32) -> gpui::Div {
    div()
        .relative()
        .size(px(icon_size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(icon_cell_radius())
}

fn icon_cell_radius() -> gpui::Pixels {
    nested_radius(surface_radius(), Spacing::Medium.pixels())
}

/// Writes the pointer's presence into `hovered`, which the owning view
/// observes, so the cell re-renders and its spring retargets.
fn track_hover(hovered: gpui::Entity<bool>) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
    move |over, _, cx| {
        hovered.update(cx, |state, cx| {
            if *state != *over {
                *state = *over;
                cx.notify();
            }
        });
    }
}

/// The glow effect's ring at `phase` of the way in: an overlay, so it never
/// squeezes the icon it frames.
fn glow_ring(accent: gpui::Hsla, phase: AnimationPhase) -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .rounded(icon_cell_radius())
        .border_2()
        .border_color(accent.opacity(phase.0.clamp(0.0, 1.0)))
}

/// How far a lifted icon rises off the dock's edge.
const LIFT: f32 = 8.0;
/// How much a magnified icon grows.
const MAGNIFY: f32 = 1.3;

/// The hover style of an icon cell at `phase` of the way in (0 at rest, 1
/// hovered). A lift is an offset away from the screen edge, so it reads the
/// same on every side and moves nothing else; a magnified cell grows, and
/// its icon with it. Clamped: the dock reserves exactly this much room.
fn with_hover_effect<E: Styled>(
    element: E,
    effect: config::DockHoverEffect,
    phase: AnimationPhase,
    icon_size: f32,
    position: crate::bar::config::BarPosition,
) -> E {
    use crate::bar::config::BarPosition;
    let lift = |element: E| {
        let rise = phase.interpolate_clamped(px(0.0), px(LIFT));
        match position {
            BarPosition::Bottom => element.bottom(rise),
            BarPosition::Top => element.top(rise),
            BarPosition::Left => element.left(rise),
            BarPosition::Right => element.right(rise),
        }
    };
    let magnify = |element: E| {
        element.size(phase.interpolate_clamped(px(icon_size), px(icon_size * MAGNIFY)))
    };
    match effect {
        config::DockHoverEffect::None | config::DockHoverEffect::Glow => element,
        config::DockHoverEffect::Lift => lift(element),
        config::DockHoverEffect::Magnify => magnify(element),
        config::DockHoverEffect::MagnifyLift => lift(magnify(element)),
    }
}

/// Corners and hairline for the dock's style: a floating dock is a card, an
/// attached one rounds and outlines only the sides facing the screen, and
/// stretched along its edge it is a strip with one hairline.
fn dock_chrome(
    element: gpui::Stateful<gpui::Div>,
    position: crate::bar::config::BarPosition,
    style: config::DockStyle,
    stretch: bool,
) -> gpui::Stateful<gpui::Div> {
    use crate::bar::config::BarPosition;
    let radius = surface_radius();
    match style {
        config::DockStyle::Floating => element.rounded(radius).border_1(),
        config::DockStyle::Attached => {
            let element = match position {
                BarPosition::Top => element.border_b_1(),
                BarPosition::Bottom => element.border_t_1(),
                BarPosition::Left => element.border_r_1(),
                BarPosition::Right => element.border_l_1(),
            };
            if stretch {
                return element;
            }
            match position {
                BarPosition::Top => element.rounded_b(radius).border_l_1().border_r_1(),
                BarPosition::Bottom => element.rounded_t(radius).border_l_1().border_r_1(),
                BarPosition::Left => element.rounded_r(radius).border_t_1().border_b_1(),
                BarPosition::Right => element.rounded_l(radius).border_t_1().border_b_1(),
            }
        }
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
    use super::config::{DockAlign, DockConfig, DockGeometry, DockStyle};
    use super::{
        DOCK_CONTEXT_MENU_WIDTH, DockHideBehavior, DockHoverEffect, InitialHideAction,
        activation_index, dock_bounds_in_compositor_space, dock_context_menu_height,
        dock_context_menu_size, dock_extent, dock_item_can_be_pinned, dock_item_count,
        dock_panel_placement_from_dock_click, dock_thickness, dock_window_size,
        dodge_windows_hide_behavior, focused_window_is_on_other_monitor, geometry_overlaps_dock,
        hidden_strip_size, initial_hide_action, revealed_after_visibility_update, toggled_pins,
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

    /// A 1920x1080 monitor at the origin showing workspace 1.
    fn monitor() -> services::Monitor {
        services::Monitor {
            name: "eDP-1".to_string(),
            active_workspace_id: 1,
            width: 1920,
            height: 1080,
            scale: 1.0,
            ..Default::default()
        }
    }

    fn placed(
        window: services::Window,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> services::Window {
        services::Window {
            geometry: Some(services::WindowGeometry {
                x,
                y,
                width,
                height,
            }),
            ..window
        }
    }

    /// The floating inset the tests place the dock at.
    const GAP: f32 = 8.0;
    fn geometry(position: BarPosition) -> DockGeometry {
        DockGeometry {
            position,
            gap: GAP,
            align: DockAlign::Center,
        }
    }
    const DOCK: Size<gpui::Pixels> = Size {
        width: px(200.0),
        height: px(61.0),
    };

    #[test]
    fn dodge_windows_hides_for_any_window_covering_the_dock_on_the_current_workspace() {
        let monitor = monitor();
        let dodge = |windows: &[services::Window]| {
            dodge_windows_hide_behavior(
                windows,
                Some(&monitor),
                geometry(BarPosition::Bottom),
                DOCK,
            )
        };
        let tiled = placed(window("tiled", "eDP-1"), 0, 0, 1920, 1080);
        let floating = placed(window("floating", "eDP-1"), 100, 100, 400, 300);

        assert_eq!(dodge(&[]), DockHideBehavior::Visible);
        assert_eq!(
            dodge(std::slice::from_ref(&floating)),
            DockHideBehavior::Visible
        );
        // Unfocused windows count too: the dock dodges what is on screen.
        assert_eq!(
            dodge(&[floating.clone(), tiled.clone()]),
            DockHideBehavior::Immediate
        );

        let elsewhere = services::Window {
            workspace_id: 2,
            ..tiled.clone()
        };
        let other_monitor = services::Window {
            monitor: "HDMI-A-1".to_string(),
            ..tiled.clone()
        };
        let minimized = services::Window {
            is_minimized: true,
            ..tiled.clone()
        };
        assert_eq!(
            dodge(&[elsewhere, other_monitor, minimized]),
            DockHideBehavior::Visible
        );
        assert_eq!(
            dodge_windows_hide_behavior(&[tiled], None, geometry(BarPosition::Bottom), DOCK),
            DockHideBehavior::Visible
        );
    }

    #[test]
    fn dodge_windows_treats_unplaced_windows_as_covering_the_dock() {
        let monitor = monitor();
        let niri_window = window("column", "eDP-1");

        assert_eq!(
            dodge_windows_hide_behavior(
                std::slice::from_ref(&niri_window),
                Some(&monitor),
                geometry(BarPosition::Bottom),
                DOCK
            ),
            DockHideBehavior::Immediate
        );
        let elsewhere = services::Window {
            workspace_id: 2,
            ..niri_window
        };
        assert_eq!(
            dodge_windows_hide_behavior(
                &[elsewhere],
                Some(&monitor),
                geometry(BarPosition::Bottom),
                DOCK
            ),
            DockHideBehavior::Visible
        );
    }

    #[test]
    fn dock_bounds_follow_the_monitor_scale() {
        let monitor = services::Monitor {
            width: 3840,
            height: 2160,
            scale: 2.0,
            ..monitor()
        };

        let dock_bounds =
            dock_bounds_in_compositor_space(geometry(BarPosition::Bottom), DOCK, &monitor);

        // Logical 1920x1080: centered, 8px above the bottom edge.
        assert_eq!(dock_bounds.origin, point(px(860.0), px(1011.0)));
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
            scale: 1.0,
            ..Default::default()
        };
        let dock_bounds = dock_bounds_in_compositor_space(
            geometry(BarPosition::Bottom),
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

    /// Stretched, the dock runs the length of its edge less the gap at both
    /// ends, keeps its fitted thickness, and reserves that thickness.
    #[test]
    fn a_stretched_dock_spans_its_edge_and_reserves_its_thickness() {
        let display = Size::new(px(1920.0), px(1080.0));
        let fitted = DockConfig {
            hover_effect: DockHoverEffect::None,
            ..Default::default()
        };
        assert_eq!(
            dock_extent(&fitted, 2, display),
            Size::new(px(104.0), px(61.0))
        );

        let floating = DockConfig {
            stretch: true,
            ..fitted.clone()
        };
        let size = dock_extent(&floating, 2, display);
        assert_eq!(size, Size::new(px(1904.0), px(61.0)));
        assert_eq!(dock_thickness(&floating, size), px(61.0));

        let attached = DockConfig {
            style: DockStyle::Attached,
            position: BarPosition::Left,
            ..floating
        };
        let size = dock_extent(&attached, 2, display);
        assert_eq!(size, Size::new(px(56.0), px(1080.0)));
        assert_eq!(dock_thickness(&attached, size), px(56.0));
    }

    /// An attached dock sits on the edge itself, so the geometry the dodge
    /// rule and the menus work from has no inset.
    #[test]
    fn an_attached_dock_has_no_edge_gap() {
        let attached = DockConfig {
            style: DockStyle::Attached,
            ..Default::default()
        };
        let bounds = dock_bounds_in_compositor_space(attached.geometry(), DOCK, &monitor());
        assert_eq!(bounds.origin, point(px(860.0), px(1019.0)));
    }

    /// A fitted dock aligned to an end keeps its gap from that end.
    #[test]
    fn an_aligned_dock_sits_at_its_end_of_the_edge() {
        let aligned = |align| DockGeometry {
            align,
            ..geometry(BarPosition::Bottom)
        };
        let start = dock_bounds_in_compositor_space(aligned(DockAlign::Start), DOCK, &monitor());
        assert_eq!(start.origin, point(px(8.0), px(1011.0)));
        let end = dock_bounds_in_compositor_space(aligned(DockAlign::End), DOCK, &monitor());
        assert_eq!(end.origin, point(px(1712.0), px(1011.0)));
    }

    /// The launcher button takes an icon slot of its own.
    #[test]
    fn the_launcher_button_widens_a_fitted_dock_by_one_slot() {
        let display = Size::new(px(1920.0), px(1080.0));
        let plain = DockConfig {
            hover_effect: DockHoverEffect::None,
            ..Default::default()
        };
        let with_launcher = DockConfig {
            launcher: true,
            ..plain.clone()
        };
        assert_eq!(
            dock_extent(&with_launcher, 2, display).width,
            dock_extent(&plain, 3, display).width
        );
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
            geometry(BarPosition::Bottom),
            point(px(78.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.anchor, Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(placement.margin, (0.0, 0.0, 69.0, 416.0));
    }

    #[test]
    fn dock_context_menu_top_opens_below_the_clicked_icon() {
        let placement = dock_panel_placement_from_dock_click(
            geometry(BarPosition::Top),
            point(px(26.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.margin, (69.0, 0.0, 0.0, 364.0));
    }

    #[test]
    fn dock_context_menu_left_opens_to_the_right_of_the_clicked_icon() {
        let placement = dock_panel_placement_from_dock_click(
            geometry(BarPosition::Left),
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
            geometry(BarPosition::Right),
            point(px(20.0), px(30.0)),
            Size::new(px(56.0), px(114.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0))),
        );

        assert_eq!(placement.margin, (321.0, 0.0, 0.0, 716.0));
    }

    /// A reserved dock is outside the usable area the menu is placed in,
    /// so the menu sits on its edge instead of a dock's height above it.
    #[test]
    fn dock_context_menu_sits_on_a_reserved_dock() {
        let display = Bounds::new(point(px(0.0), px(0.0)), Size::new(px(1000.0), px(800.0)));
        // A 32px bar at the top and the dock's 69px at the bottom are reserved.
        let usable = Bounds::new(point(px(0.0), px(32.0)), Size::new(px(1000.0), px(699.0)));
        let placement = dock_panel_placement_from_dock_click(
            geometry(BarPosition::Bottom),
            point(px(78.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            display,
            usable,
        );
        assert_eq!(placement.margin, (0.0, 0.0, 0.0, 416.0));

        let placement = dock_panel_placement_from_dock_click(
            geometry(BarPosition::Top),
            point(px(26.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            display,
            Bounds::new(point(px(0.0), px(69.0)), Size::new(px(1000.0), px(731.0))),
        );
        assert_eq!(placement.margin, (0.0, 0.0, 0.0, 364.0));
    }

    #[test]
    fn dock_context_menu_clamps_the_dock_local_position_to_usable_bounds() {
        let placement = dock_panel_placement_from_dock_click(
            geometry(BarPosition::Bottom),
            point(px(130.0), px(28.0)),
            Size::new(px(104.0), px(61.0)),
            dock_context_menu_size(2),
            Bounds::new(point(px(0.0), px(0.0)), Size::new(px(300.0), px(200.0))),
            Bounds::new(point(px(20.0), px(10.0)), Size::new(px(260.0), px(170.0))),
        );

        assert_eq!(placement.anchor, Anchor::BOTTOM | Anchor::LEFT);
        assert_eq!(placement.margin, (0.0, 0.0, 49.0, 40.0));
    }
}
