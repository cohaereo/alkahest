pub const BUILD_DATE: &str = env!("BUILD_DATE");
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CHANGELOG_MD: &str = include_str!("../../CHANGELOG.md");

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

pub fn print_banner() {
    println!("{}", BANNER);
    println!(
        "                     v{} ({} built on {})",
        VERSION, GIT_HASH, BUILD_DATE
    );
    println!();
    println!("{}", QUOTE);
}
