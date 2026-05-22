/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use std::env;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

macro_rules! static_feature {
    ($feature:literal) => {
        #[cfg(feature = $feature)]
        println!("cargo:rustc-link-lib=static:-whole-archive={}", $feature);
    };
}

macro_rules! static_feature_h {
    ($file:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        writeln!(&mut $file, "#include <AK/Plugin/{}Factory.h>", $feature)?;
    };
}

fn main() -> io::Result<()> {
    let wwise_sdk = PathBuf::from(env::var("WWISESDK").expect("env variable WWISESDK not found"));

    // --- RERUN CONFIG
    println!("cargo:rerun-if-changed=c/ak.h");
    println!("cargo:rerun-if-changed=c/utilities/default_streaming_mgr.h");
    println!("cargo:rerun-if-changed=c/utilities/default_streaming_mgr.cpp");
    println!("cargo:rerun-if-env-changed=WWISESDK");
    println!("cargo:rerun-if-env-changed=RRISE_RERUN_BUILD");
    // --- END RERUN CONFIG

    // --- SETUP BUILD ENV
    println!("cargo:rustc-link-lib=static=AkSoundEngine");
    println!("cargo:rustc-link-lib=static=AkMusicEngine");
    println!("cargo:rustc-link-lib=static=AkSpatialAudio");
    println!("cargo:rustc-link-lib=static=AkMemoryMgr");
    println!("cargo:rustc-link-lib=static=AkStreamMgr");
    static_feature!("AkVorbisDecoder");
    static_feature!("AkOpusDecoder");
    static_feature!("AkMeterFX");
    static_feature!("AkAudioInputSource");
    static_feature!("AkCompressorFX");
    static_feature!("AkDelayFX");
    static_feature!("AkExpanderFX");
    static_feature!("AkFlangerFX");
    static_feature!("AkGainFX");
    static_feature!("AkGuitarDistortionFX");
    static_feature!("AkHarmonizerFX");
    static_feature!("AkMatrixReverbFX");
    static_feature!("AkParametricEQFX");
    static_feature!("AkPeakLimiterFX");
    static_feature!("AkPitchShifterFX");
    static_feature!("AkRecorderFX");
    static_feature!("AkRoomVerbFX");
    static_feature!("AkSilenceSource");
    static_feature!("AkSineSource");
    static_feature!("AkStereoDelayFX");
    static_feature!("AkSynthOneSource");
    static_feature!("AkTimeStretchFX");
    static_feature!("AkToneSource");
    static_feature!("AkTremoloFX");

    let config_folder = if cfg!(wwdebug) {
        "Debug"
    } else if cfg!(wwrelease) {
        "Release"
    } else {
        "Profile"
    };
    println!("Selected Wwise config: {}", config_folder);
    platform_dependencies(&wwise_sdk, config_folder);

    #[cfg(not(wwrelease))]
    println!("cargo:rustc-link-lib=static=CommunicationCentral");
    // --- END SETUP BUILD ENV

    // --- PREP PLUGIN STATIC DEPS
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let static_deps_h_path = out_path.join("static_plugins.h");
    {
        let mut static_deps_h = File::create(static_deps_h_path.clone())?;

        writeln!(
            &mut static_deps_h,
            "#include <AK/SoundEngine/Common/IAkPlugin.h>"
        )?;

        static_feature_h!(static_deps_h, "AkVorbisDecoder");
        static_feature_h!(static_deps_h, "AkOpusDecoder");
        static_feature_h!(static_deps_h, "AkMeterFX");
        static_feature_h!(static_deps_h, "AkAudioInputSource");
        static_feature_h!(static_deps_h, "AkCompressorFX");
        static_feature_h!(static_deps_h, "AkDelayFX");
        static_feature_h!(static_deps_h, "AkExpanderFX");
        static_feature_h!(static_deps_h, "AkFlangerFX");
        static_feature_h!(static_deps_h, "AkGainFX");
        static_feature_h!(static_deps_h, "AkGuitarDistortionFX");
        static_feature_h!(static_deps_h, "AkHarmonizerFX");
        static_feature_h!(static_deps_h, "AkMatrixReverbFX");
        static_feature_h!(static_deps_h, "AkParametricEQFX");
        static_feature_h!(static_deps_h, "AkPeakLimiterFX");
        static_feature_h!(static_deps_h, "AkPitchShifterFX");
        static_feature_h!(static_deps_h, "AkRecorderFX");
        static_feature_h!(static_deps_h, "AkRoomVerbFX");
        static_feature_h!(static_deps_h, "AkSilenceSource");
        static_feature_h!(static_deps_h, "AkSineSource");
        static_feature_h!(static_deps_h, "AkStereoDelayFX");
        static_feature_h!(static_deps_h, "AkSynthOneSource");
        static_feature_h!(static_deps_h, "AkTimeStretchFX");
        static_feature_h!(static_deps_h, "AkToneSource");
        static_feature_h!(static_deps_h, "AkTremoloFX");

        static_deps_h.sync_all()?;
    }
    // --- END PREP PLUGIN STATIC DEPS

    // --- BUILD UTILITIES
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("c")
        .join("utilities");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(crate_dir.join("default_streaming_mgr.cpp"))
        .file(crate_dir.join("static_plugins.cpp"))
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("Common")
                .join("AkFilePackage.cpp"),
        )
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("Common")
                .join("AkFilePackageLUT.cpp"),
        )
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("Common")
                .join("AkMultipleFileLocation.cpp"),
        )
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("Common")
                .join("AkFileLocationBase.cpp"),
        )
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("Common")
                .join("AkDefaultLowLevelIODispatcher.cpp"),
        )
        .include(out_path.clone())
        .include(wwise_sdk.join("include"))
        .include(wwise_sdk.join("samples").join("SoundEngine"))
        .define("UNICODE", None)
        .no_default_flags(true)
        .flag_if_supported("-Brepro")
        .flag_if_supported("-fpermissive")
        .warnings(false);

    stream_cc_platform_specifics(&mut build, &wwise_sdk)?;

    #[cfg(wwdebug)]
    build.flag_if_supported("-Z7").define("_DEBUG", None);

    #[cfg(not(debug_assertions))]
    build.define("NDEBUG", None);

    #[cfg(wwrelease)]
    build.define("AK_OPTIMIZED", None);

    build.compile("rrise_utilities");
    // --- END BUILD UTILITIES

    // --- RUN BINDGEN
    let bindings = bindgen::Builder::default()
        .header("c/ak.h")
        .header("c/utilities/default_streaming_mgr.h")
        .clang_arg(format!(
            "-I{}",
            wwise_sdk
                .join("include")
                .into_os_string()
                .into_string()
                .unwrap()
        ))
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++14")
        .opaque_type("AkArray")
        .opaque_type("AkListBareLight")
        .opaque_type("AkHashList")
        .opaque_type("AkHashList_HashTableArray")
        .opaque_type("AkHashListBare")
        .opaque_type("AkHashListBare_HashTableArray")
        .opaque_type("AkDbString")
        .opaque_type("AkDbString_Instance")
        .allowlist_type("AK::.*")
        .allowlist_type("Ak.*")
        .allowlist_type("AK.*")
        .allowlist_var("AK::.*")
        .allowlist_var("AK.*")
        .allowlist_var("Ak.*")
        .allowlist_var("Ak.*")
        .allowlist_function("AK::.*")
        .allowlist_function("Ak.*")
        .allowlist_function("InitDefaultStreamMgr")
        .allowlist_function("TermDefaultStreamMgr")
        .blocklist_item("AK_INVALID_GAME_OBJECT")
        .blocklist_item("AK_INVALID_AUDIO_OBJECT_ID")
        .rustified_enum("AKRESULT")
        .rustified_enum("AkGroupType")
        .rustified_enum("AkConnectionType")
        .rustified_enum("AkCurveInterpolation")
        .rustified_enum("MultiPositionType")
        .rustified_enum("AkSpeakerPanningType")
        .rustified_enum("Ak3DPositionType")
        .rustified_enum("AkPanningRule")
        .rustified_enum("Ak3DSpatializationMode")
        .rustified_enum("AkPluginType")
        .rustified_enum("AkNodeType")
        .rustified_enum("AK::SoundEngine::Query::RTPCValue_type")
        .bitfield_enum("AkAudioDeviceState")
        .bitfield_enum("AkBusHierarchyFlags")
        .bitfield_enum("AkMeteringFlags")
        .bitfield_enum("AkCallbackType")
        .bitfield_enum("AkAudioAPILinux")
        .must_use_type("AKRESULT")
        .enable_cxx_namespaces()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .layout_tests(false)
        .generate()
        .expect("Unable to generate bindings");

    let contents = bindings.to_string().replace("doc = \"< ", "doc = \"");

    File::create(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!")
        .write_all(contents.as_bytes())?;
    // --- END RUN BINDGEN

    Ok(())
}

#[cfg(target_os = "windows")]
include!("build/windows.rs");

#[cfg(target_os = "linux")]
include!("build/linux.rs");

#[cfg(target_os = "macos")]
include!("build/macos.rs");
