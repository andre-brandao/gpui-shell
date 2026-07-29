use gpui::{AnyElement, Context, IntoElement, Window};

use super::style;
use crate::config::ActiveConfig;
use ui::ActiveTheme;
use ui::patterns::BarChip;

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
        let shell = self.shell();
        if shell == BarWidgetShell::None {
            return content;
        }

        let theme = cx.theme();
        let bar = &cx.config().bar;
        let is_vertical = bar.is_vertical();

        BarChip::new(content)
            .vertical(is_vertical)
            .grouped(shell == BarWidgetShell::Group)
            .background(style::widget_background(theme, bar))
            .border(
                bar.widget_border
                    .then(|| style::widget_border(theme, bar, is_vertical)),
            )
            .hover(
                self.is_interactive()
                    .then(|| style::widget_hover_background(theme, bar)),
            )
            .into_any_element()
    }

    fn render_bar_widget(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let content = self.render_content(window, cx);
        self.wrap_bar_shell(cx, content)
    }
}
