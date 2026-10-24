//! What a list view hands the launcher. Plain data with no closures or
//! elements, so the launcher owns ranking, rows and clicks.

use std::path::PathBuf;
use std::time::SystemTime;

use gpui::SharedString;
use ui::IconSource;

#[derive(Clone)]
pub enum ItemIcon {
    None,
    Icon(IconSource),
    /// Small image file drawn as-is (app icons).
    Path(PathBuf),
    /// Large image drawn through the shared thumbnail cache.
    Thumbnail {
        path: PathBuf,
        mtime: Option<SystemTime>,
    },
}

/// Something the user can do with an item. The first action runs on Enter
/// and click, the second on Shift+Enter.
#[derive(Clone)]
pub struct ItemAction {
    pub id: SharedString,
    pub label: SharedString,
}

#[derive(Clone)]
pub struct Item {
    /// Stable across refreshes, so the selection survives them.
    pub id: SharedString,
    pub title: SharedString,
    pub subtitle: Option<SharedString>,
    pub icon: ItemIcon,
    /// Searched by the matcher but never shown.
    pub keywords: Option<SharedString>,
    pub actions: Vec<ItemAction>,
}

impl Item {
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            icon: ItemIcon::None,
            keywords: None,
            actions: Vec::new(),
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn icon(mut self, icon: ItemIcon) -> Self {
        self.icon = icon;
        self
    }

    pub fn keywords(mut self, keywords: impl Into<SharedString>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    pub fn action(mut self, id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.actions.push(ItemAction {
            id: id.into(),
            label: label.into(),
        });
        self
    }
}
