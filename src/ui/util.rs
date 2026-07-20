use std::{cell::RefCell, time::Instant};

use egui::*;

pub trait UiExt {
    #[must_use]
    fn d_button(&mut self, text: impl Into<RichText>) -> Response;

    // fn d_spinner(&mut self, size: Vec2) -> Response;
    fn d_paint_spinner_at(&mut self, rect: Rect);

    fn section_separator(&mut self, text: impl Into<RichText>);

    fn image_link<'a>(
        &mut self,
        image_source: impl Into<ImageSource<'a>>,
        size: Vec2,
        link: &str,
    ) -> Response;
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

    fn image_link<'a>(
        &mut self,
        image_source: impl Into<ImageSource<'a>>,
        size: Vec2,
        link: &str,
    ) -> Response {
        let response = self
            .allocate_response(size, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        egui::Image::new(image_source)
            .tint(if response.hovered() {
                egui::Color32::LIGHT_GRAY
            } else {
                egui::Color32::DARK_GRAY
            })
            .paint_at(self, response.rect);

        if response.clicked() {
            self.ctx().open_url(egui::OpenUrl::new_tab(link));
        }

        response
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
    text: String,
    subtitle: Option<String>,
    text_color: Color32,
    stroke: Stroke,
    fill_color: Color32,
    padding: Vec2,
}

impl<'a> DButton<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        let atoms = atoms.into_atoms();
        Self {
            text: atoms.text().unwrap_or_default().to_string(),
            button: egui::Button::new(atoms)
                .min_size(vec2(100.0, 60.0))
                .corner_radius(0),
            subtitle: None,
            text_color: Color32::WHITE,
            stroke: Stroke::new(1f32, Color32::WHITE),
            fill_color: Color32::from_gray(96).gamma_multiply(0.2),
            padding: vec2(25.0, 20.0),
        }
    }

    pub fn new_white(atoms: impl IntoAtoms<'a>) -> Self {
        let atoms = atoms.into_atoms();
        Self {
            text: atoms.text().unwrap_or_default().to_string(),
            button: egui::Button::new(atoms)
                .min_size(vec2(120.0, 60.0))
                .corner_radius(0),
            subtitle: None,
            text_color: Color32::BLACK,
            stroke: Stroke::new(1f32, Color32::WHITE),
            fill_color: Color32::from_white_alpha(196),
            padding: vec2(25.0, 20.0),
        }
    }

    pub fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = self.padding;
            let text_color = if self.subtitle.is_some() {
                // Hide the text as we'll be doing the layout ourselves. This will still reserve the space necessary for the main button text.
                Color32::TRANSPARENT
            } else {
                self.text_color
            };
            ui.style_mut().visuals.override_text_color = Some(text_color);

            let r = ui
                .add(self.button.stroke(self.stroke).fill(self.fill_color))
                .on_hover_cursor(CursorIcon::PointingHand);

            if let Some(subtitle) = self.subtitle {
                let title_pos = r.rect.left_center() + vec2(64.0, 4.0);
                let text_style = ui.style().text_styles[&TextStyle::Button].clone();
                ui.painter().text(
                    title_pos + vec2(0.0, 1.0),
                    Align2::LEFT_BOTTOM,
                    &self.text,
                    text_style.clone(),
                    self.text_color,
                );
                ui.painter().text(
                    title_pos + vec2(6.0, 0.0),
                    Align2::LEFT_TOP,
                    &subtitle,
                    FontId::proportional(text_style.size / 1.5),
                    Color32::GRAY,
                );
            }

            if r.hovered() {
                ui.painter().rect(
                    r.rect.expand(4.0),
                    0,
                    Color32::TRANSPARENT,
                    Stroke::new(2f32, Color32::from_white_alpha(196)),
                    StrokeKind::Outside,
                );
            }

            r
        })
        .inner
    }

    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.button = self.button.min_size(size);
        self
    }

    pub fn padding(mut self, padding: Vec2) -> Self {
        self.padding = padding;
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
