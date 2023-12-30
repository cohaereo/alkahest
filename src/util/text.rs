use egui::Color32;

use super::fnv1;

pub fn text_color_for_background(background: Color32) -> Color32 {
    let r = background.r() as f32 / 255.;
    let g = background.g() as f32 / 255.;
    let b = background.b() as f32 / 255.;
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    if luma > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

pub fn name_to_color(name: &str) -> Color32 {
    let hash = fnv1(name.as_bytes());
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}

/// Simplifies meters to other metric measurement units (mm, cm, m, km)
pub fn prettify_distance(meters: f32) -> String {
    if meters < 0.001 {
        format!("{:.2} mm", meters * 1000.0)
    } else if meters < 1.0 {
        format!("{:.2} cm", meters * 100.0)
    } else if meters < 1000.0 {
        format!("{:.2} m", meters)
    } else {
        format!("{:.2} km", meters / 1000.0)
    }
}

pub fn split_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut last_upper = false;
    for c in s.chars() {
        if c.is_uppercase() {
            if !last_upper {
                result.push(' ');
            }
            last_upper = true;
        } else {
            last_upper = false;
        }
        result.push(c);
    }
    result
}
