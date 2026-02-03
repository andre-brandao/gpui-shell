//! Workspaces widget for displaying and switching compositor workspaces.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState, CompositorSubscriber};
use ui::{ActiveTheme, radius, spacing};

/// Workspaces widget that displays workspace indicators and allows switching.
pub struct Workspaces {
    compositor: CompositorSubscriber,
    state: CompositorState,
}

impl Workspaces {
    /// Create a new Workspaces widget with the given compositor subscriber.
    pub fn new(compositor: CompositorSubscriber, cx: &mut Context<Self>) -> Self {
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
}

impl Render for Workspaces {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let active_workspace_id = self.state.active_workspace_id;

        // Pre-compute colors for closures
        let accent_primary = theme.accent.primary;
        let accent_hover = theme.accent.hover;
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let text_primary = theme.text.primary;
        let text_secondary = theme.text.secondary;
        let text_muted = theme.text.muted;
        let transparent = gpui::transparent_black();

        div()
            .id("workspaces")
            .flex()
            .items_center()
            .gap(px(spacing::XS))
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

                        div()
                            .id(format!("workspace-{}", ws.id))
                            .px(if is_active {
                                px(spacing::MD)
                            } else {
                                px(spacing::SM)
                            })
                            .py(px(2.))
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
                            .child(
                                div()
                                    .text_color(if is_active {
                                        text_primary
                                    } else if has_windows {
                                        text_secondary
                                    } else {
                                        text_muted
                                    })
                                    .child(ws.name.clone()),
                            )
                    }),
            )
    }
}
