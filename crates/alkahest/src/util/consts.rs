use build_time::build_time_utc;
use glam::Vec4;

pub const BUILD_DATE: &str = build_time_utc!("%Y-%m-%d");
pub const BUILD_TIMESTAMP: &str = build_time_utc!();
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CHANGELOG_MD: &str = include_str!("../../../../CHANGELOG.md");

pub const BANNER: &str = r#"
    :::     :::        :::    :::     :::     :::    ::: :::::::::: :::::::: ::::::::::: 
  :+: :+:   :+:        :+:   :+:    :+: :+:   :+:    :+: :+:       :+:    :+:    :+:     
 +:+   +:+  +:+        +:+  +:+    +:+   +:+  +:+    +:+ +:+       +:+           +:+     
+#++:++#++: +#+        +#++:++    +#++:++#++: +#++:++#++ +#++:++#  +#++:++#++    +#+     
+#+     +#+ +#+        +#+  +#+   +#+     +#+ +#+    +#+ +#+              +#+    +#+     
#+#     #+# #+#        #+#   #+#  #+#     #+# #+#    #+# #+#       #+#    #+#    #+#     
###     ### ########## ###    ### ###     ### ###    ### ########## ########     ###     
"#;

pub const QUOTE: &str = r#"
    "Made possible by Clarity Control.
     Magnificent, wasn't it? An entity from beyond our own dimension. 
     And the answer to humanity's eternal struggle: mortality"
        - Clovis Bray
"#;

fn text_gradient(a: Vec4, b: Vec4, text: &str) -> String {
    let mut result = String::new();

    result.push_str("\x1b[1m");
    result.push_str("\x1b[52m");

    let longest_line = text.lines().map(|l| l.len()).max().unwrap_or(0);

    for l in text.lines() {
        let mut gradient = String::new();
        for (i, c) in l.chars().enumerate() {
            let t = i as f32 / longest_line as f32;
            let color = a.lerp(b, t);

            // Background
            if c.is_whitespace() {
                gradient.push_str("\x1b[49m");
            } else {
                gradient.push_str(&format!(
                    "\x1b[48;2;{};{};{}m",
                    ((color.x * 0.1) * 255.) as u8,
                    ((color.y * 0.1) * 255.) as u8,
                    ((color.z * 0.1) * 255.) as u8,
                ));
            }

            // Foreground + character
            gradient.push_str(&format!(
                "\x1b[38;2;{};{};{}m{}",
                (color.x * 255.) as u8,
                (color.y * 255.) as u8,
                (color.z * 255.) as u8,
                c
            ));
        }
        result.push_str(&format!("{}\n", gradient));
    }

    result.push_str("\x1b[0m");

    result
}

pub fn print_banner() {
    println!(
        "{}",
        text_gradient(
            Vec4::new(0.0, 1.0, 0.66, 1.0),
            Vec4::new(0.88, 0.0, 0.88, 1.0),
            BANNER
        )
    );
    println!(
        "                     \x1b[4mv{} ({} built on {})\x1b[0m",
        VERSION, GIT_HASH, BUILD_DATE
    );
    println!();
    println!("{}", QUOTE);
}
