use crate::prelude::*;

pub struct TextFieldStory {
    field: Entity<TextField>,
    submitted: SharedString,
}

impl TextFieldStory {
    fn new(cx: &mut Context<Self>) -> Self {
        let weak = cx.entity().downgrade();
        let field = cx.new(|cx| {
            TextField::with_value(cx, "Hello, gpui-shell")
                .placeholder("Type something...")
                .on_submit(move |value, _window, cx| {
                    let value = SharedString::from(value.to_string());
                    weak.update(cx, |this, cx| {
                        this.submitted = value;
                        cx.notify();
                    })
                    .ok();
                })
        });
        Self {
            field,
            submitted: SharedString::default(),
        }
    }
}

impl Render for TextFieldStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Text input (click to focus, type, Enter to submit)",
            vec![example(
                "Default",
                v_flex()
                    .gap(Spacing::Small.pixels())
                    .child(self.field.clone())
                    .child(
                        Label::new(if self.submitted.is_empty() {
                            SharedString::from("Last submitted: (none yet)")
                        } else {
                            SharedString::from(format!("Last submitted: {}", self.submitted))
                        })
                        .size(TextSize::Small)
                        .color(Color::Muted),
                    )
                    .into_any_element(),
            )],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(TextFieldStory::new).into()
}
