//! Application-wide runtime state stored as a GPUI global.

use gpui::{App, Global};
use services::Services;

/// Global runtime state shared across views/widgets.
#[derive(Clone)]
pub struct AppState {
    services: Services,
}

impl Global for AppState {}

impl AppState {
    /// Initialize the global app state.
    pub fn init(services: Services, cx: &mut App) {
        cx.set_global(Self { services });
    }

    /// Get the global app state.
    #[inline(always)]
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Get the shared services container.
    #[inline(always)]
    pub fn services(cx: &App) -> &Services {
        &Self::global(cx).services
    }
}
