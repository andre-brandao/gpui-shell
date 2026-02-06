//! Launcher view implementations and registration.

mod apps;
mod help;
mod shell;
mod web;

use gpui::{AppContext, Context};
use services::Services;

use super::Launcher;
use super::view::{LauncherView, ViewEvent, ViewHandle, ViewMeta};

fn register_view<V: LauncherView>(
    entity: gpui::Entity<V>,
    cx: &mut Context<Launcher>,
) -> ViewHandle {
    cx.subscribe(&entity, |launcher, _entity, event: &ViewEvent, cx| {
        launcher.handle_view_event(event, cx);
    })
    .detach();

    ViewHandle::new(entity, cx)
}

/// Create and register all launcher views.
pub fn register_views(services: &Services, cx: &mut Context<Launcher>) -> Vec<ViewHandle> {
    let apps = cx.new(|_cx| apps::AppsView::new(services.clone()));
    let shell = cx.new(|_cx| shell::ShellView::new());
    let web = cx.new(|_cx| web::WebSearchView::new());

    let metas: Vec<ViewMeta> = vec![
        apps.read(cx).meta(),
        shell.read(cx).meta(),
        web.read(cx).meta(),
    ];

    let help = cx.new(|_cx| help::HelpView::new(metas, services.clone()));

    vec![
        register_view(apps, cx),
        register_view(shell, cx),
        register_view(web, cx),
        register_view(help, cx),
    ]
}
