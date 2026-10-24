//! Config sections as signals, so a window reacts to its own section only.

use futures_signals::signal::Mutable;
use gpui::{App, Global};

use super::{ActiveConfig, BarConfig, DockConfig, State};

/// Effective config values, republished with `set_neq` after every reload or
/// state write, so an edit to one section wakes only that section's watchers.
pub struct ConfigSignals {
    pub bar: Mutable<BarConfig>,
    pub dock: Mutable<DockConfig>,
    /// [`State::pinned`]: the UI's list, else `dock.pinned`.
    pub pinned: Mutable<Vec<String>>,
}

impl Global for ConfigSignals {}

impl ConfigSignals {
    pub(super) fn init(cx: &mut App) {
        let signals = Self {
            bar: Mutable::new(cx.config().bar.clone()),
            dock: Mutable::new(cx.config().dock.clone()),
            pinned: Mutable::new(State::pinned(cx).to_vec()),
        };
        cx.set_global(signals);
    }

    #[inline(always)]
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(super) fn publish(cx: &App) {
        let signals = Self::global(cx);
        signals.bar.set_neq(cx.config().bar.clone());
        signals.dock.set_neq(cx.config().dock.clone());
        signals.pinned.set_neq(State::pinned(cx).to_vec());
    }
}
