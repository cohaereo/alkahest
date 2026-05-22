/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use rrise::settings::*;
use rrise::{communication, memory_mgr, sound_engine, stream_mgr, AkResult};

pub fn init_sound_engine() -> Result<(), AkResult> {
    // init memorymgr
    memory_mgr::init(&mut AkMemSettings::default())?;
    assert!(memory_mgr::is_initialized());

    // init streamingmgr
    #[cfg(target_os = "windows")]
    let platform = "Windows";
    #[cfg(target_os = "linux")]
    let platform = "Linux";
    stream_mgr::init_default_stream_mgr(
        &AkStreamMgrSettings::default(),
        &mut AkDeviceSettings::default(),
        format!("examples/WwiseProject/GeneratedSoundBanks/{}", platform),
    )?;
    stream_mgr::set_current_language("English(US)")?;

    // init soundengine
    sound_engine::init(
        &mut AkInitSettings::default(),
        &mut AkPlatformInitSettings::default(),
    )?;

    // no need for music engine

    // no need for spatial

    // init comms
    #[cfg(not(wwrelease))]
    communication::init(&AkCommSettings::default())?;

    assert!(sound_engine::is_initialized());
    Ok(())
}

pub fn term_sound_engine() -> Result<(), AkResult> {
    // term comms
    #[cfg(not(wwrelease))]
    communication::term();

    // term spatial

    // term music

    // term soundengine
    sound_engine::term();

    // term streamingmgr
    stream_mgr::term_default_stream_mgr();

    // term memorymgr
    memory_mgr::term();

    Ok(())
}

pub fn one_frame_render() -> Result<(), AkResult> {
    init_sound_engine()?;
    sound_engine::render_audio(false)?;
    term_sound_engine()?;
    Ok(())
}
