use std::{env, path::PathBuf};

fn main() {
    let mut cfg = cmake::Config::new("./umbra-source");

    let profile = match &std::env::var("PROFILE").unwrap()[..] {
        "debug" => "Debug",
        "release" | "bench" => "Release",
        unknown => {
            eprintln!(
                "Warning: unknown Rust profile={}; defaulting to a release build.",
                unknown
            );
            "Release"
        }
    };

    let dst = cfg.profile(profile).build_target("umbra_runtime").build();

    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build/").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").join(profile).display()
    );
    println!("cargo:rustc-link-lib=umbra_common");
    println!("cargo:rustc-link-lib=umbra_runtime");

    let bindings = bindgen::Builder::default()
        .headers([
            "umbra-source/interface/runtime/umbraTome.hpp",
            "umbra-source/interface/runtime/umbraQuery.hpp",
        ])
        .clang_arg("-Iumbra-source/interface")
        // .default_enum_style(bindgen::EnumVariation::Consts)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed=umbra-source/CMakeLists.txt");
}
