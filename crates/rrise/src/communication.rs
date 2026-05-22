/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

//! Handles profiling communications.
//!
//! Will be disabled entirely (not linked nor built) when `--cfg wwrelease` is set.

use crate::bindings::root::AK;
use crate::settings::AkCommSettings;
use crate::{ak_call_result, AkResult};

/// Initializes the communication module. When this is called, and [sound_engine::render_audio](crate::sound_engine::render_audio)
/// is called periodically, you may use the authoring tool to connect to the sound engine.
///
/// *Warning* This function must be called after the sound engine and memory manager have
/// been properly initialized.
///
/// *Remark* The [AkCommSettings] structure should be initialized with [AkCommSettings::default].
/// You can then change some of the parameters before calling this function.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) if initialization was successful.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if one of the settings is invalid.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory) if the specified pool size is too small for initialization.
/// > - [AK_Fail](AkResult::AK_Fail) for other errors.
pub fn init(settings: &AkCommSettings) -> Result<(), AkResult> {
    ak_call_result![AK::Comm::Init(settings)]
}

/// Terminates the communication module.
///
/// *Warning* This function must be called before the memory manager is terminated.
pub fn term() {
    unsafe {
        AK::Comm::Term();
    }
}
