use std::{cell::RefCell, time::Instant};

use egui::*;

pub trait UiExt {
    #[must_use]
    fn d_button(&mut self, text: impl Into<RichText>) -> Response;

    // fn d_spinner(&mut self, size: Vec2) -> Response;
    fn d_paint_spinner_at(&mut self, rect: Rect);

    fn section_separator(&mut self, text: impl Into<RichText>);
}

impl UiExt for Ui {
    fn d_button(&mut self, text: impl Into<RichText>) -> Response {
        DButton::new(text.into()).ui(self)
    }

    // fn d_spinner(&mut self, size: Vec2) -> Response {
    //     let (rect, response) = self.allocate_exact_size(size, Sense::hover());
    //     // self.add(Image::new(spinner_image().clone()).fit_to_exact_size(size))

    //     self.d_paint_spinner_at(rect);

    //     response
    // }

    fn d_paint_spinner_at(&mut self, rect: Rect) {
        let (img1, img2, t) = spinner_image();
        let alpha1 = 1.0 - t;
        let alpha2 = t;
        Image::new(img1.clone())
            .tint(Color32::from_white_alpha((alpha1 * 255.0) as u8))
            .paint_at(self, rect);
        Image::new(img2.clone())
            .tint(Color32::from_white_alpha((alpha2 * 255.0) as u8))
            .paint_at(self, rect);
    }

    fn section_separator(&mut self, text: impl Into<RichText>) {
        self.add_space(6.0);
        self.add(egui::Label::new(text.into().weak().size(12.0)).selectable(false));
    }
}

pub fn spinner_image() -> (
    &'static ImageSource<'static>,
    &'static ImageSource<'static>,
    f32,
) {
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
    const IMG5: ImageSource = include_image!("../../assets/ui/load5.png");

    const IMAGES: &[ImageSource] = &[IMG0, IMG1, IMG2, IMG3, IMG4, IMG5];

    const SPEED: f32 = 4.0;
    let img1 = &IMAGES[(time * SPEED) as usize % IMAGES.len()];
    let img2 = &IMAGES[(time * SPEED + 1.0) as usize % IMAGES.len()];
    let t = (time * SPEED) % 1.0;

    (img1, img2, t)
}

/// Extension trait for adding widgets for external data types.
pub trait ExternalDataWidgetExt {
    fn show_input(&mut self, ui: &mut Ui) -> Response;
}

pub struct DButton<'a> {
    button: egui::Button<'a>,
    text_color: Color32,
    stroke: Stroke,
    fill_color: Color32,
}

impl<'a> DButton<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            button: egui::Button::new(atoms)
                .min_size(vec2(120.0, 60.0))
                .corner_radius(0),
            text_color: Color32::WHITE,
            stroke: Stroke::new(1.0, Color32::WHITE),
            fill_color: Color32::from_gray(96).gamma_multiply(0.2),
        }
    }

    pub fn new_white(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            button: egui::Button::new(atoms)
                .min_size(vec2(120.0, 60.0))
                .corner_radius(0),
            text_color: Color32::BLACK,
            stroke: Stroke::new(1.0, Color32::WHITE),
            fill_color: Color32::from_white_alpha(196),
        }
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(25.0, 20.0);
            ui.style_mut().visuals.override_text_color = Some(self.text_color);

            let r = ui
                .add(self.button.stroke(self.stroke).fill(self.fill_color))
                .on_hover_cursor(CursorIcon::PointingHand);

            if r.hovered() {
                ui.painter().rect(
                    r.rect.expand(4.0),
                    0,
                    Color32::TRANSPARENT,
                    Stroke::new(2.0, Color32::from_white_alpha(196)),
                    StrokeKind::Outside,
                );
            }

            r
        })
        .inner
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.button = self.button.min_size(size);
        self
    }

    pub fn stroke(mut self, width: f32, color: Color32) -> Self {
        self.stroke = Stroke::new(width, color);
        self
    }

    pub fn fill(mut self, color: Color32) -> Self {
        self.fill_color = color;
        self
    }
}
