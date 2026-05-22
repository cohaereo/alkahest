/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::bindings::root::{InitDefaultStreamMgr, TermDefaultStreamMgr, AK};
use crate::settings::{AkDeviceSettings, AkStreamMgrSettings};
use crate::{ak_call_result, to_os_char, AkResult};

/// Stream Manager factory.
///
/// *Remarks*
/// > - In order for the Stream Manager to work properly, you also need to create
/// at least one streaming device (and implement its I/O hook), and register the
/// File Location Resolver with AK::StreamMgr::SetFileLocationResolver().
/// > - Use [AkStreamMgrSettings::default], then modify the settings you want,
/// then feed this function with them.
///
/// *See also*
/// - [AkStreamMgrSettings::default]
pub fn init(settings: &AkStreamMgrSettings) -> Result<(), AkResult> {
    let addr = unsafe { AK::StreamMgr::Create(settings) };
    if addr == std::ptr::null_mut() {
        Err(AkResult::AK_Fail)
    } else {
        Ok(())
    }
}

/// Initializes the default streaming manager, specifying the folder in which to find the generated soundbanks when they are loaded.
pub fn init_default_stream_mgr<T: AsRef<str>>(
    stream_mgr_settings: &AkStreamMgrSettings,
    device_settings: &mut AkDeviceSettings,
    bank_location: T,
) -> Result<(), AkResult> {
    init(stream_mgr_settings)?;
    device_settings.use_stream_cache = true;

    let device_settings = device_settings.as_ak();
    let pin_bytes = to_os_char(&bank_location);
    ak_call_result![InitDefaultStreamMgr(&device_settings, pin_bytes.as_ptr())]
}

/// Terminates the default streaming manager.
pub fn term_default_stream_mgr() {
    unsafe {
        TermDefaultStreamMgr();
    }
}

/// Set the current language once and only once, here. The language name is stored in a static buffer
/// inside the Stream Manager. In order to resolve localized (language-specific) file location, the
/// stream manger will query this string. It may use it to
/// construct a file path (for e.g. SDK/samples/SoundEngine/Common/AkFileLocationBase.cpp), or to
/// find a language-specific file within a look-up table (for e.g. SDK/samples/SoundEngine/Common/AkFilePackageLUT.cpp).
///
/// Pass a string, without a trailing slash or backslash. Empty strings are accepted.
///
/// You may register for language changes (see [register_to_language_change_notification]).
///
/// After changing the current language, all observers are notified.
///
/// *Return* [AK_Success](AkResult::AK_Success) if successful (if language string has less than
/// AK_MAX_LANGUAGE_NAME_SIZE characters). [AK_Fail](AkResult::AK_Fail) otherwise.
///
/// *Warning* Not multithread safe.
///
/// *See also*
/// - [current_language]
/// - [add_language_change_observer]
pub fn set_current_language<T: AsRef<str>>(language_name: T) -> Result<(), AkResult> {
    let pin_bytes = to_os_char(&language_name);
    ak_call_result![AK::StreamMgr::SetCurrentLanguage(pin_bytes.as_ptr())]
}
