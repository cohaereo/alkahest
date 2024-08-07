use alkahest_renderer::util::color::Color;
use egui::Color32;

use super::fnv1;

pub fn name_to_color(name: &str) -> Color32 {
    let hash = fnv1(name.as_bytes());
    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    Color32::from_rgb(r, g, b)
}

pub fn alk_color_to_egui(c: Color) -> Color32 {
    Color32::from_rgb(
        (c.r() * 255.0) as u8,
        (c.g() * 255.0) as u8,
        (c.b() * 255.0) as u8,
    )
}

// pub fn split_pascal_case(s: &str) -> String {
//     let mut result = String::new();
//     let mut last_upper = false;
//     for c in s.chars() {
//         if c.is_uppercase() {
//             if !last_upper {
//                 result.push(' ');
//             }
//             last_upper = true;
//         } else {
//             last_upper = false;
//         }
//         result.push(c);
//     }
//     result
// }
