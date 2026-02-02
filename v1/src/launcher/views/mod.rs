mod apps;
mod control;
mod help;
mod monitors;
mod workspaces;

pub use apps::AppsView;
pub use control::ControlView;
pub use help::HelpView;
pub use monitors::MonitorsView;
pub use workspaces::WorkspacesView;

use super::view::{LauncherView, ViewObserver};
use crate::services::Services;
use gpui::Context;

/// Create all available views.
pub fn all_views() -> Vec<Box<dyn LauncherView>> {
    vec![
        Box::new(AppsView),
        Box::new(WorkspacesView),
        Box::new(MonitorsView),
        Box::new(ControlView),
    ]
}

/// Register observers for all views on the given entity.
///
/// This ensures each view only observes the services it needs,
/// rather than the launcher observing everything.
pub fn register_all_observers<T: 'static>(services: &Services, cx: &mut Context<T>) {
    AppsView::observe_services(services, cx);
    WorkspacesView::observe_services(services, cx);
    MonitorsView::observe_services(services, cx);
    ControlView::observe_services(services, cx);
    HelpView::observe_services(services, cx);
}
