use crate::launcher::view::{LauncherView, ViewContext, ViewObserver, render_list_item};
use crate::services::Services;
use crate::services::compositor::types::CompositorCommand;
use gpui::{AnyElement, App, Context, div, prelude::*, px};

pub struct MonitorsView;

impl<T: 'static> ViewObserver<T> for MonitorsView {
    fn observe_services(services: &Services, cx: &mut Context<T>) {
        // MonitorsView only needs to observe the compositor service
        cx.observe(&services.compositor, |_, _, cx| cx.notify())
            .detach();
    }
}

impl LauncherView for MonitorsView {
    fn prefix(&self) -> &'static str {
        "mon"
    }

    fn name(&self) -> &'static str {
        "Monitors"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Focus a monitor"
    }

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let compositor = vx.services.compositor.read(cx);
        let query_lower = vx.query.to_lowercase();

        let filtered: Vec<_> = compositor
            .monitors
            .iter()
            .filter(|mon| {
                if vx.query.is_empty() {
                    return true;
                }
                mon.name.to_lowercase().contains(&query_lower)
            })
            .collect();

        let count = filtered.len();
        let services = vx.services.clone();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(filtered.into_iter().enumerate().map(|(i, mon)| {
                let subtitle = format!("Workspace {}", mon.active_workspace_id);
                let mon_id = mon.id;
                let services_clone = services.clone();

                render_list_item(
                    format!("mon-{}", mon.id),
                    "",
                    &mon.name,
                    Some(&subtitle),
                    i == vx.selected_index,
                    move |cx| {
                        services_clone.compositor.update(cx, |compositor, cx| {
                            compositor.dispatch(CompositorCommand::FocusMonitor(mon_id), cx);
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
            .monitors
            .iter()
            .filter(|mon| {
                if vx.query.is_empty() {
                    return true;
                }
                mon.name.to_lowercase().contains(&query_lower)
            })
            .collect();

        if let Some(mon) = filtered.get(index) {
            let mon_id = mon.id;
            vx.services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusMonitor(mon_id), cx);
            });
            true
        } else {
            false
        }
    }
}
