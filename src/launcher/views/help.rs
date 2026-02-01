use crate::launcher::view::{LauncherView, ViewAction, ViewItem};
use crate::services::Services;
use gpui::App;

pub struct HelpView {
    prefix_char: char,
    views: Vec<HelpEntry>,
}

struct HelpEntry {
    prefix: String,
    name: String,
    icon: String,
    description: String,
}

impl HelpView {
    pub fn new(prefix_char: char, views: &[Box<dyn LauncherView>]) -> Self {
        let entries = views
            .iter()
            .map(|v| HelpEntry {
                prefix: v.prefix().to_string(),
                name: v.name().to_string(),
                icon: v.icon().to_string(),
                description: v.description().to_string(),
            })
            .collect();

        HelpView {
            prefix_char,
            views: entries,
        }
    }
}

impl LauncherView for HelpView {
    fn prefix(&self) -> &'static str {
        "help"
    }

    fn name(&self) -> &'static str {
        "Help"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn items(&self, query: &str, _services: &Services, _cx: &App) -> Vec<ViewItem> {
        self.views
            .iter()
            .filter_map(|entry| {
                let title = format!("{}{}", self.prefix_char, entry.prefix);
                let item = ViewItem::new(&title, &title, &entry.icon)
                    .with_subtitle(&entry.description)
                    .with_action(ViewAction::SwitchView(entry.prefix.clone()));

                if query.is_empty() || item.matches(query) {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    }
}
