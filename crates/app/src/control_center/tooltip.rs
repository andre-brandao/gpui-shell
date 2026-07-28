use gpui::{AnyView, App, Render, SharedString, Window, div, prelude::*};

use ui::{ActiveTheme, Radius, Spacing, TextSize};

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
            .px(Spacing::Medium.pixels())
            .py(Spacing::XSmall.pixels())
            .bg(theme.colors.elevated_surface_background)
            .rounded(Radius::Small.pixels())
            .shadow_md()
            .text_size(TextSize::XSmall.rems())
            .text_color(theme.colors.text)
            .child(self.text.clone())
    }
}
