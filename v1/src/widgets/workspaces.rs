use crate::services::Services;
use crate::services::compositor::{Compositor, CompositorCommand};
use crate::theme::{accent, interactive, radius, spacing, text};
use gpui::{Context, Entity, MouseButton, Window, div, prelude::*, px};

pub struct Workspaces {
    compositor: Entity<Compositor>,
}

impl Workspaces {
    /// Create workspaces widget with shared services.
    pub fn with_services(services: Services, cx: &mut Context<Self>) -> Self {
        cx.observe(&services.compositor, |_, _, cx| cx.notify())
            .detach();
        Workspaces {
            compositor: services.compositor,
        }
    }
}

impl Render for Workspaces {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compositor = self.compositor.read(cx);
        let active_workspace_id = compositor.active_workspace_id;

        div().flex().items_center().gap(px(spacing::XS)).children(
            compositor
                .workspaces
                .iter()
                .filter(|ws| !ws.is_special)
                .map(|ws| {
                    let workspace_id = ws.id;
                    let is_active = active_workspace_id == Some(ws.id);
                    let compositor = self.compositor.clone();

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
                            accent::primary()
                        } else {
                            interactive::default()
                        })
                        .hover(|s| {
                            if is_active {
                                s.bg(accent::hover())
                            } else {
                                s.bg(interactive::hover())
                            }
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_this, _event, _window, cx| {
                                compositor.update(cx, |compositor, cx| {
                                    compositor.dispatch(
                                        CompositorCommand::FocusWorkspace(workspace_id),
                                        cx,
                                    );
                                });
                            }),
                        )
                        .child(
                            div()
                                .text_color(if is_active {
                                    text::primary()
                                } else {
                                    text::secondary()
                                })
                                .child(ws.name.clone()),
                        )
                }),
        )
    }
}
