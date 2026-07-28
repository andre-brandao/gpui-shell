use gpui::{AnyElement, Context, Window};

use super::style;
use crate::config::ActiveConfig;
use ui::ActiveTheme;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BarWidgetShell {
    None,
    Standard,
    Group,
}

/// Shared rendering contract for widgets that live in the bar.
///
/// Widgets provide vertical and horizontal content variants. The trait owns
/// the bar-specific shell so compact spacing and chrome stay consistent.
pub(crate) trait BarWidget: Sized {
    fn render_vertical(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement;

    fn render_horizontal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement;

    fn is_interactive(&self) -> bool {
        false
    }

    fn shell(&self) -> BarWidgetShell {
        BarWidgetShell::Standard
    }

    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if cx.config().bar.is_vertical() {
            self.render_vertical(window, cx)
        } else {
            self.render_horizontal(window, cx)
        }
    }

    fn wrap_bar_shell(&mut self, cx: &mut Context<Self>, content: AnyElement) -> AnyElement {
        let theme = cx.theme();
        let bar = &cx.config().bar;
        let is_vertical = bar.is_vertical();
        let is_interactive = self.is_interactive();

        match self.shell() {
            BarWidgetShell::None => content,
            BarWidgetShell::Standard => style::bar_widget_slot(
                is_vertical,
                style::bar_widget_shell(theme, bar, is_vertical, is_interactive, content),
            ),
            BarWidgetShell::Group => style::bar_widget_slot(
                is_vertical,
                style::bar_group_shell(theme, bar, is_vertical, is_interactive, content),
            ),
        }
    }

    fn render_bar_widget(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let content = self.render_content(window, cx);
        self.wrap_bar_shell(cx, content)
    }
}
