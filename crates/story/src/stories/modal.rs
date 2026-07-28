use crate::prelude::*;
use gpui::FocusHandle;

pub struct ModalStory {
    modal_open: bool,
    modal_focus: FocusHandle,
}

impl ModalStory {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            modal_open: false,
            modal_focus: cx.focus_handle(),
        }
    }
}

impl Render for ModalStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Modal dialog",
                vec![example(
                    "Click to open",
                    Button::new("btn-open-modal", "Open modal")
                        .style(ButtonStyle::Tinted(TintColor::Accent))
                        .on_click({
                            let weak = weak.clone();
                            move |_event, window, cx| {
                                weak.update(cx, |this, cx| {
                                    this.modal_open = true;
                                    window.focus(&this.modal_focus, cx);
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .into_any_element(),
                )],
            ))
            .when(self.modal_open, |this| {
                let weak = weak.clone();
                let weak_buttons = weak.clone();
                this.child(modal_overlay(
                    self.modal_focus.clone(),
                    Modal::new()
                        .title("Delete file?")
                        .child(Label::new("This action cannot be undone.").color(Color::Muted))
                        .footer(
                            h_flex()
                                .gap(Spacing::Small.pixels())
                                .justify_end()
                                .child(
                                    Button::new("modal-cancel", "Cancel")
                                        .style(ButtonStyle::Subtle)
                                        .on_click({
                                            let weak = weak_buttons.clone();
                                            move |_, _, cx| {
                                                weak.update(cx, |this, cx| {
                                                    this.modal_open = false;
                                                    cx.notify();
                                                })
                                                .ok();
                                            }
                                        }),
                                )
                                .child(
                                    Button::new("modal-delete", "Delete")
                                        .style(ButtonStyle::Tinted(TintColor::Accent))
                                        .on_click({
                                            let weak = weak_buttons.clone();
                                            move |_, _, cx| {
                                                weak.update(cx, |this, cx| {
                                                    this.modal_open = false;
                                                    cx.notify();
                                                })
                                                .ok();
                                            }
                                        }),
                                ),
                        ),
                    move |_window, cx| {
                        weak.update(cx, |this, cx| {
                            this.modal_open = false;
                            cx.notify();
                        })
                        .ok();
                    },
                ))
            })
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(ModalStory::new).into()
}
