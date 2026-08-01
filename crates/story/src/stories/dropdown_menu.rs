use crate::layout::{example, example_group};
use crate::prelude::*;

pub struct DropdownMenuStory {
    basic: Entity<DropdownMenu>,
    styled: Entity<DropdownMenu>,
    no_chevron: Entity<DropdownMenu>,
}

impl DropdownMenuStory {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let basic = cx.new(|cx| {
            DropdownMenu::new("dd-basic", "File", cx, |menu| {
                menu.header("File")
                    .entry_with_icon("dd-new", IconName::Plus, "New File", |_, _| {})
                    .keybinding_entry("dd-save", "Save", ["Ctrl", "S"], |_, _| {})
                    .keybinding_entry(
                        "dd-saveas",
                        "Save As\u{2026}",
                        ["Ctrl", "Shift", "S"],
                        |_, _| {},
                    )
                    .separator()
                    .entry("dd-close", "Close", |_, _| {})
            })
        });

        let styled = cx.new(|cx| {
            DropdownMenu::new("dd-styled", "Actions", cx, |menu| {
                menu.entry("dd-cut", "Cut", |_, _| {})
                    .entry("dd-copy", "Copy", |_, _| {})
                    .entry("dd-paste", "Paste", |_, _| {})
                    .separator()
                    .disabled_entry("dd-unavail", "Unavailable")
            })
            .style(ButtonStyle::Filled)
            .size(ButtonSize::Compact)
            .icon(IconName::ChevronDown)
        });

        let no_chevron = cx.new(|cx| {
            DropdownMenu::new("dd-nochev", "More", cx, |menu| {
                menu.entry("dd-settings", "Settings", |_, _| {}).entry(
                    "dd-about",
                    "About",
                    |_, _| {},
                )
            })
            .style(ButtonStyle::Subtle)
            .no_icon()
        });

        Self {
            basic,
            styled,
            no_chevron,
        }
    }
}

impl Render for DropdownMenuStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "DropdownMenu variants",
            vec![
                example(
                    "Outlined (default)",
                    h_flex().child(self.basic.clone()).into_any_element(),
                ),
                example(
                    "Filled + compact",
                    h_flex().child(self.styled.clone()).into_any_element(),
                ),
                example(
                    "Subtle, no chevron",
                    h_flex().child(self.no_chevron.clone()).into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|cx| DropdownMenuStory::new(window, cx)).into()
}
