use crate::services::applications::{Application, Applications};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton,
    Point, Size, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, actions, div, layer_shell::*, prelude::*, px, rgba,
};

actions!(launcher, [Escape, Enter]);

const LAUNCHER_WIDTH: f32 = 600.0;
const LAUNCHER_HEIGHT: f32 = 400.0;

pub struct Launcher {
    applications: Entity<Applications>,
    search_query: String,
    selected_index: usize,
    focus_handle: FocusHandle,
}

impl Launcher {
    pub fn new(applications: Entity<Applications>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        Launcher {
            applications,
            search_query: String::new(),
            selected_index: 0,
            focus_handle,
        }
    }

    fn filtered_apps(&self, cx: &Context<Self>) -> Vec<Application> {
        let apps = self.applications.read(cx);
        apps.search(&self.search_query)
            .into_iter()
            .cloned()
            .collect()
    }

    fn launch_selected(&mut self, cx: &mut Context<Self>, window: &mut Window) {
        let apps = self.filtered_apps(cx);
        if let Some(app) = apps.get(self.selected_index) {
            app.launch();
            window.remove_window();
        }
    }

    fn launch_app(&mut self, app: &Application, window: &mut Window) {
        app.launch();
        window.remove_window();
    }

    fn move_selection(&mut self, delta: i32, cx: &Context<Self>) {
        let apps = self.filtered_apps(cx);
        if apps.is_empty() {
            self.selected_index = 0;
            return;
        }

        let len = apps.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let apps = self.filtered_apps(cx);
        let selected_index = self.selected_index;
        let query = self.search_query.clone();

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|_, _: &Escape, window, _cx| {
                window.remove_window();
            }))
            .on_action(cx.listener(|this, _: &Enter, window, cx| {
                this.launch_selected(cx, window);
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
                    "backspace" => {
                        this.search_query.pop();
                        this.selected_index = 0;
                        cx.notify();
                    }
                    _ => {
                        // Handle text input
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
            // Search input display
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
                                        "Search applications...".to_string()
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
            // App list
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(apps.into_iter().enumerate().map(|(i, app)| {
                        let is_selected = i == selected_index;
                        let app_for_click = app.clone();

                        div()
                            .id(format!("app-{}", i))
                            .w_full()
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
                            .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, _cx| {
                                    this.launch_app(&app_for_click, window);
                                }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    // Icon placeholder
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
                                            .child(
                                                app.name.chars().next().unwrap_or('?').to_string(),
                                            ),
                                    )
                                    // App info
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(14.))
                                                    .font_weight(gpui::FontWeight::MEDIUM)
                                                    .child(app.name.clone()),
                                            )
                                            .when_some(app.description.clone(), |el, desc| {
                                                el.child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .text_color(rgba(0x888888ff))
                                                        .child(desc),
                                                )
                                            }),
                                    ),
                            )
                    })),
            )
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
pub fn toggle(applications: Entity<Applications>, cx: &mut App) {
    let mut guard = LAUNCHER_WINDOW.lock().unwrap();

    if let Some(handle) = guard.take() {
        // Close existing launcher
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
    } else {
        // Open new launcher
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
            move |_, cx| cx.new(|cx| Launcher::new(applications.clone(), cx)),
        ) {
            *guard = Some(handle);
        }
    }
}

/// Open the launcher (convenience for when you don't need toggle).
pub fn open(applications: Entity<Applications>, cx: &mut App) {
    toggle(applications, cx);
}
