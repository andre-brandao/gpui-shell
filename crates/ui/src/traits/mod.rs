//! Small behavioural traits shared across components.

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
