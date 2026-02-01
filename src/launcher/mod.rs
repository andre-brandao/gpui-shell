mod item;

use crate::services::Services;
use gpui::{
    App, AppContext, Bounds, Context, FocusHandle, Focusable, KeyBinding, MouseButton, Point, Size,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    actions, div, layer_shell::*, prelude::*, px, rgba,
};
use item::{Category, LauncherItem, SystemAction};

actions!(launcher, [Escape, Enter]);

const LAUNCHER_WIDTH: f32 = 600.0;
const LAUNCHER_HEIGHT: f32 = 450.0;

pub struct Launcher {
    services: Services,
    search_query: String,
    selected_index: usize,
    active_category: Option<Category>,
    focus_handle: FocusHandle,
}

impl Launcher {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        // Observe all services for updates
        cx.observe(&services.applications, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.compositor, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&services.audio, |_, _, cx| cx.notify()).detach();
        cx.observe(&services.network, |_, _, cx| cx.notify())
            .detach();

        Launcher {
            services,
            search_query: String::new(),
            selected_index: 0,
            active_category: None,
            focus_handle,
        }
    }

    fn get_all_items(&self, cx: &Context<Self>) -> Vec<LauncherItem> {
        let mut items = Vec::new();

        // Apps
        let apps = self.services.applications.read(cx);
        for app in &apps.apps {
            items.push(LauncherItem::App(app.clone()));
        }

        // Workspaces
        let compositor = self.services.compositor.read(cx);
        for ws in &compositor.workspaces {
            if !ws.is_special {
                items.push(LauncherItem::Workspace(ws.clone()));
            }
        }

        // Monitors
        for mon in &compositor.monitors {
            items.push(LauncherItem::Monitor(mon.clone()));
        }

        // System actions
        for action in SystemAction::all() {
            items.push(LauncherItem::System(action));
        }

        items
    }

    fn filtered_items(&self, cx: &Context<Self>) -> Vec<LauncherItem> {
        let all_items = self.get_all_items(cx);

        // Parse query for prefix filters
        let (category_filter, search_query) = self.parse_query();

        all_items
            .into_iter()
            .filter(|item| {
                // Category filter
                if let Some(cat) = category_filter {
                    if item.category() != cat {
                        return false;
                    }
                } else if let Some(active) = self.active_category {
                    if item.category() != active {
                        return false;
                    }
                }

                // Search filter
                item.matches(&search_query)
            })
            .collect()
    }

    fn parse_query(&self) -> (Option<Category>, String) {
        let query = self.search_query.trim();
        if query.is_empty() {
            return (None, String::new());
        }

        let first_char = query.chars().next().unwrap();
        match first_char {
            '@' => (Some(Category::Windows), query[1..].to_string()),
            '#' => (Some(Category::Workspaces), query[1..].to_string()),
            '!' => (Some(Category::Monitors), query[1..].to_string()),
            '>' => (Some(Category::System), query[1..].to_string()),
            _ => (None, query.to_string()),
        }
    }

    fn execute_selected(&mut self, cx: &mut Context<Self>, window: &mut Window) {
        let items = self.filtered_items(cx);
        if let Some(item) = items.get(self.selected_index) {
            item.execute(&self.services, cx);
            window.remove_window();
        }
    }

    fn execute_item(&mut self, item: &LauncherItem, cx: &mut Context<Self>, window: &mut Window) {
        item.execute(&self.services, cx);
        window.remove_window();
    }

    fn move_selection(&mut self, delta: i32, cx: &Context<Self>) {
        let items = self.filtered_items(cx);
        if items.is_empty() {
            self.selected_index = 0;
            return;
        }

        let len = items.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
    }

    fn set_category(&mut self, category: Option<Category>) {
        self.active_category = category;
        self.selected_index = 0;
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self.filtered_items(cx);
        let selected_index = self.selected_index;
        let query = self.search_query.clone();
        let active_category = self.active_category;

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                this.execute_selected(cx, window);
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "up" => {
                        this.move_selection(-1, cx);
                        cx.notify();
                    }
                    "down" => {
                        this.move_selection(1, cx);
                        cx.notify();
                    }
                    "tab" => {
                        // Cycle through categories
                        let categories = [
                            None,
                            Some(Category::Apps),
                            Some(Category::Workspaces),
                            Some(Category::Monitors),
                            Some(Category::System),
                        ];
                        let current_idx = categories
                            .iter()
                            .position(|c| *c == this.active_category)
                            .unwrap_or(0);
                        let next_idx = (current_idx + 1) % categories.len();
                        this.set_category(categories[next_idx]);
                        cx.notify();
                    }
                    "backspace" => {
                        this.search_query.pop();
                        this.selected_index = 0;
                        cx.notify();
                    }
                    _ => {
                        if let Some(key_char) = &event.keystroke.key_char {
                            this.search_query.push_str(key_char);
                            this.selected_index = 0;
                            cx.notify();
                        } else if event.keystroke.key.len() == 1
                            && !event.keystroke.modifiers.control
                            && !event.keystroke.modifiers.alt
                        {
                            this.search_query.push_str(&event.keystroke.key);
                            this.selected_index = 0;
                            cx.notify();
                        }
                    }
                }
            }))
            .size_full()
            .bg(rgba(0x1a1a1aee))
            .border_1()
            .border_color(rgba(0x333333ff))
            .rounded(px(12.))
            .p(px(16.))
            .text_color(rgba(0xffffffff))
            .flex()
            .flex_col()
            .gap(px(12.))
            // Search input
            .child(
                div()
                    .w_full()
                    .px(px(12.))
                    .py(px(10.))
                    .bg(rgba(0x333333ff))
                    .rounded(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().text_size(px(16.)).child(""))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(14.))
                                    .child(if query.is_empty() {
                                        "Search apps, workspaces, actions...".to_string()
                                    } else {
                                        query
                                    })
                                    .text_color(if self.search_query.is_empty() {
                                        rgba(0x888888ff)
                                    } else {
                                        rgba(0xffffffff)
                                    }),
                            ),
                    ),
            )
            // Category tabs
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(self.render_category_tab(None, active_category, cx))
                    .child(self.render_category_tab(Some(Category::Apps), active_category, cx))
                    .child(self.render_category_tab(
                        Some(Category::Workspaces),
                        active_category,
                        cx,
                    ))
                    .child(self.render_category_tab(Some(Category::Monitors), active_category, cx))
                    .child(self.render_category_tab(Some(Category::System), active_category, cx)),
            )
            // Items list
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(items.into_iter().enumerate().map(|(i, item)| {
                        let is_selected = i == selected_index;
                        let item_for_click = item.clone();

                        div()
                            .id(item.id())
                            .w_full()
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                            .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    this.execute_item(&item_for_click, cx, window);
                                }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(12.))
                                            // Icon
                                            .child(
                                                div()
                                                    .w(px(32.))
                                                    .h(px(32.))
                                                    .rounded(px(6.))
                                                    .bg(rgba(0x444444ff))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_size(px(16.))
                                                    .child(item.icon()),
                                            )
                                            // Title and subtitle
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(2.))
                                                    .child(
                                                        div()
                                                            .text_size(px(14.))
                                                            .font_weight(gpui::FontWeight::MEDIUM)
                                                            .child(item.title()),
                                                    )
                                                    .when_some(item.subtitle(), |el, subtitle| {
                                                        el.child(
                                                            div()
                                                                .text_size(px(12.))
                                                                .text_color(rgba(0x888888ff))
                                                                .child(subtitle),
                                                        )
                                                    }),
                                            ),
                                    )
                                    // Category badge
                                    .child(
                                        div()
                                            .px(px(8.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(rgba(0x444444ff))
                                            .text_size(px(10.))
                                            .text_color(rgba(0x888888ff))
                                            .child(item.category().label()),
                                    ),
                            )
                    })),
            )
    }
}

