use gpui::{div, prelude::*, px, AnyView, App, Render, SharedString, Window};

use ui::{radius, spacing, ActiveTheme};

pub fn control_center_tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let text = text.into();
    move |_, cx| {
        cx.new(|_| ControlCenterTooltip { text: text.clone() })
            .into()
    }
}

struct ControlCenterTooltip {
    text: SharedString,
}

impl Render for ControlCenterTooltip {
    fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .bg(theme.bg.elevated)
            .rounded(px(radius::SM))
            .shadow_md()
            .text_size(theme.font_sizes.xs)
            .text_color(theme.text.primary)
            .child(self.text.clone())
    }
}
