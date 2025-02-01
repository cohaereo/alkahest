use std::{path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

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

    if let Ok(v) = rustc_version::version_meta() {
        println!("cargo:rustc-env=RUSTC_VERSION={}", v.semver);
    }

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon2.ico");
        res.compile().unwrap();
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    println!(
        "cargo:rustc-link-search=native={}",
        manifest_dir.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=iron");
}
