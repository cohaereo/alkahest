pub trait StringExt {
    fn truncate_ellipsis(&self, max_len: usize) -> String;

    fn split_pascalcase(&self) -> String;
}

impl StringExt for String {
    fn truncate_ellipsis(&self, max_len: usize) -> String {
        if self.len() > max_len {
            format!("{}...", &self[..max_len - 3])
        } else {
            self.clone()
        }
    }

    fn split_pascalcase(&self) -> String {
        let mut result = String::new();
        let mut last_upper = false;
        for c in self.chars() {
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
}

/// Simplifies meters to other metric measurement units (mm, cm, m, km)
pub fn prettify_distance(meters: f32) -> String {
    if meters < 0.001 {
        format!("{:.2} mm", meters * 1000.0)
    } else if meters < 1.0 {
        format!("{:.2} cm", meters * 100.0)
    } else if meters < 1000.0 {
        format!("{:.2} m", meters)
    } else if meters.is_finite() {
        format!("{:.2} km", meters / 1000.0)
    } else {
        format!("{:.2}", meters)
    }
}
