//! Applications view for searching and launching desktop applications.

pub mod config;

use gpui::{AnyElement, App, div, img, prelude::*, px};
use ui::{
    ActiveTheme, Clickable, Color, Label, LabelCommon, ListItem, ListItemSpacing, TextSize,
    Toggleable,
};

use self::config::AppsConfig;
use crate::launcher::view::{LauncherView, ViewContext};
use crate::state::AppState;

/// Applications view - searches and launches desktop applications.
pub struct AppsView {
    prefix: String,
    /// Indices into `ApplicationsService::all()`, refreshed once per frame
    /// by `update_matches`.
    matches: Vec<usize>,
}

impl AppsView {
    pub fn new(config: &AppsConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
            matches: Vec::new(),
        }
    }

    fn app<'a>(&self, index: usize, cx: &'a App) -> Option<&'a services::Application> {
        let ix = *self.matches.get(index)?;
        AppState::applications(cx).all().get(ix)
    }
}

impl LauncherView for AppsView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Applications"
    }

    fn icon(&self) -> &'static str {
        "󰀻"
    }

    fn description(&self) -> &'static str {
        "Search and launch applications"
    }

    fn is_default(&self) -> bool {
        true
    }

    fn update_matches(&mut self, vx: &ViewContext, cx: &App) {
        self.matches = AppState::applications(cx).search_indices(vx.query);
    }

    fn match_count(&self, _vx: &ViewContext, _cx: &App) -> usize {
        self.matches.len()
    }

    fn render_item(&self, index: usize, selected: bool, _vx: &ViewContext, cx: &App) -> AnyElement {
        let Some(app) = self.app(index, cx) else {
            return div().into_any_element();
        };

        let theme = cx.theme();
        let exec = app.exec.clone();
        let interactive_default = theme.colors.element_background;
        let fallback_icon = self.icon();
        ListItem::new(format!("app-{}", app.name))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when_some(app.icon_path.clone(), |el, path| {
                        el.child(img(path).size_full())
                    })
                    .when(app.icon_path.is_none(), move |el| {
                        el.bg(interactive_default)
                            .text_size(px(14.))
                            .child(fallback_icon)
                    }),
            )
            .on_click(move |_, _, _cx| {
                launch_app(&exec);
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(Label::new(app.name.clone()).size(TextSize::Default))
                    .when_some(app.description.as_ref(), |el, desc| {
                        el.child(
                            Label::new(desc.clone())
                                .size(TextSize::Small)
                                .color(Color::Muted),
                        )
                    }),
            )
            .into_any_element()
    }

    fn on_select(&self, index: usize, _vx: &ViewContext, cx: &mut App) -> bool {
        if let Some(app) = self.app(index, cx) {
            launch_app(&app.exec);
            true
        } else {
            false
        }
    }
}

fn launch_app(exec: &str) {
    let exec = exec.to_string();
    std::thread::spawn(move || {
        let exec_cleaned = exec
            .replace("%f", "")
            .replace("%F", "")
            .replace("%u", "")
            .replace("%U", "")
            .replace("%d", "")
            .replace("%D", "")
            .replace("%n", "")
            .replace("%N", "")
            .replace("%i", "")
            .replace("%c", "")
            .replace("%k", "");

        let _ = std::process::Command::new("sh")
            .args(["-c", &exec_cleaned])
            .spawn();
    });
}
