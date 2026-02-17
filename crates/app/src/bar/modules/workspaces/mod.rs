//! Workspaces widget for displaying and switching compositor workspaces.

mod config;
pub use config::WorkspacesConfig;

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState};
use ui::{ActiveTheme, radius};

use super::style;
use crate::config::ActiveConfig;
use crate::state::AppState;

/// Workspaces widget that displays workspace indicators and allows switching.
pub struct Workspaces {
    compositor: services::CompositorSubscriber,
    state: CompositorState,
}

impl Workspaces {
    /// Create a new Workspaces widget.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = AppState::services(cx).compositor.clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    let result = this.update(cx, |this, cx| {
                        this.state = new_state;
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

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
}

impl Render for Workspaces {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let active_workspace_id = self.state.active_workspace_id;
        let config = &cx.config().bar.modules.workspaces;
        let show_numbers = config.show_numbers;
        let show_icons = config.show_icons;

        // Pre-compute colors for closures
        let accent_primary = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let bg_primary = theme.bg.primary;
        let text_secondary = theme.text.secondary;
        let text_muted = theme.text.muted;
        let transparent = gpui::transparent_black();

        div()
            .id("workspaces")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            // Scroll to switch workspaces
            .on_scroll_wheel(
                cx.listener(|this, event: &gpui::ScrollWheelEvent, _window, _cx| {
                    let delta = event.delta.pixel_delta(px(1.0));
                    // Use Pixels comparison methods instead of accessing private field
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
                    .map(|ws| {
                        let workspace_id = ws.id;
                        let is_active = active_workspace_id == Some(ws.id);
                        let has_windows = ws.windows > 0;
                        let label =
                            Self::workspace_label(ws, is_vertical, show_numbers, show_icons);

                        div()
                            .id(format!("workspace-{}", ws.id))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(is_vertical, |this| {
                                this.w(if is_active {
                                    px(style::WORKSPACE_PILL_WIDTH_ACTIVE)
                                } else {
                                    px(style::WORKSPACE_PILL_WIDTH)
                                })
                                .h(px(style::WORKSPACE_PILL_HEIGHT))
                            })
                            .when(!is_vertical, |this| {
                                this.w(if is_active {
                                    px(style::WORKSPACE_PILL_WIDTH_HORIZONTAL_ACTIVE)
                                } else {
                                    px(style::WORKSPACE_PILL_WIDTH_HORIZONTAL)
                                })
                                .h(px(style::WORKSPACE_PILL_HEIGHT))
                            })
                            .rounded(px(radius::SM))
                            .cursor_pointer()
                            .bg(if is_active {
                                accent_primary
                            } else if has_windows {
                                interactive_default
                            } else {
                                transparent
                            })
                            .hover(move |s| {
                                if is_active {
                                    s.bg(accent_hover)
                                } else {
                                    s.bg(interactive_hover)
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
                                        .text_size(px(style::label(is_vertical)))
                                        .text_color(if is_active {
                                            bg_primary
                                        } else if has_windows {
                                            text_secondary
                                        } else {
                                            text_muted
                                        })
                                        .child(label),
                                )
                            })
                    }),
            )
    }
}
