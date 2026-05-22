/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use std::{ffi::OsStr, io::ErrorKind};

use cc::{self, windows_registry::VsVers};

fn get_msvc_env_paths(env_key: &str) -> io::Result<Vec<PathBuf>> {
    let target_env = match env::var("TARGET") {
        Ok(target) => target,
        Err(_) => {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                "Can't find TARGET cargo environment variable",
            ))
        }
    };

    let cl_tool = match cc::windows_registry::find_tool(&target_env, "cl.exe") {
        Some(tool) => tool,
        None => {
            return Err(io::Error::new(
                ErrorKind::NotFound,
                format!(
                    "Can't find cl.exe tool given by target triple {}",
                    target_env
                ),
            ))
        }
    };

    for env_entry in cl_tool.env().into_iter() {
        if env_entry.0 == env_key {
            return Ok(env_entry
                .1
                .to_str()
                .unwrap()
                .split(";")
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(OsStr::new(s)))
                    }
                })
                .collect());
        }
    }

    Err(io::Error::new(
        ErrorKind::NotFound,
        format!(
            "Can't find cl.exe tool's {} env variable - check your MSVC install",
            env_key
        ),
    ))
}

/// Updates the default stream manager cc build specs for Windows target
fn stream_cc_platform_specifics(build: &mut cc::Build, wwise_sdk: &PathBuf) -> io::Result<()> {
    let msvc_include_paths = get_msvc_env_paths("INCLUDE")?;
    let msvc_libs = get_msvc_env_paths("LIB")?;

    for lib in msvc_libs {
        println!(
            "cargo:rustc-link-search={}",
            lib.into_os_string().into_string().unwrap()
        );
    }

    build
        .file(wwise_sdk.join(r"samples\SoundEngine\Win32\AkDefaultIOHookDeferred.cpp"))
        .flag("-nologo")
        .flag("-MD")
        .flag("-MP")
        .define("WIN64", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .includes(msvc_include_paths)
        .include(wwise_sdk.join(r"samples\SoundEngine\Win32"));

    Ok(())
}

/// Updates build environment with required dependencies for Windows target
fn platform_dependencies(wwise_sdk: &PathBuf, config_folder: &str) {
    let vs_version = cc::windows_registry::find_vs_version().expect("No MSVC install found");

    let wwise_vc = match vs_version {
        VsVers::Vs14 => "x64_vc140",
        VsVers::Vs15 => "x64_vc150",
        VsVers::Vs16 => "x64_vc160",
        VsVers::Vs17 => "x64_vc170",
        _ => panic!("Unsupported MSVC version: {:?}", vs_version),
    };
    let path = wwise_sdk.join(wwise_vc);

    if !path.exists() {
        panic!(
            "Could not find {}.\nYou are using MSVC {:?} but the {} Wwise SDK target probably \
             wasn't installed or doesn't exist for this version of Wwise.\nNote that Vs17 (Visual \
             Studio 2022) is supported since Wwise 2021.1.10 only.",
            path.to_str().unwrap(),
            vs_version,
            wwise_vc
        )
    }

    println!(
        "cargo:rustc-link-search={}",
        path.join(config_folder)
            .join("lib")
            .into_os_string()
            .into_string()
            .unwrap()
    );
    println!(
        "cargo:rustc-link-search={}",
        path.join(config_folder)
            .join("bin")
            .into_os_string()
            .into_string()
            .unwrap()
    ); // For effect dlls

    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dsound");
    println!("cargo:rustc-link-lib=dylib=dxguid");
    println!("cargo:rustc-link-lib=dylib=XInput");
    println!("cargo:rustc-link-lib=dylib=user32");

    #[cfg(not(wwrelease))]
    {
        println!("cargo:rustc-link-lib=dylib=AkAutobahn"); // for WAAPI support in game editors
        println!("cargo:rustc-link-lib=dylib=ws2_32"); // for profiling networking
    }
}
