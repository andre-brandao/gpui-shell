//! Dock item model: grouping running windows by application, merged with
//! the persisted pinned-apps list.

use std::path::PathBuf;

use services::applications::match_app_id;

/// One dock icon: either a pinned app (possibly not running), a running
/// app (possibly pinned too, in which case it's merged, not duplicated),
/// or a running app not found in the catalog (grouped by its raw app_id).
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct DockItem {
    /// Stable identity for this item: a pinned app's desktop file name, or
    /// the resolved app's desktop file name, or the raw window app_id as a
    /// last resort.
    pub key: String,
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub is_pinned: bool,
    pub windows: Vec<services::Window>,
    /// `Some` when this item can be launched (found in the app catalog).
    pub exec: Option<String>,
}

impl DockItem {
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        !self.windows.is_empty()
    }
}

/// The stable identity used to key a pinned entry: the desktop file's own
/// file name (e.g. `"firefox.desktop"`), falling back to the app's display
/// name if the path has no file name for some reason (shouldn't happen in
/// practice - `desktop_file` always comes from a real directory scan).
#[allow(dead_code)]
pub fn desktop_file_id(app: &services::Application) -> String {
    app.desktop_file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&app.name)
        .to_string()
}

/// Build the dock's item list: pinned entries first (in configured order),
/// then any running window whose resolved (or raw) identity isn't already
/// present, in first-seen order.
#[allow(dead_code)]
pub fn build_dock_items(
    windows: &[services::Window],
    apps: &[services::Application],
    pinned: &[String],
) -> Vec<DockItem> {
    let mut items: Vec<DockItem> = Vec::new();

    for pin_id in pinned {
        if let Some(app) = apps.iter().find(|a| desktop_file_id(a) == *pin_id) {
            items.push(DockItem {
                key: pin_id.clone(),
                name: app.name.clone(),
                icon_path: app.icon_path.clone(),
                is_pinned: true,
                windows: Vec::new(),
                exec: Some(app.exec.clone()),
            });
        }
    }

    for window in windows {
        let matched = match_app_id(apps, &window.app_id);
        let key = matched
            .map(desktop_file_id)
            .unwrap_or_else(|| window.app_id.clone());

        if let Some(item) = items.iter_mut().find(|i| i.key == key) {
            item.windows.push(window.clone());
        } else {
            items.push(DockItem {
                key: key.clone(),
                name: matched
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| window.app_id.clone()),
                icon_path: matched.and_then(|a| a.icon_path.clone()),
                is_pinned: false,
                windows: vec![window.clone()],
                exec: matched.map(|a| a.exec.clone()),
            });
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::{build_dock_items, desktop_file_id};
    use std::path::PathBuf;

    fn app(name: &str, exec: &str, desktop_file: &str) -> services::Application {
        services::Application {
            name: name.to_string(),
            exec: exec.to_string(),
            icon: None,
            icon_path: None,
            description: None,
            desktop_file: PathBuf::from(format!("/usr/share/applications/{desktop_file}")),
            startup_wm_class: None,
        }
    }

    fn window(id: &str, app_id: &str) -> services::Window {
        services::Window {
            id: id.to_string(),
            app_id: app_id.to_string(),
            title: format!("{app_id} window"),
            monitor: "eDP-1".to_string(),
            workspace_id: 1,
            is_focused: false,
            is_minimized: false,
            geometry: None,
        }
    }

    #[test]
    fn pinned_app_with_no_running_windows_shows_as_not_running() {
        let apps = vec![app("Firefox", "firefox", "firefox.desktop")];
        let items = build_dock_items(&[], &apps, &["firefox.desktop".to_string()]);
        assert_eq!(items.len(), 1);
        assert!(items[0].is_pinned);
        assert!(items[0].windows.is_empty());
        assert_eq!(items[0].name, "Firefox");
    }

    #[test]
    fn pinned_app_that_is_running_merges_into_one_item() {
        let apps = vec![app("Firefox", "firefox", "firefox.desktop")];
        let windows = vec![window("0x1", "firefox")];
        let items = build_dock_items(&windows, &apps, &["firefox.desktop".to_string()]);
        assert_eq!(items.len(), 1);
        assert!(items[0].is_pinned);
        assert_eq!(items[0].windows.len(), 1);
    }

    #[test]
    fn running_unpinned_app_windows_are_grouped_by_matched_app() {
        let apps = vec![app("Kitty", "kitty", "kitty.desktop")];
        let windows = vec![window("0x1", "kitty"), window("0x2", "kitty")];
        let items = build_dock_items(&windows, &apps, &[]);
        assert_eq!(items.len(), 1);
        assert!(!items[0].is_pinned);
        assert_eq!(items[0].windows.len(), 2);
        assert_eq!(items[0].exec.as_deref(), Some("kitty"));
    }

    #[test]
    fn unmatched_window_falls_back_to_raw_app_id_grouping() {
        let windows = vec![window("0x1", "some-unknown-app")];
        let items = build_dock_items(&windows, &[], &[]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "some-unknown-app");
        assert_eq!(items[0].exec, None);
        assert!(!items[0].is_pinned);
    }

    #[test]
    fn desktop_file_id_uses_the_file_name() {
        let a = app("Firefox", "firefox", "firefox.desktop");
        assert_eq!(desktop_file_id(&a), "firefox.desktop");
    }
}
