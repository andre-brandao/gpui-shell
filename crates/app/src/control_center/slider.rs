use gpui::{
    Bounds, DragMoveEvent, EntityId, EventEmitter, Hsla, MouseButton, MouseDownEvent, Pixels,
    Point, Window, canvas, div, prelude::*, px,
};
use ui::ActiveTheme;
#[derive(Clone, Render)]
pub struct Thumb(EntityId);

#[derive(Debug, Clone)]
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
    track_height: Pixels,
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
            track_height: px(6.0),
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

    pub fn track_height(mut self, h: Pixels) -> Self {
        self.track_height = h;
        self
    }

    pub fn track_color(mut self, color: Hsla) -> Self {
        self.track_color = Some(color);
        self
    }

    pub fn fill_color(mut self, color: Hsla) -> Self {
        self.fill_color = Some(color);
        self
    }

    pub fn thumb_color(mut self, color: Hsla) -> Self {
        self.thumb_color = Some(color);
        self
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

    fn update_thumb_pos(&mut self) {
        self.percentage = if self.max.abs() < f32::EPSILON {
            0.0
        } else {
            self.value.clamp(self.min, self.max) / self.max
        };
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
        track_width: f32,
    ) -> impl IntoElement {
        let entity_id = cx.entity_id();
        let theme = cx.theme();
        let thumb_color = self.thumb_color.unwrap_or(theme.colors().text_accent);
        let thumb_size = px(12.0);

        // Guard against zero/negative track widths during initial layout to avoid clamp panics.
        let safe_track_width = track_width.max(f32::from(thumb_size));
        let thumb_left_val = f32::from(thumb_bar_size) - f32::from(thumb_size) / 2.0;
        let thumb_left = px(thumb_left_val.clamp(0.0, safe_track_width - f32::from(thumb_size)));

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
            .left(thumb_left)
            .size(thumb_size)
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
        let colors = theme.colors();
        let track_color = self.track_color.unwrap_or(colors.element_background);
        let fill_color = self.fill_color.unwrap_or(colors.text_accent);

        let width: f32 = f32::from(self.bounds.size.width);
        let thumb_bar_size = if self.percentage < 0.05 {
            px(0.05 * width)
        } else {
            px(self.percentage * width)
        };

        div().id("slider").child(
            div()
                .flex()
                .items_center()
                .w_full()
                .flex_shrink_0()
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                .child(
                    div()
                        .id("bar")
                        .relative()
                        .w_full()
                        .h(self.track_height)
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
                        .child(self.render_thumb(thumb_bar_size, window, cx, width))
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
