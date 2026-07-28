use gpui::{AnyElement, App, div, prelude::*, px};

use super::buffer::{CursorPlacement, InputBuffer};
use crate::ActiveTheme;

pub fn render_input_line(buffer: &InputBuffer, placeholder: &str, cx: &App) -> AnyElement {
    let colors = cx.theme().colors();
    let cursor_el = || div().w(px(1.)).h(px(14.)).bg(colors.text).ml(px(1.));

    if buffer.is_empty() {
        return div()
            .flex()
            .items_center()
            .child(cursor_el())
            .child(
                div()
                    .ml(px(3.))
                    .text_color(colors.text_placeholder)
                    .child(placeholder.to_string()),
            )
            .into_any_element();
    }

    let parts = buffer.plain_render_parts();
    let mut line = div().flex().items_center();

    if !parts.before.is_empty() {
        line = line.child(parts.before.to_string());
    }

    if parts.cursor == CursorPlacement::Between || parts.cursor == CursorPlacement::BeforeSelection
    {
        line = line.child(cursor_el());
    }

    if let Some(selected) = parts.selected {
        line = line.child(
            div()
                .px(px(2.))
                .bg(colors.element_selected)
                .rounded(px(3.))
                .child(selected.to_string()),
        );
    }

    if parts.cursor == CursorPlacement::AfterSelection {
        line = line.child(cursor_el());
    }

    if !parts.after.is_empty() {
        line = line.child(parts.after.to_string());
    }

    line.into_any_element()
}

pub fn render_masked_input_line(
    buffer: &InputBuffer,
    placeholder: &str,
    mask: char,
    cx: &App,
) -> AnyElement {
    let colors = cx.theme().colors();
    let cursor_el = || div().w(px(1.)).h(px(14.)).bg(colors.text).ml(px(1.));

    if buffer.is_empty() {
        return div()
            .flex()
            .items_center()
            .child(cursor_el())
            .child(
                div()
                    .ml(px(3.))
                    .text_color(colors.text_placeholder)
                    .child(placeholder.to_string()),
            )
            .into_any_element();
    }

    let parts = buffer.masked_render_parts(mask);
    let mut line = div().flex().items_center();

    if !parts.before.is_empty() {
        line = line.child(parts.before);
    }

    if parts.cursor == CursorPlacement::Between || parts.cursor == CursorPlacement::BeforeSelection
    {
        line = line.child(cursor_el());
    }

    if let Some(selected) = parts.selected {
        line = line.child(
            div()
                .px(px(1.))
                .bg(colors.element_selected)
                .rounded(px(2.))
                .child(selected),
        );
    }

    if parts.cursor == CursorPlacement::AfterSelection {
        line = line.child(cursor_el());
    }

    if !parts.after.is_empty() {
        line = line.child(parts.after);
    }

    line.into_any_element()
}
