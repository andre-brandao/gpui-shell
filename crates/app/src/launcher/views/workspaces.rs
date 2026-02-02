//! Workspaces view for switching between compositor workspaces.

use crate::launcher::view::{LauncherView, ViewContext, render_list_item};
use gpui::{AnyElement, App, div, prelude::*, px};
use services::CompositorCommand;

/// Workspaces view - displays and switches between workspaces.
pub struct WorkspacesView;

impl LauncherView for WorkspacesView {
    fn prefix(&self) -> &'static str {
        ";ws"
    }

    fn name(&self) -> &'static str {
        "Workspaces"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Switch between workspaces"
    }

    fn render(&self, vx: &ViewContext, _cx: &App) -> (AnyElement, usize) {
        let compositor = vx.services.compositor.get();
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = compositor
            .workspaces
            .iter()
            .filter(|ws| !ws.is_special)
            .filter(|ws| {
                if vx.query.is_empty() {
                    return true;
                }
                let title = if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                };
                title.to_lowercase().contains(&query_lower)
                    || ws.monitor.to_lowercase().contains(&query_lower)
            })
            .collect();

        let count = filtered.len();
        let compositor_sub = vx.services.compositor.clone();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, ws)| {
                let title = if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                };
                let subtitle = format!("{} windows on {}", ws.windows, ws.monitor);
                let ws_id = ws.id;
                let compositor_clone = compositor_sub.clone();

                render_list_item(
                    format!("ws-{}", ws.id),
                    "",
                    &title,
                    Some(&subtitle),
                    i == vx.selected_index,
                    move |_cx| {
                        let _ = compositor_clone.dispatch(CompositorCommand::FocusWorkspace(ws_id));
                    },
                )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let compositor = vx.services.compositor.get();
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = compositor
            .workspaces
            .iter()
            .filter(|ws| !ws.is_special)
            .filter(|ws| {
                if vx.query.is_empty() {
                    return true;
                }
                let title = if ws.name.is_empty() {
                    format!("Workspace {}", ws.id)
                } else {
                    ws.name.clone()
                };
                title.to_lowercase().contains(&query_lower)
                    || ws.monitor.to_lowercase().contains(&query_lower)
            })
            .collect();

        if let Some(ws) = filtered.get(index) {
            let ws_id = ws.id;
            let _ = vx
                .services
                .compositor
                .dispatch(CompositorCommand::FocusWorkspace(ws_id));
            true // Close launcher
        } else {
            false
        }
    }

    fn footer_actions(&self, _vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Switch", "Enter"), ("Close", "Esc")]
    }
}
