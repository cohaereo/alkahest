fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    embed_resource::compile("assets/res.rc", embed_resource::NONE)
        .manifest_required()
        .expect("Failed to compile resource file");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Include lib folder in the search path
        println!("cargo:rustc-link-search=lib");

        let target_dir = if cfg!(target_os = "windows") {
            std::path::Path::new("target").to_path_buf()
        } else {
            std::path::Path::new("target").join(std::env::var("TARGET").unwrap())
        };

        // Copy lib/SDL3.dll to OUT_DIR
        let out_dir = target_dir.join(std::env::var("PROFILE").expect("PROFILE not set"));
        let sdl3_dll = std::path::Path::new("lib/SDL3.dll");
        std::fs::copy(sdl3_dll, out_dir.join("SDL3.dll")).expect("Failed to copy SDL3.dll");
    }
}
