use crate::launcher::view::{
    LauncherView, ViewAction, ViewContext, execute_action, render_list_item,
};
use gpui::{AnyElement, App, div, prelude::*, px};

pub struct MonitorsView;

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
                        execute_action(&ViewAction::FocusMonitor(mon_id), &services_clone, cx);
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
            execute_action(&ViewAction::FocusMonitor(mon.id), vx.services, cx);
            true
        } else {
            false
        }
    }
}
