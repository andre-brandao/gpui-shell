use crate::services::compositor::{Compositor, CompositorCommand};
use gpui::{Context, Entity, MouseButton, Window, div, prelude::*, px, rgba};

pub struct Workspaces {
    compositor: Entity<Compositor>,
}

impl Workspaces {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let compositor = cx.new(Compositor::new);

        // Observe the compositor entity to re-render when it changes
        cx.observe(&compositor, |_, _, cx| cx.notify()).detach();

        Workspaces { compositor }
    }

    /// Create workspaces widget with a shared compositor entity.
    /// Use this when you want multiple widgets to share the same compositor state.
    pub fn with_compositor(compositor: Entity<Compositor>, cx: &mut Context<Self>) -> Self {
        cx.observe(&compositor, |_, _, cx| cx.notify()).detach();
        Workspaces { compositor }
    }
}

impl Render for Workspaces {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let compositor = self.compositor.read(cx);
        let active_workspace_id = compositor.active_workspace_id;

        div().flex().items_center().gap(px(4.)).children(
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
                        .px(if is_active { px(16.) } else { px(8.) })
                        .py(px(2.))
                        .rounded(px(25.))
                        .bg(if is_active {
                            rgba(0x3b82f6ff)
                        } else {
                            rgba(0x333333ff)
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
                        .child(ws.name.clone())
                }),
        )
    }
}
