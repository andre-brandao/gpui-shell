//! Workspaces widget for displaying and switching compositor workspaces.

mod config;
pub use config::WorkspacesConfig;

use gpui::{AnyElement, Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState};
use ui::{ActiveTheme, Radius};

use super::{BarWidget, BarWidgetShell, style};
use crate::config::ActiveConfig;
use crate::state::AppState;
use crate::state::watch;

/// Workspaces widget that displays workspace indicators and allows switching.
pub struct Workspaces {
    compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl Workspaces {
    /// Create a new Workspaces widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        watch(cx, compositor.subscribe(), |this, new_state, cx| {
            this.state = new_state;
            cx.notify();
        });

        Self { compositor, state }
    }

    /// Handle clicking on a workspace to focus it.
    fn focus_workspace(&self, workspace_id: i32) {
        if let Err(e) = self
            .compositor
            .dispatch(CompositorCommand::FocusWorkspace(workspace_id))
        {
            tracing::error!("Failed to focus workspace {}: {}", workspace_id, e);
        }
    }

    /// Resolve the compositor monitor name for the display this widget's
    /// window is on, so we only show that monitor's workspaces.
    ///
    /// Neither `Window::display()` nor display bounds/position nor ordinal
    /// list position are reliable here: gpui_linux derives `Window::display()`
    /// from `primary_output_scale()` (whichever output has the highest scale
    /// factor, unrelated to which output this layer-shell window is actually
    /// anchored to); every display's `bounds().origin` comes back as (0, 0)
    /// regardless of its real position; and `cx.displays()` isn't guaranteed
    /// to enumerate outputs in the same order as the compositor's own monitor
    /// list (confirmed to differ on at least one real setup, which silently
    /// swapped which bar controlled which monitor). Instead we match by
    /// identity: gpui_linux's `PlatformDisplay::uuid()` is a deterministic
    /// hash of the Wayland output's name (`Uuid::new_v5(NAMESPACE_DNS,
    /// name)`), so we look up the `DisplayId` this bar window was opened
    /// with (`crate::state::display_id_for_window`), and compare its uuid
    /// against the same hash computed from each compositor monitor's name.
    fn current_monitor_name(&self, window: &Window, cx: &gpui::App) -> Option<String> {
        let display_id = crate::state::display_id_for_window(window)?;
        crate::state::monitor_for_display(Some(display_id), &self.state, cx).map(|m| m.name.clone())
    }

    /// Handle scrolling to switch workspaces.
    fn scroll_workspace(&self, direction: i32) {
        if let Err(e) = self
            .compositor
            .dispatch(CompositorCommand::ScrollWorkspace(direction))
        {
            tracing::error!("Failed to scroll workspace: {}", e);
        }
    }

    fn workspace_label(
        ws: &services::Workspace,
        is_vertical: bool,
        show_numbers: bool,
        show_icons: bool,
    ) -> String {
        if !show_numbers && !show_icons {
            return String::new();
        }

        let name = ws.name.trim();
        let is_numeric_name = !name.is_empty() && name.chars().all(|ch| ch.is_ascii_digit());

        if show_icons && !show_numbers {
            if name.is_empty() || is_numeric_name {
                return String::new();
            }
            return name.chars().take(3).collect::<String>().to_uppercase();
        }

        if show_numbers && !show_icons {
            return ws.id.to_string();
        }

        if is_vertical {
            return ws.id.to_string();
        }

        if name.is_empty() {
            return ws.id.to_string();
        }

        if is_numeric_name {
            name.to_string()
        } else {
            name.chars().take(3).collect::<String>().to_uppercase()
        }
    }

    fn render_workspace_pill(
        &self,
        ws: &services::Workspace,
        active_workspace_id: Option<i32>,
        theme: &ui::Theme,
        is_vertical: bool,
        config: &WorkspacesConfig,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let workspace_id = ws.id;
        let is_active = active_workspace_id == Some(ws.id);
        let has_windows = ws.windows > 0;
        let label = Self::workspace_label(ws, is_vertical, config.show_numbers, config.show_icons);

        div()
            .id(format!("workspace-{}", ws.id))
            .flex()
            .items_center()
            .justify_center()
            .w(if is_vertical {
                if is_active {
                    px(style::WORKSPACE_PILL_WIDTH_ACTIVE)
                } else {
                    px(style::WORKSPACE_PILL_WIDTH)
                }
            } else if is_active {
                px(style::WORKSPACE_PILL_WIDTH_HORIZONTAL_ACTIVE)
            } else {
                px(style::WORKSPACE_PILL_WIDTH_HORIZONTAL)
            })
            .h(px(style::WORKSPACE_PILL_HEIGHT))
            .rounded(Radius::Small.pixels())
            .cursor_pointer()
            .bg(if is_active {
                theme.colors.accent
            } else if has_windows {
                theme.colors.elevated_surface_background
            } else {
                gpui::transparent_black()
            })
            .hover(move |s| {
                if is_active {
                    s.bg(theme.colors.accent)
                } else {
                    s.bg(theme.colors.element_hover)
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event, _window, _cx| {
                    this.focus_workspace(workspace_id);
                }),
            )
            .when(!label.is_empty(), |this| {
                this.child(
                    div()
                        .text_size(style::label_size(is_vertical).rems())
                        .text_color(if is_active {
                            theme.colors.background
                        } else if has_windows {
                            theme.colors.text
                        } else {
                            theme.colors.text_muted
                        })
                        .child(label),
                )
            })
            .into_any_element()
    }

    fn render_workspace_strip(
        &self,
        theme: &ui::Theme,
        is_vertical: bool,
        config: &WorkspacesConfig,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let monitor_name = self.current_monitor_name(window, cx);

        // Prefer this monitor's own active tag over the compositor-wide
        // `active_workspace_id`, which only ever reflects whichever monitor
        // currently has focus - on multi-monitor setups that's a different
        // monitor from this bar's as soon as focus moves elsewhere, which
        // would otherwise make this bar lose (or misreport) its highlight.
        let active_workspace_id = monitor_name
            .as_deref()
            .and_then(|name| self.state.monitors.iter().find(|m| m.name == name))
            .map(|m| m.active_workspace_id)
            .filter(|&id| id >= 0)
            .or(self.state.active_workspace_id);

        div()
            .id("workspaces")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::group_gap(is_vertical)))
            .on_scroll_wheel(
                cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, _cx| {
                    let delta = event.delta.pixel_delta(px(1.0));
                    if delta.y.abs() > px(0.5) {
                        let direction = if delta.y > px(0.0) { -1 } else { 1 };
                        this.scroll_workspace(direction);
                    }
                }),
            )
            .children(
                self.state
                    .workspaces
                    .iter()
                    .filter(|ws| !ws.is_special)
                    .filter(|ws| {
                        monitor_name
                            .as_deref()
                            .is_none_or(|name| ws.monitor == name)
                    })
                    .map(|ws| {
                        self.render_workspace_pill(
                            ws,
                            active_workspace_id,
                            theme,
                            is_vertical,
                            config,
                            cx,
                        )
                    }),
            )
            .into_any_element()
    }
}

impl BarWidget for Workspaces {
    fn shell(&self) -> BarWidgetShell {
        BarWidgetShell::Group
    }

    fn render_vertical(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = cx.config().bar.modules.workspaces.clone();
        self.render_workspace_strip(&theme, true, &config, window, cx)
    }

    fn render_horizontal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = cx.config().bar.modules.workspaces.clone();
        self.render_workspace_strip(&theme, false, &config, window, cx)
    }
}

impl Render for Workspaces {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}
