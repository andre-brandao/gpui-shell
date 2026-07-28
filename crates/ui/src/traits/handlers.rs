//! Shared type aliases for component event handlers.
//!
//! Every interactive component in engram stores its callbacks as
//! `Rc<dyn Fn(...) + 'static>`. Before this module existed, each component
//! re-declared the same type alias (`ClickHandler`, `DismissHandler`,
//! `MenuClickHandler`, ...) with slightly different names - a real maintenance
//! trap when the signature needs to change. Keeping the aliases here means:
//!
//! - there's **one** place to update the handler signature,
//! - component code reads uniformly (`ClickHandler` means the same thing
//!   everywhere),
//! - new components can pick an existing alias instead of minting a new one.
//!
//! All handlers use `Rc` rather than `Box` so that a single handler can be
//! cloned into multiple closures (e.g. `on_click` + `on_key_down`) within a
//! single render pass.
//!
//! If you need a handler shape that isn't represented here, add it - don't
//! re-declare a local alias.

use std::rc::Rc;

use gpui::{AnyView, App, ClickEvent, MouseDownEvent, Window};

use crate::traits::ToggleState;

/// Handler fired on a mouse click. The canonical shape for buttons, list
/// items, tabs, menu entries, disclosure toggles, and modal-backdrop dismisses.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Handler fired with a string payload. Used by text fields for both
/// `on_change` and `on_submit`.
pub type StringHandler = Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// Handler fired when a toggleable element flips state. Used by checkboxes
/// and switches; the handler receives the *new* state after the flip.
pub type ToggleHandler = Rc<dyn Fn(&ToggleState, &mut Window, &mut App) + 'static>;

/// Handler fired when an overlay (modal, popover) wants to close itself.
/// No event payload - it's called from both mouse (backdrop click) and
/// keyboard (Escape) paths, so there's no single meaningful event.
pub type DismissHandler = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

/// Handler fired on the mouse hover-enter and hover-leave events. The
/// boolean payload is `true` when the cursor enters and `false` when it
/// leaves - matches gpui's `Div::on_hover` shape.
pub type HoverHandler = Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>;

/// Handler fired on a raw mouse-down event. Used by list items to bind
/// secondary (right-click) actions where a richer event payload than
/// [`ClickHandler`] is needed.
pub type MouseDownHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// Handler fired with an `f64` payload. Used by [`Stepper`] for value
/// changes (increment/decrement).
pub type F64Handler = Rc<dyn Fn(f64, &mut Window, &mut App) + 'static>;

/// Handler fired with an `f32` payload. Used by [`Slider`] for value
/// changes (drag/click).
pub type F32Handler = Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>;

/// Handler fired with a `usize` payload. Used by [`Pagination`] (page
/// index) and anywhere an index/position callback is needed.
pub type UsizeHandler = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// Builder closure that lazily produces a tooltip view. Used by every
/// component that exposes a `.tooltip(...)` builder method (today: ListItem
/// and ButtonLike). The closure is invoked at hover time so the tooltip
/// view is only created when actually needed.
pub type TooltipBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView + 'static>;
