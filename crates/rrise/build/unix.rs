/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

/// Updates the default stream manager cc build specs for Unix targets
fn stream_cc_platform_specifics(build: &mut cc::Build, wwise_sdk: &PathBuf) -> io::Result<()> {
    build
        .compiler("clang")
        .shared_flag(true)
        .opt_level(2)
        .flag("-MMD")
        .flag("-MP")
        .flag("-fPIC")
        .flag("-g")
        .flag("-Wno-invalid-offsetof")
        .flag("-fno-exceptions")
        .flag("-fno-rtti")
        .define("AUDIOKINETIC", None)
        .file(
            wwise_sdk
                .join("samples")
                .join("SoundEngine")
                .join("POSIX")
                .join("AkDefaultIOHookDeferred.cpp"),
        )
        .include(wwise_sdk.join("samples").join("SoundEngine").join("POSIX"));

    Ok(())
}
