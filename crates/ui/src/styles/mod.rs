//! Style helpers shared across components.
//!
//! Today this is just [`ElevationIndex`] - a small enum that maps a
//! semantic z-axis "level" (background, surface, elevated surface, modal)
//! to the box-shadow stack engram components should use at that level.
//! It exists so popovers, modals, and notifications don't each hand-roll
//! their own shadow constants and drift apart over time.

pub mod elevation;

pub use elevation::ElevationIndex;
