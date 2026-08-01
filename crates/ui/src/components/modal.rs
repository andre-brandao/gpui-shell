//! Modal - centered overlay card with a dimmed backdrop.

use crate::theme::{ActiveTheme, Radius, Spacing};
use gpui::{
    AnyElement, App, FocusHandle, Hsla, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Window, hsla, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::label::{Label, LabelCommon};
use crate::components::overlay::{
    OVERLAY_PRIORITY_MODAL, OverlayConfig, OverlayPlacement, overlay_shell,
};
use crate::components::stack::v_flex;
use crate::styles::ElevationIndex;
use crate::theme::TextSize;

fn backdrop() -> Hsla {
    hsla(0.0, 0.0, 0.0, 0.45)
}

/// A centered card with optional title, body children, and footer row.
#[derive(IntoElement)]
#[must_use = "Modal does nothing unless rendered"]
pub struct Modal {
    title: Option<SharedString>,
    children: SmallVec<[AnyElement; 4]>,
    footer: Option<AnyElement>,
    width: Pixels,
}

impl Modal {
    pub fn new() -> Self {
        Self {
            title: None,
            children: SmallVec::new(),
            footer: None,
            width: px(420.0),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }
}

impl Default for Modal {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Modal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Modal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        v_flex()
            .w(self.width)
            .gap(Spacing::Medium.pixels())
            .p(Spacing::Large.pixels())
            .rounded(Radius::Large.pixels())
            .border_1()
            .border_color(colors.border)
            .bg(colors.elevated_surface_background)
            .shadow(ElevationIndex::ModalSurface.shadow(cx))
            .when_some(self.title, |this, title| {
                this.child(Label::new(title).size(TextSize::Large))
            })
            .child(
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .children(self.children),
            )
            .when_some(self.footer, |this, footer| this.child(footer))
    }
}

/// Wrap a [`Modal`] (or any element) in a full-window backdrop layer that dismisses on
/// **backdrop click** or **`Escape`**.
pub fn modal_overlay(
    focus_handle: FocusHandle,
    content: impl IntoElement,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    overlay_shell(
        OverlayConfig {
            id: "modal-backdrop",
            focus_handle,
            priority: OVERLAY_PRIORITY_MODAL,
            backdrop: Some(backdrop()),
            placement: OverlayPlacement::Centered,
        },
        on_dismiss,
        content,
    )
}
