//! Button family: text [`Button`], icon-only [`IconButton`], and the shared
//! [`ButtonLike`] chrome they both compose.
//!
//! See `button/button_like.rs` for the architectural notes - this file is
//! just the module wiring.

#[allow(clippy::module_inception)]
mod button;
mod button_like;
mod button_link;
mod copy_button;
mod icon_button;
mod split_button;
mod toggle_button;

pub use button::Button;
pub use button_like::{
    ButtonCommon, ButtonLike, ButtonSize, ButtonStyle, SelectableButton, TintColor,
};
pub use button_link::ButtonLink;
pub use copy_button::CopyButton;
pub use icon_button::IconButton;
pub use split_button::{SplitButton, SplitButtonKind, SplitButtonStyle};
pub use toggle_button::{
    ButtonBuilder, ToggleButtonGroup, ToggleButtonGroupStyle, ToggleButtonSimple,
    ToggleButtonWithIcon,
};
