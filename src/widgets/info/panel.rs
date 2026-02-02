//! Info panel that wraps the shared ControlCenter component.

use crate::services::Services;
use crate::theme::{bg, border, radius, text};
use crate::ui::ControlCenter;
use gpui::{Context, Entity, FocusHandle, Focusable, Window, div, prelude::*, px};

/// Info panel content - wraps the shared ControlCenter component.
pub struct InfoPanelContent {
    control_center: Entity<ControlCenter>,
    focus_handle: FocusHandle,
}

impl InfoPanelContent {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let control_center = cx.new(|cx| ControlCenter::new(services, cx));

        // Re-render when control center updates
        cx.observe(&control_center, |_, _, cx| cx.notify()).detach();

        InfoPanelContent {
            control_center,
            focus_handle,
        }
    }
}

impl Focusable for InfoPanelContent {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InfoPanelContent {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("info-panel-content")
            .track_focus(&self.focus_handle)
            .key_context("InfoPanel")
            .size_full()
            .bg(bg::primary())
            .border_1()
            .border_color(border::default())
            .rounded(px(radius::LG))
            .text_color(text::primary())
            .child(self.control_center.clone())
    }
}
