use crate::launcher::view::{LauncherView, ViewContext, ViewObserver, render_list_item};
use crate::services::Services;
use crate::services::compositor::types::CompositorCommand;
use gpui::{AnyElement, App, Context, div, prelude::*, px};

pub struct WorkspacesView;

impl<T: 'static> ViewObserver<T> for WorkspacesView {
    fn observe_services(services: &Services, cx: &mut Context<T>) {
        // WorkspacesView only needs to observe the compositor service
        cx.observe(&services.compositor, |_, _, cx| cx.notify())
            .detach();
    }
}

impl LauncherView for WorkspacesView {
    fn prefix(&self) -> &'static str {
        "ws"
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

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let compositor = vx.services.compositor.read(cx);
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
        let services = vx.services.clone();

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
                let services_clone = services.clone();

                render_list_item(
                    format!("ws-{}", ws.id),
                    "",
                    &title,
                    Some(&subtitle),
                    i == vx.selected_index,
                    move |cx| {
                        services_clone.compositor.update(cx, |compositor, cx| {
                            compositor.dispatch(CompositorCommand::FocusWorkspace(ws_id), cx);
                        });
                    },
                )
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let compositor = vx.services.compositor.read(cx);
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
            vx.services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusWorkspace(ws_id), cx);
            });
            true
        } else {
            false
        }
    }
}
