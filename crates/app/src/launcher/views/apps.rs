//! Applications view — search and launch desktop applications.

use gpui::{AnyElement, App, Context, EventEmitter};
use services::{Application, Services};
use ui::{ActiveTheme, ListItem, ListItemSpacing, prelude::*};

use crate::launcher::view::{LauncherView, ViewEvent};

const APP_DESCRIPTION_MAX_CHARS: usize = 75;

pub struct AppsView {
    services: Services,
    query: String,
    filtered: Vec<Application>,
}

impl EventEmitter<ViewEvent> for AppsView {}

impl AppsView {
    pub fn new(services: Services) -> Self {
        let filtered = services
            .applications
            .search("")
            .into_iter()
            .cloned()
            .collect();
        Self {
            services,
            query: String::new(),
            filtered,
        }
    }
}

impl LauncherView for AppsView {
    fn id(&self) -> &'static str {
        "apps"
    }

    fn prefix(&self) -> &'static str {
        "@"
    }

    fn name(&self) -> &'static str {
        "Applications"
    }

    fn icon(&self) -> IconName {
        IconName::Sparkle
    }

    fn description(&self) -> &'static str {
        "Search and launch applications"
    }

    fn is_default(&self) -> bool {
        true
    }

    fn match_count(&self) -> usize {
        self.filtered.len()
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_lowercase();
        self.filtered = self
            .services
            .applications
            .search(&self.query)
            .into_iter()
            .cloned()
            .collect();
    }

    fn render_item(&self, index: usize, selected: bool, cx: &App) -> AnyElement {
        let Some(app) = self.filtered.get(index) else {
            return gpui::Empty.into_any_element();
        };

        let exec = app.exec.clone();

        ListItem::new(format!("app-{index}"))
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(selected)
            .start_slot(
                div()
                    .w(px(28.))
                    .h(px(28.))
                    .rounded(px(6.))
                    .bg(cx.theme().colors().element_background)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.))
                    .child("\u{f17c}"),
            )
            .on_click(move |_, _, _cx| {
                launch_app(&exec);
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(Label::new(app.name.clone()).size(LabelSize::Default))
                    .child(
                        Label::new(display_description(app.description.as_deref()))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .into_any_element()
    }

    fn confirm(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(app) = self.filtered.get(index) {
            launch_app(&app.exec);
            cx.emit(ViewEvent::Close);
        }
    }
}

fn truncate_description(description: &str, maximum_characters: usize) -> String {
    if description.chars().count() <= maximum_characters {
        return description.to_string();
    }

    let visible_characters = maximum_characters.saturating_sub(3);
    let mut truncated = String::with_capacity(maximum_characters);

    for character in description.chars().take(visible_characters) {
        truncated.push(character);
    }

    truncated.push_str("...");
    truncated
}

fn display_description(description: Option<&str>) -> String {
    match description.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => truncate_description(value, APP_DESCRIPTION_MAX_CHARS),
        None => "...".to_string(),
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
