use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct SheetStory;

impl Render for SheetStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Sheet (static, no overlay)",
            vec![
                example(
                    "Right panel",
                    Sheet::new()
                        .side(SheetSide::Right)
                        .title("Details")
                        .width(px(260.0))
                        .child(Label::new("Right-side panel content."))
                        .into_any_element(),
                ),
                example(
                    "Left panel",
                    Sheet::new()
                        .side(SheetSide::Left)
                        .title("Navigation")
                        .width(px(200.0))
                        .child(Label::new("Left-side panel content."))
                        .into_any_element(),
                ),
                example(
                    "Bottom panel",
                    Sheet::new()
                        .side(SheetSide::Bottom)
                        .title("Console")
                        .height(px(120.0))
                        .child(Label::new("Bottom panel content."))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| SheetStory).into()
}