impl Launcher {
    fn render_category_tab(
        &self,
        category: Option<Category>,
        active: Option<Category>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = category == active;
        let label = category.map(|c| c.label()).unwrap_or("All");
        let icon = category.map(|c| c.icon()).unwrap_or("󰣆");

        div()
            .id(format!("tab-{}", label))
            .flex()
            .items_center()
            .gap(px(4.))
            .px(px(10.))
            .py(px(6.))
            .rounded(px(6.))
            .cursor_pointer()
            .when(is_active, |el| el.bg(rgba(0x3b82f6ff)))
            .when(!is_active, |el| {
                el.bg(rgba(0x333333ff)).hover(|s| s.bg(rgba(0x444444ff)))
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.set_category(category);
                    cx.notify();
                }),
            )
            .child(div().text_size(px(12.)).child(icon))
            .child(div().text_size(px(12.)).child(label))
    }
}

/// Global state to track the launcher window.
static LAUNCHER_WINDOW: std::sync::Mutex<Option<WindowHandle<Launcher>>> =
    std::sync::Mutex::new(None);

/// Register keybindings for the launcher.
pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Escape, Some("Launcher")),
        KeyBinding::new("enter", Enter, Some("Launcher")),
    ]);
}

/// Toggle the launcher window.
pub fn toggle(services: Services, cx: &mut App) {
    let mut guard = LAUNCHER_WINDOW.lock().unwrap();

    if let Some(handle) = guard.take() {
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
    } else {
        if let Ok(handle) = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(0.), px(0.)),
                    size: Size::new(px(LAUNCHER_WIDTH), px(LAUNCHER_HEIGHT)),
                })),
                app_id: Some("gpui-launcher".to_string()),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "launcher".to_string(),
                    layer: Layer::Overlay,
                    anchor: Anchor::TOP,
                    exclusive_zone: None,
                    margin: Some((px(100.), px(0.), px(0.), px(0.))),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| Launcher::new(services.clone(), cx)),
        ) {
            *guard = Some(handle);
        }
    }
}

/// Open the launcher.
pub fn open(services: Services, cx: &mut App) {
    toggle(services, cx);
}
