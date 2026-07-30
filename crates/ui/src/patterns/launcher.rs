//! Launcher chrome: the bordered surface that holds a query line, a body, and
//! a footer hint bar.

use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

/// The launcher surface: query row, body, footer hint bar.
#[derive(IntoElement)]
#[must_use = "LauncherFrame does nothing unless rendered"]
pub struct LauncherFrame {
    query: AnyElement,
    icon: IconName,
    badge: Option<SharedString>,
    hints: Option<SharedString>,
    actions: Option<AnyElement>,
    body: Vec<AnyElement>,
}

impl LauncherFrame {
    /// Build a frame around `query` - typically
    /// [`render_input_line`](crate::render_input_line).
    pub fn new(query: impl IntoElement) -> Self {
        Self {
            query: query.into_any_element(),
            icon: IconName::MagnifyingGlass,
            badge: None,
            hints: None,
            actions: None,
            body: Vec::new(),
        }
    }

    /// Replace the search icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// Pill on the right of the query row, naming the active view.
    pub fn badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Left side of the footer: which prefixes are available.
    pub fn hints(mut self, hints: impl Into<SharedString>) -> Self {
        self.hints = Some(hints.into());
        self
    }

    /// Right side of the footer: what the current view does with a key.
    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
}

impl ParentElement for LauncherFrame {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for LauncherFrame {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let (text, muted, disabled) = (colors.text, colors.text_muted, colors.text_disabled);
        let (background, border, badge_bg) =
            (colors.background, colors.border, colors.element_background);
        let hairline = move || div().w_full().h(px(1.)).bg(border);
        let has_footer = self.hints.is_some() || self.actions.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(background)
            .border_1()
            .border_color(border)
            .rounded(Radius::Large.pixels())
            .text_color(text)
            // Query row
            .child(
                div()
                    .w_full()
                    .px(Spacing::XLarge.pixels())
                    .py(Spacing::Large.pixels())
                    .flex()
                    .items_center()
                    .gap(Spacing::Large.pixels())
                    .child(
                        Icon::new(self.icon)
                            .size(IconSize::Medium)
                            .color(Color::Custom(muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(TextSize::Medium.rems())
                            .child(self.query),
                    )
                    .when_some(self.badge, |el, badge| {
                        el.child(
                            div()
                                .px(Spacing::Medium.pixels())
                                .py(px(3.))
                                .rounded(Radius::Small.pixels())
                                .bg(badge_bg)
                                .text_size(TextSize::Small.rems())
                                .child(badge),
                        )
                    }),
            )
            .child(hairline())
            // Body. Owns no scroll of its own - which element scrolls is the
            // app's call.
            .child(
                div()
                    .id("launcher-body")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .py(Spacing::XSmall.pixels())
                    .children(self.body),
            )
            .when(has_footer, |el| {
                el.child(hairline()).child(
                    div()
                        .w_full()
                        .px(Spacing::XLarge.pixels())
                        .py(Spacing::Medium.pixels())
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(disabled)
                                .children(self.hints),
                        )
                        .children(self.actions),
                )
            })
    }
}

/// Render `(action, key)` pairs as the footer's right-hand hints - `Open ⏎ Close Esc`.
pub fn footer_hints(actions: Vec<(&'static str, &'static str)>, cx: &App) -> AnyElement {
    let colors = cx.theme().colors();
    let muted = colors.text_muted;
    let key_bg = colors.element_background;

    div()
        .flex()
        .items_center()
        .gap(Spacing::XLarge.pixels())
        .text_size(TextSize::Small.rems())
        .text_color(muted)
        .children(actions.into_iter().map(|(action, key)| {
            div()
                .flex()
                .items_center()
                .gap(px(Spacing::Medium.value() - 2.0))
                .child(action)
                .child(
                    div()
                        .px(px(Spacing::Medium.value() - 2.0))
                        .py(px(2.))
                        .rounded(px(Radius::Small.value() - 1.0))
                        .bg(key_bg)
                        .text_size(TextSize::XSmall.rems())
                        .child(key),
                )
        }))
        .into_any_element()
}
