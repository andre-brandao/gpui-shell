//! Banner / Notification - surface alerts inline (`Banner`) or as a
//! short-lived toast card (`Notification`).
//!
//! Both components are stateless `RenderOnce` views: the parent decides
//! when to show / hide them. Their *severity* drives the icon, label color,
//! and a colored leading stripe so they can be skimmed at a glance.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Color, Radius, Spacing, StatusColors};
use gpui::{
    AnyElement, App, ClickEvent, Hsla, IntoElement, ParentElement, RenderOnce, SharedString,
    Window, div, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::styles::ElevationIndex;
use crate::theme::TextSize;
use crate::traits::ClickHandler;

/// Severity of an alert. Drives the leading icon and accent color used by
/// both [`Banner`] and [`Notification`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}

impl Severity {
    fn icon(self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Success => IconName::Check,
            Self::Warning => IconName::Warning,
            Self::Error => IconName::XCircle,
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Info => Color::Info,
            Self::Success => Color::Success,
            Self::Warning => Color::Warning,
            Self::Error => Color::Error,
        }
    }

    /// The status-tinted surface background for this severity. Drives
    /// `Banner`'s body fill so the chrome itself signals the severity, not
    /// just the leading icon.
    fn background(self, status: &StatusColors) -> Hsla {
        match self {
            Self::Info => status.info_background,
            Self::Success => status.success_background,
            Self::Warning => status.warning_background,
            Self::Error => status.error_background,
        }
    }

    /// The status-tinted border for this severity. Pairs with
    /// [`Severity::background`] so the banner outline reinforces the fill.
    fn border(self, status: &StatusColors) -> Hsla {
        match self {
            Self::Info => status.info_border,
            Self::Success => status.success_border,
            Self::Warning => status.warning_border,
            Self::Error => status.error_border,
        }
    }
}

/// An inline message strip with an icon, title, optional description, and
/// optional action area on the right (e.g. a Button).
#[derive(IntoElement)]
#[must_use = "Banner does nothing unless rendered"]
pub struct Banner {
    severity: Severity,
    title: SharedString,
    description: Option<SharedString>,
    actions: SmallVec<[AnyElement; 2]>,
    on_dismiss: Option<ClickHandler>,
}

impl Banner {
    pub fn new(severity: Severity, title: impl Into<SharedString>) -> Self {
        Self {
            severity,
            title: title.into(),
            description: None,
            actions: SmallVec::new(),
            on_dismiss: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Banner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let severity_color = self.severity.color();
        let bg = self.severity.background(&colors.status);
        let border = self.severity.border(&colors.status);
        // Indent the description so it lines up under the title text
        // (skipping past where the icon sits in the row above).
        let description_indent = IconSize::Small.pixels() + Spacing::Medium.pixels();
        div()
            .w_full()
            .px(Spacing::Medium.pixels())
            .py(Spacing::Small.pixels())
            .rounded(Radius::Small.pixels())
            .border_1()
            .border_color(border)
            .bg(bg)
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(
                        // Title row: icon, title, spacer, actions, dismiss
                        // all on one line so they stay vertically aligned.
                        h_flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(self.severity.icon())
                                    .size(IconSize::Small)
                                    .color(severity_color),
                            )
                            .child(Label::new(self.title))
                            .child(div().flex_1())
                            .children(self.actions)
                            .when_some(self.on_dismiss, |this, dismiss| {
                                this.child(
                                    div()
                                        .id("engram-banner-dismiss")
                                        .cursor_pointer()
                                        .ml(Spacing::Small.pixels())
                                        .child(
                                            Icon::new(IconName::Close)
                                                .size(IconSize::Small)
                                                .color(Color::Muted),
                                        )
                                        .on_click(move |event, window, cx| {
                                            dismiss(event, window, cx)
                                        }),
                                )
                            }),
                    )
                    .when_some(self.description, |this, desc| {
                        this.child(
                            div()
                                .pl(description_indent)
                                .child(Label::new(desc).size(TextSize::Small).color(Color::Muted)),
                        )
                    }),
            )
    }
}

/// A floating, dismissible alert card. Same content model as [`Banner`]
/// but with a heavier card style suitable for stacking in a corner of the
/// window.
#[derive(IntoElement)]
#[must_use = "Notification does nothing unless rendered"]
pub struct Notification {
    severity: Severity,
    title: SharedString,
    description: Option<SharedString>,
    on_dismiss: Option<ClickHandler>,
}

impl Notification {
    pub fn new(severity: Severity, title: impl Into<SharedString>) -> Self {
        Self {
            severity,
            title: title.into(),
            description: None,
            on_dismiss: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Notification {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let severity_color = self.severity.color();
        let description_indent = IconSize::Small.pixels() + Spacing::Medium.pixels();
        let has_description = self.description.is_some();
        h_flex()
            .min_w(px(280.0))
            .max_w(px(360.0))
            .when(has_description, |this| this.items_start())
            .gap(Spacing::Medium.pixels())
            .p(Spacing::Medium.pixels())
            .rounded(Radius::Small.pixels())
            .border_1()
            .border_color(colors.border)
            .bg(colors.elevated_surface_background)
            .shadow(ElevationIndex::ElevatedSurface.shadow(cx))
            .child(
                v_flex()
                    .flex_grow(1.)
                    .gap(px(2.0))
                    .child(
                        h_flex()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                Icon::new(self.severity.icon())
                                    .size(IconSize::Small)
                                    .color(severity_color),
                            )
                            .child(Label::new(self.title)),
                    )
                    .when_some(self.description, |this, desc| {
                        this.child(
                            div()
                                .pl(description_indent)
                                .child(Label::new(desc).size(TextSize::Small).color(Color::Muted)),
                        )
                    }),
            )
            .when_some(self.on_dismiss, |this, dismiss| {
                this.child(
                    div()
                        .id("engram-notification-dismiss")
                        .cursor_pointer()
                        .ml(Spacing::Small.pixels())
                        .child(
                            Icon::new(IconName::Close)
                                .size(IconSize::Small)
                                .color(Color::Muted),
                        )
                        .on_click(move |event, window, cx| dismiss(event, window, cx)),
                )
            })
    }
}
