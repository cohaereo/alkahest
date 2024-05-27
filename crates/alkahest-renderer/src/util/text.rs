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
