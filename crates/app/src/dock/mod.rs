//! Dock UI: pinned + running app icons, shown as a standalone layer-shell
//! window per configured monitor.

pub mod config;

pub use config::{DockConfig, DockHoverEffect, DockMonitors, DockVisibility};
