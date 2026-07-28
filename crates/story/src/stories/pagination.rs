use crate::prelude::*;

pub struct PaginationStory;

impl Render for PaginationStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Few pages",
                vec![example(
                    "3 pages, page 2 active",
                    Pagination::new("pg-few")
                        .current_page(2)
                        .total_pages(3)
                        .into_any_element(),
                )],
            ))
            .child(example_group(
                "Many pages (truncation)",
                vec![
                    example(
                        "Page 1 of 20",
                        Pagination::new("pg-start")
                            .current_page(1)
                            .total_pages(20)
                            .into_any_element(),
                    ),
                    example(
                        "Page 10 of 20",
                        Pagination::new("pg-mid")
                            .current_page(10)
                            .total_pages(20)
                            .into_any_element(),
                    ),
                    example(
                        "Page 20 of 20",
                        Pagination::new("pg-end")
                            .current_page(20)
                            .total_pages(20)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Disabled",
                vec![example(
                    "Disabled state",
                    Pagination::new("pg-dis")
                        .current_page(3)
                        .total_pages(10)
                        .disabled(true)
                        .into_any_element(),
                )],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| PaginationStory).into()
}
