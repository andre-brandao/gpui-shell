use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct AccordionStory {
    expanded: [bool; 3],
}

impl AccordionStory {
    fn new() -> Self {
        Self {
            expanded: [true, false, false],
        }
    }
}

impl Render for AccordionStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let sections = ["Getting started", "Configuration", "FAQ"];

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Interactive accordion",
                vec![example(
                    "Toggle sections",
                    Accordion::new()
                        .child({
                            let w = weak.clone();
                            AccordionItem::new("acc-0", sections[0], self.expanded[0])
                                .body(Label::new(
                                    "Install the package and call init() during startup.",
                                ))
                                .on_toggle(move |_, _, cx| {
                                    w.update(cx, |this, cx| {
                                        this.expanded[0] = !this.expanded[0];
                                        cx.notify();
                                    })
                                    .ok();
                                })
                        })
                        .child({
                            let w = weak.clone();
                            AccordionItem::new("acc-1", sections[1], self.expanded[1])
                                .body(Label::new("Set theme, colors, and spacing tokens."))
                                .on_toggle(move |_, _, cx| {
                                    w.update(cx, |this, cx| {
                                        this.expanded[1] = !this.expanded[1];
                                        cx.notify();
                                    })
                                    .ok();
                                })
                        })
                        .child({
                            let w = weak.clone();
                            AccordionItem::new("acc-2", sections[2], self.expanded[2])
                                .body(Label::new("Yes, it works on Wayland."))
                                .on_toggle(move |_, _, cx| {
                                    w.update(cx, |this, cx| {
                                        this.expanded[2] = !this.expanded[2];
                                        cx.notify();
                                    })
                                    .ok();
                                })
                        })
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "States",
                vec![example(
                    "Disabled",
                    AccordionItem::new("acc-dis", "Cannot expand", false)
                        .body(Label::new("Hidden"))
                        .disabled(true)
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| AccordionStory::new()).into()
}
