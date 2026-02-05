//! Workspaces widget for displaying and switching compositor workspaces.

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::{CompositorCommand, CompositorState, CompositorSubscriber, Services};
use ui::prelude::*;

pub struct Workspaces {
    compositor: CompositorSubscriber,
    state: CompositorState,
}

impl Workspaces {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let compositor = services.compositor.clone();
        let state = compositor.get();

        // Subscribe to compositor state changes
        cx.spawn({
            let mut signal = compositor.subscribe().to_stream();
            async move |this, cx| {
                while let Some(new_state) = signal.next().await {
                    if this
                        .update(cx, |this, cx| {
                            this.state = new_state;
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

        Self { compositor, state }
    }

    fn focus_workspace(&self, workspace_id: i32) {
        if let Err(e) = self
            .compositor
            .dispatch(CompositorCommand::FocusWorkspace(workspace_id))
        {
            tracing::error!("Failed to focus workspace {}: {}", workspace_id, e);
        }
    }

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
        let colors = cx.theme().colors();
        let active_workspace_id = self.state.active_workspace_id;

        div()
            .id("workspaces")
            .flex()
            .items_center()
            .gap(px(6.0))
            // Scroll to switch workspaces
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
                    .map(|ws| {
                        let workspace_id = ws.id;
                        let is_active = active_workspace_id == Some(ws.id);
                        let has_windows = ws.windows > 0;

                        let base_bg = if is_active {
                            colors.element_active
                        } else if has_windows {
                            colors.element_background
                        } else {
                            colors.surface_background
                        };

                        let hover_bg = colors.element_hover;

                        let text_color = if is_active {
                            colors.text
                        } else if has_windows {
                            colors.text_muted
                        } else {
                            colors.text_muted
                        };

                        div()
                            .id(format!("workspace-{}", ws.id))
                            .px(if is_active { px(10.0) } else { px(8.0) })
                            .py(px(2.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .bg(base_bg)
                            .hover(move |s| s.bg(hover_bg))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _event, _window, _cx| {
                                    this.focus_workspace(workspace_id);
                                }),
                            )
                            .child(div().text_color(text_color).child(ws.name.clone()))
                    }),
            )
    }
}
