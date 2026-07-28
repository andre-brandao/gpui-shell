//! Label family: text [`Label`], typographic [`Headline`], and the shared
//! [`LabelLike`] chrome they both compose.
//!
//! See `label/label_like.rs` for the architectural notes - this file is
//! just the module wiring.

mod headline;
mod highlighted_label;
#[allow(clippy::module_inception)]
mod label;
mod label_like;

pub use headline::{Headline, HeadlineSize};
pub use highlighted_label::HighlightedLabel;
pub use label::Label;
pub use label_like::{LabelCommon, LabelLike, LineHeightStyle};
