//! Small behavioural traits shared across components.
//!
//! These traits (`Clickable`, `Disableable`, `Toggleable`) are deliberately
//! **not** used as generic bounds anywhere in engram. They exist for a
//! different reason: **naming uniformity across unrelated component
//! structs**.
//!
//! Consider the alternative: let every component invent its own method
//! name. `Button::click(...)`, `Checkbox::on_toggle(...)`,
//! `Tab::set_active(true)`, `Disclosure::on_open(...)` - all technically
//! fine, all inconsistent. Callers would have to look up the exact method
//! per component, and the library would drift over time.
//!
//! By routing every "binary state" through [`Toggleable::toggle_state`],
//! every "click-like action" through [`Clickable::on_click`], and every
//! "disabled" through [`Disableable::disabled`], we get a contract that
//! shows up in rustdoc and in IDE autocomplete the same way for every
//! component. That's the whole value. The traits are infrastructure for
//! human authors, not for generic code.
//!
//! If a future component needs genuinely generic trigger behavior (e.g. a
//! `MenuButton<T: Clickable + Toggleable>` wrapper that works with either
//! [`components::button::Button`](../components/button/struct.Button.html)
//! or [`components::button::IconButton`](../components/button/struct.IconButton.html)),
//! the bounds are already in place to make that work without first
//! establishing the contract.

mod clickable;
mod disableable;
pub mod handlers;
mod styled_ext;
mod toggleable;

pub use clickable::Clickable;
pub use disableable::Disableable;
pub use handlers::{
    ClickHandler, DismissHandler, F32Handler, F64Handler, HoverHandler, MouseDownHandler,
    StringHandler, ToggleHandler, TooltipBuilder,
};
pub use styled_ext::StyledExt;
pub use toggleable::{ToggleState, Toggleable};
