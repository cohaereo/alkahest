use std::process::Command;

fn main() {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        let is_dirty = Command::new("git")
            .args(["diff", "--quiet"])
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

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();
    }

    // Generate UTC BUILD_DATE using chrono
    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=BUILD_DATE={build_date}");
    println!("cargo:rerun-if-changed=build.rs");
}
