use std::process::Command;

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

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        let is_dirty = Command::new("git")
            .args([
                "diff",
                "--ignore-matching-lines='^version = \".*\"'",
                "--quiet",
            ])
            .status()
            .unwrap()
            .code()
            .unwrap_or_default()
            != 0;

        let dirty = if is_dirty { "-dirty" } else { "" };
        let git_hash = String::from_utf8(output.stdout).unwrap();
        println!(
            "cargo:rustc-env=GIT_HASH={}{dirty}",
            git_hash.strip_suffix('\n').unwrap_or(&git_hash)
        );
    } else {
        println!("cargo:rustc-env=GIT_HASH=unknown-revision");
    }
}
