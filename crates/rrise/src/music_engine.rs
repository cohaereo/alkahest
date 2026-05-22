/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::bindings::root::AK::MusicEngine::*;
use crate::settings::AkMusicSettings;
use crate::{ak_call_result, AkResult};

/// Initialize the music engine.
///
/// *Warning* This function must be called after the base sound engine has been properly initialized.
///
/// There should be no AK API call between [sound_engine::init](crate::sound_engine::init) and this call.
/// Any call done in between is potentially unsafe.
///
/// *Return* [AkResult::AK_Success] if the Init was successful, [AkResult::AK_Fail] otherwise.
pub fn init(init_settings: &mut AkMusicSettings) -> Result<(), AkResult> {
    ak_call_result![Init(init_settings)]?;
    Ok(())
}

/// Terminate the music engine.
///
/// *Warning* This function must be called before calling Term() on the base sound engine.
pub fn term() {
    unsafe {
        Term();
    }
}
