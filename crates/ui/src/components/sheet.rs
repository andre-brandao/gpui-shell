//! Sheet - a panel overlay that slides in from a window edge.
//!
//! Follows the same pattern as [`Modal`](super::modal): the parent owns the
//! open state and conditionally inserts a [`sheet_overlay`] in the element
//! tree. The overlay draws a full-window backdrop (click or Escape to
//! dismiss) and positions the sheet content along one edge.
//!
//! ```ignore
//! .when(self.sheet_open, |this| {
//!     this.child(sheet_overlay(
//!         self.sheet_focus.clone(),
//!         Sheet::new().title("Details").side(SheetSide::Right)
//!             .child(Label::new("Panel content")),
//!         cx.listener(|this, _, _, cx| { this.sheet_open = false; cx.notify(); }),
//!     ))
//! })
//! ```

use crate::theme::{ActiveTheme, Spacing};
use gpui::{
    AnyElement, App, FocusHandle, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Window, hsla, prelude::*, px,
};
use smallvec::SmallVec;

use crate::components::label::{Label, LabelCommon};
use crate::components::overlay::{
    OVERLAY_PRIORITY_MODAL, OverlayConfig, OverlayPlacement, overlay_shell,
};
use crate::components::stack::v_flex;
use crate::styles::ElevationIndex;
use crate::theme::TextSize;

/// Which window edge the sheet attaches to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SheetSide {
    #[default]
    Right,
    Left,
    Bottom,
}

/// A panel overlay surface.
#[derive(IntoElement)]
#[must_use = "Sheet does nothing unless rendered"]
pub struct Sheet {
    side: SheetSide,
    title: Option<SharedString>,
    children: SmallVec<[AnyElement; 4]>,
    width: Pixels,
    /// Only used for `SheetSide::Bottom`.
    height: Pixels,
}

impl Sheet {
    pub fn new() -> Self {
        Self {
            side: SheetSide::default(),
            title: None,
            children: SmallVec::new(),
            width: px(360.0),
            height: px(300.0),
        }
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.side = side;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the panel width (used for Left/Right sides).
    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    /// Set the panel height (used for Bottom side).
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }
}

impl Default for Sheet {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Sheet {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();

        let panel = v_flex()
            .gap(Spacing::Medium.pixels())
            .p(Spacing::Large.pixels())
            .bg(colors.elevated_surface_background)
            .border_1()
            .border_color(colors.border)
            .shadow(ElevationIndex::ModalSurface.shadow(cx))
            .when_some(self.title, |this, title| {
                this.child(Label::new(title).size(TextSize::Large))
            })
            .child(
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .children(self.children),
            );

        // Size the panel based on the side it attaches to.
        match self.side {
            SheetSide::Right | SheetSide::Left => panel.w(self.width).h_full(),
            SheetSide::Bottom => panel.w_full().h(self.height),
        }
    }
}

/// Wrap a [`Sheet`] in a full-window backdrop layer, similar to
/// [`modal_overlay`](super::modal::modal_overlay).
pub fn sheet_overlay(
    focus_handle: FocusHandle,
    sheet: Sheet,
    on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let side = sheet.side;
    overlay_shell(
        OverlayConfig {
            id: "engram-sheet-backdrop",
            focus_handle,
            priority: OVERLAY_PRIORITY_MODAL,
            backdrop: Some(hsla(0.0, 0.0, 0.0, 0.35)),
            placement: OverlayPlacement::Edge(side),
        },
        on_dismiss,
        sheet,
    )
}
