/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

include!("unix.rs");

/// Updates build environment with required dependencies for Linux targets
fn platform_dependencies(wwise_sdk: &PathBuf, config_folder: &str) {
    println!(
        "cargo:rustc-link-search={}",
        wwise_sdk
            .join("Linux_x64")
            .join(config_folder)
            .join("lib")
            .into_os_string()
            .into_string()
            .unwrap()
    );
    println!(
        "cargo:rustc-link-search={}",
        wwise_sdk
            .join("Linux_x64")
            .join(config_folder)
            .join("bin")
            .into_os_string()
            .into_string()
            .unwrap()
    ); // For effect so's
}
