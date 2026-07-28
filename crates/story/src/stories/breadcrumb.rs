use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct BreadcrumbStory;

impl Render for BreadcrumbStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "Breadcrumb",
            vec![
                example(
                    "With icons",
                    Breadcrumb::new()
                        .child(
                            BreadcrumbItem::new("b-home", "Home")
                                .icon(IconName::Home)
                                .on_click(|_, _, _| {}),
                        )
                        .child(
                            BreadcrumbItem::new("b-proj", "Projects")
                                .icon(IconName::Folder)
                                .on_click(|_, _, _| {}),
                        )
                        .child(BreadcrumbItem::new("b-cur", "engram").current(true))
                        .into_any_element(),
                ),
                example(
                    "Text only",
                    Breadcrumb::new()
                        .child(BreadcrumbItem::new("t-root", "Root").on_click(|_, _, _| {}))
                        .child(BreadcrumbItem::new("t-sub", "Nested").on_click(|_, _, _| {}))
                        .child(BreadcrumbItem::new("t-leaf", "Current").current(true))
                        .into_any_element(),
                ),
                example(
                    "Single item",
                    Breadcrumb::new()
                        .child(BreadcrumbItem::new("s-only", "Dashboard").current(true))
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| BreadcrumbStory).into()
}
