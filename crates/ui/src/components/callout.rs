//! Callout - a highlighted info/warning/error block for important messages
//! that require user attention.

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*, px};

use crate::components::banner::Severity;
use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::theme::TextSize;

/// Whether the accent border appears at the top or bottom of the callout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderPosition {
    Top,
    Bottom,
}

/// A block-level callout for displaying important information.
#[derive(IntoElement)]
#[must_use = "Callout does nothing unless rendered"]
pub struct Callout {
    severity: Severity,
    icon: Option<IconName>,
    title: Option<gpui::SharedString>,
    description: Option<gpui::SharedString>,
    actions_slot: Option<AnyElement>,
    dismiss_action: Option<AnyElement>,
    border_position: BorderPosition,
}

impl Callout {
    pub fn new() -> Self {
        Self {
            severity: Severity::Info,
            icon: None,
            title: None,
            description: None,
            actions_slot: None,
            dismiss_action: None,
            border_position: BorderPosition::Top,
        }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn title(mut self, title: impl Into<gpui::SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<gpui::SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn actions_slot(mut self, action: impl IntoElement) -> Self {
        self.actions_slot = Some(action.into_any_element());
        self
    }

    pub fn dismiss_action(mut self, action: impl IntoElement) -> Self {
        self.dismiss_action = Some(action.into_any_element());
        self
    }

    pub fn border_position(mut self, border_position: BorderPosition) -> Self {
        self.border_position = border_position;
        self
    }
}

impl Default for Callout {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Callout {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let status = &colors.status;

        let (icon, icon_color, bg_color) = match self.severity {
            Severity::Info => (IconName::Info, Color::Muted, status.info_background),
            Severity::Success => (IconName::Check, Color::Success, status.success_background),
            Severity::Warning => (IconName::Warning, Color::Warning, status.warning_background),
            Severity::Error => (IconName::XCircle, Color::Error, status.error_background),
        };

        let has_actions = self.actions_slot.is_some() || self.dismiss_action.is_some();

        h_flex()
            .w_full()
            .p(Spacing::Medium.pixels())
            .gap(Spacing::Medium.pixels())
            .items_start()
            .map(|this| match self.border_position {
                BorderPosition::Top => this.border_t_1(),
                BorderPosition::Bottom => this.border_b_1(),
            })
            .border_color(colors.border)
            .bg(bg_color)
            .overflow_x_hidden()
            .when(self.icon.is_some(), |this| {
                this.child(Icon::new(icon).size(IconSize::Small).color(icon_color))
            })
            .child(
                v_flex()
                    .w_full()
                    .gap(px(2.0))
                    .child(
                        h_flex()
                            .w_full()
                            .gap(Spacing::Small.pixels())
                            .justify_between()
                            .flex_wrap()
                            .when_some(self.title, |this, title| {
                                this.child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .child(Label::new(title).size(TextSize::Small)),
                                )
                            })
                            .when(has_actions, |this| {
                                this.child(
                                    h_flex()
                                        .gap(px(2.0))
                                        .when_some(self.actions_slot, |this, action| {
                                            this.child(action)
                                        })
                                        .when_some(self.dismiss_action, |this, action| {
                                            this.child(action)
                                        }),
                                )
                            }),
                    )
                    .when_some(self.description, |this, desc| {
                        this.child(Label::new(desc).size(TextSize::Small).color(Color::Muted))
                    }),
            )
    }
}
