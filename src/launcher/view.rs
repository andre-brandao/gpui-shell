use crate::services::Services;
use gpui::{AnyElement, App, MouseButton, div, prelude::*, px, rgba};

/// Input event passed to views for handling.
#[derive(Clone, Debug)]
pub enum ViewInput {
    /// Character typed.
    Char(String),
    /// Backspace pressed.
    Backspace,
    /// Up arrow pressed.
    Up,
    /// Down arrow pressed.
    Down,
    /// Enter pressed.
    Enter,
}

/// Result of handling input.
pub enum InputResult {
    /// Input was handled, update the query to this value.
    Handled { query: String, close: bool },
    /// Input was not handled, use default behavior.
    Unhandled,
}

/// Context passed to views for rendering.
pub struct ViewContext<'a> {
    pub services: &'a Services,
    pub query: &'a str,
    pub selected_index: usize,
    pub prefix_char: char,
}

/// A launcher view that provides custom rendering and input handling.
pub trait LauncherView: Send + Sync {
    /// The prefix command to activate this view (e.g., "apps", "ws").
    fn prefix(&self) -> &'static str;

    /// Display name for the view.
    fn name(&self) -> &'static str;

    /// Icon for the view (Nerd font).
    fn icon(&self) -> &'static str;

    /// Description shown in help.
    fn description(&self) -> &'static str;

    /// Whether this view is the default when no prefix is given.
    fn is_default(&self) -> bool {
        false
    }

    /// Render the view content. Returns the element and number of selectable items.
    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize);

    /// Handle input. Return InputResult::Handled to consume the input.
    fn handle_input(&self, _input: &ViewInput, _vx: &ViewContext, _cx: &mut App) -> InputResult {
        InputResult::Unhandled
    }

    /// Handle item selection (Enter pressed or clicked).
    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        // Return true to close the launcher
        false
    }
}

/// Action to perform when an item is selected.
#[derive(Clone)]
pub enum ViewAction {
    /// Launch an application by exec command.
    Launch(String),
    /// Focus a workspace by ID.
    FocusWorkspace(i32),
    /// Focus a monitor by ID.
    FocusMonitor(i128),
    /// Toggle WiFi.
    ToggleWifi,
    /// Toggle audio mute.
    ToggleMute,
    /// Adjust volume by delta.
    AdjustVolume(i8),
    /// Switch to a different view prefix.
    SwitchView(String),
    /// No action (for display only).
    None,
}

/// Execute a view action.
pub fn execute_action(action: &ViewAction, services: &Services, cx: &mut App) {
    use crate::services::audio::AudioCommand;
    use crate::services::compositor::types::CompositorCommand;
    use crate::services::network::NetworkCommand;

    match action {
        ViewAction::Launch(exec) => {
            let exec = exec.clone();
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
        ViewAction::FocusWorkspace(id) => {
            services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusWorkspace(*id), cx);
            });
        }
        ViewAction::FocusMonitor(id) => {
            services.compositor.update(cx, |compositor, cx| {
                compositor.dispatch(CompositorCommand::FocusMonitor(*id), cx);
            });
        }
        ViewAction::ToggleWifi => {
            services.network.update(cx, |network, cx| {
                network.dispatch(NetworkCommand::ToggleWiFi, cx);
            });
        }
        ViewAction::ToggleMute => {
            services.audio.update(cx, |audio, cx| {
                audio.dispatch(AudioCommand::ToggleSinkMute, cx);
            });
        }
        ViewAction::AdjustVolume(delta) => {
            services.audio.update(cx, |audio, cx| {
                audio.dispatch(AudioCommand::AdjustSinkVolume(*delta), cx);
            });
        }
        ViewAction::SwitchView(_) | ViewAction::None => {}
    }
}

/// Helper to render a standard list item.
pub fn render_list_item(
    id: impl Into<String>,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    is_selected: bool,
    on_click: impl Fn(&mut App) + 'static,
) -> AnyElement {
    div()
        .id(id.into())
        .w_full()
        .px(px(12.))
        .py(px(8.))
        .rounded(px(6.))
        .cursor_pointer()
        .when(is_selected, |el| el.bg(rgba(0x3b82f6ff)))
        .when(!is_selected, |el| el.hover(|s| s.bg(rgba(0x333333ff))))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
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
                        .child(icon.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child(title.to_string()),
                        )
                        .when_some(subtitle, |el, sub| {
                            el.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgba(0x888888ff))
                                    .child(sub.to_string()),
                            )
                        }),
                ),
        )
        .into_any_element()
}

/// Helper to render a standard list of items.
pub fn render_item_list(
    items: Vec<(
        String,
        String,
        String,
        Option<String>,
        Box<dyn Fn(&mut App) + 'static>,
    )>,
    selected_index: usize,
) -> AnyElement {
    div()
        .flex_1()
        .overflow_hidden()
        .flex()
        .flex_col()
        .gap(px(4.))
        .children(items.into_iter().enumerate().map(
            |(i, (id, icon, title, subtitle, on_click))| {
                render_list_item(
                    id,
                    &icon,
                    &title,
                    subtitle.as_deref(),
                    i == selected_index,
                    on_click,
                )
            },
        ))
        .into_any_element()
}
