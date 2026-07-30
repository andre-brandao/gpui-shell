use crate::prelude::*;

const BALCONY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/demo/avatar.jpg");

pub struct AvatarStory {
    avatar_source: gpui::ImageSource,
}

impl AvatarStory {
    fn new() -> Self {
        Self {
            avatar_source: std::path::PathBuf::from(BALCONY).into(),
        }
    }
}

impl Render for AvatarStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let src = self.avatar_source.clone();

        v_flex()
            .gap(Spacing::Large.pixels())
            .child(example_group(
                "Sizes (monogram)",
                vec![
                    example(
                        "Small",
                        Avatar::new("Ada")
                            .size(AvatarSize::Small)
                            .into_any_element(),
                    ),
                    example("Medium (default)", Avatar::new("Linus").into_any_element()),
                    example(
                        "Large",
                        Avatar::new("Grace")
                            .size(AvatarSize::Large)
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "With image",
                vec![
                    example(
                        "Small",
                        Avatar::new("Ada")
                            .size(AvatarSize::Small)
                            .image(src.clone())
                            .into_any_element(),
                    ),
                    example(
                        "Medium",
                        Avatar::new("Linus").image(src.clone()).into_any_element(),
                    ),
                    example(
                        "Large",
                        Avatar::new("Grace")
                            .size(AvatarSize::Large)
                            .image(src.clone())
                            .into_any_element(),
                    ),
                ],
            ))
            .child(example_group(
                "Facepile",
                vec![
                    example(
                        "Monogram",
                        Facepile::new()
                            .push(Avatar::new("Ada"))
                            .push(Avatar::new("Linus"))
                            .push(Avatar::new("Grace"))
                            .push(Avatar::new("Donald"))
                            .into_any_element(),
                    ),
                    example(
                        "With image",
                        Facepile::new()
                            .push(Avatar::new("Ada").image(src.clone()))
                            .push(Avatar::new("Linus").image(src.clone()))
                            .push(Avatar::new("Grace").image(src.clone()))
                            .into_any_element(),
                    ),
                ],
            ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| AvatarStory::new()).into()
}
