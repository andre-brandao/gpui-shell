//! Composite shell surfaces - whole UIs rather than widgets.
//!
//! Everything under [`components`](crate::components) is a primitive: it
//! assumes nothing about what it is put inside. The patterns here assume
//! plenty. A launcher frame is a launcher, not a generic panel, and it is
//! shaped by decisions - the query line on top, the hint bar at the bottom,
//! the badge on the right - that only make sense for this shell.
//!
//! They are deliberately kept out of the crate-root glob: `use ui::*` pulls
//! in `components`, never `patterns`. Reaching for one is always spelled
//! out, `ui::patterns::LauncherFrame`, so a bespoke surface can never be
//! mistaken for part of the primitive set.
//!
//! Patterns own presentation only. State, keybindings and behaviour stay in
//! the app crate that drives them.

pub mod bar;
pub mod launcher;
pub mod osd;
pub mod surface;

pub use bar::{BarChip, BarEdge, BarSurface};
pub use launcher::{LauncherFrame, footer_hints};
pub use osd::OsdIndicator;
pub use surface::PanelSurface;
