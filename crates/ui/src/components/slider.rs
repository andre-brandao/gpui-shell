use gpui::*;

use crate::{ActiveTheme, h_flex};

#[derive(Clone, Render)]
pub struct Thumb(EntityId);

pub enum SliderEvent {
    Change(f32),
}

pub struct Slider {
    min: f32,
    max: f32,
    step: f32,
    value: f32,
    percentage: f32,
    bounds: Bounds<Pixels>,
    track_color: Option<Hsla>,
    fill_color: Option<Hsla>,
    thumb_color: Option<Hsla>,
}

impl EventEmitter<SliderEvent> for Slider {}

impl Default for Slider {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            value: 0.0,
            percentage: 0.0,
            bounds: Bounds::default(),
            track_color: None,
            fill_color: None,
            thumb_color: None,
        }
    }
}

impl Slider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self.update_thumb_pos();
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self.update_thumb_pos();
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the track (background) color
    pub fn track_color(mut self, color: Hsla) -> Self {
        self.track_color = Some(color);
        self
    }

    /// Set the fill (progress) color
    pub fn fill_color(mut self, color: Hsla) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set the thumb color
    pub fn thumb_color(mut self, color: Hsla) -> Self {
        self.thumb_color = Some(color);
        self
    }

    fn update_thumb_pos(&mut self) {
        self.percentage = self.value.clamp(self.min, self.max) / self.max;
    }

    pub fn default_value(mut self, value: f32) -> Self {
        self.value = value;
        self.update_thumb_pos();
        self
    }

    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        self.value = value;
        self.update_thumb_pos();
        cx.notify();
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    fn update_value_by_position(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bounds = self.bounds;
        let min = self.min;
        let max = self.max;
        let step = self.step;

        let percentage =
            (position.x - bounds.left()).clamp(px(0.), bounds.size.width) / bounds.size.width;

        let value = min + percentage * (max - min);

        let value = (value / step).round() * step;

        self.percentage = percentage;
        self.value = value.clamp(self.min, self.max);
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_value_by_position(event.position, window, cx);
    }

    fn render_thumb(
        &self,
        thumb_bar_size: Pixels,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let entity_id = cx.entity_id();
        let theme = cx.theme();
        let thumb_color = self.thumb_color.unwrap_or(theme.accent.primary);

        div()
            .id("thumb")
            .on_drag(Thumb(entity_id), |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(
                cx.listener(
                    move |view, e: &DragMoveEvent<Thumb>, window, cx| match e.drag(cx) {
                        Thumb(id) => {
                            if *id != entity_id {
                                return;
                            }

                            view.update_value_by_position(e.event.position, window, cx);
                        }
                    },
                ),
            )
            .absolute()
            .left(thumb_bar_size)
            .top(px(-3.))
            .ml_neg_2()
            .size_3()
            .border_1()
            .border_color(thumb_color)
            .rounded_full()
            .shadow_md()
            .bg(thumb_color)
    }
}

impl Render for Slider {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let track_color = self.track_color.unwrap_or(theme.bg.tertiary);
        let fill_color = self.fill_color.unwrap_or(theme.accent.primary);

        let thumb_bar_size = if self.percentage < 0.05 {
            0.05 * self.bounds.size.width
        } else {
            self.percentage * self.bounds.size.width
        };

        div().id("slider").child(
            h_flex()
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                .items_center()
                .w_full()
                .flex_shrink_0()
                .child(
                    div()
                        .id("bar")
                        .relative()
                        .w_full()
                        .h(px(6.))
                        .bg(track_color)
                        .rounded(px(3.))
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .h_full()
                                .w(thumb_bar_size)
                                .bg(fill_color)
                                .rounded(px(3.)),
                        )
                        .child(self.render_thumb(thumb_bar_size, window, cx))
                        .child({
                            let view = cx.entity().clone();
                            canvas(
                                move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds),
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full()
                        }),
                ),
        )
    }
}
