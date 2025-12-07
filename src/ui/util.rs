use std::{cell::RefCell, time::Instant};

use egui::*;

pub trait UiExt {
    #[must_use]
    fn d_button(&mut self, text: impl Into<RichText>) -> Response;

    fn d_spinner(&mut self, size: Vec2) -> Response;

    fn section_separator(&mut self, text: impl Into<RichText>);
}

impl UiExt for Ui {
    fn d_button(&mut self, text: impl Into<RichText>) -> Response {
        let r = self
            .add(
                Button::new(text.into().color(Color32::BLACK))
                    .min_size(vec2(120.0, 60.0))
                    .corner_radius(8)
                    .fill(Color32::WHITE),
            )
            .on_hover_cursor(CursorIcon::PointingHand);

        if r.hovered() {
            self.painter()
                .rect_filled(r.rect, 8, Color32::from_black_alpha(48));
        }

        r
    }

    fn d_spinner(&mut self, size: Vec2) -> Response {
        self.add(Image::new(spinner_image().clone()).fit_to_exact_size(size))
    }

    fn section_separator(&mut self, text: impl Into<RichText>) {
        self.add_space(6.0);
        self.add(egui::Label::new(text.into().weak().size(12.0)).selectable(false));
    }
}

pub fn spinner_image() -> &'static ImageSource<'static> {
    thread_local! {
        static START_TIME: RefCell<Instant> = RefCell::new(Instant::now());
    }

    let time = START_TIME
        .with(|start_time| start_time.borrow().elapsed())
        .as_secs_f32();

    const IMG0: ImageSource = include_image!("../../assets/ui/load0.png");
    const IMG1: ImageSource = include_image!("../../assets/ui/load1.png");
    const IMG2: ImageSource = include_image!("../../assets/ui/load2.png");
    const IMG3: ImageSource = include_image!("../../assets/ui/load3.png");
    const IMG4: ImageSource = include_image!("../../assets/ui/load4.png");

    &[IMG0, IMG0, IMG1, IMG2, IMG3, IMG4, IMG4, IMG3, IMG2, IMG1][(time * 5.0) as usize % 10]
}

/// Extension trait for adding widgets for external data types.
pub trait ExternalDataWidgetExt {
    fn show_input(&mut self, ui: &mut Ui) -> Response;
}
