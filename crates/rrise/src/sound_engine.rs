/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use ::std::{convert::TryInto, ffi::CStr, fmt::Debug, mem::zeroed};

use crate::{
    bindings::root::{AK::SoundEngine::*, *},
    settings::{AkInitSettings, AkPlatformInitSettings},
    *,
};

macro_rules! link_static_plugin {
    ($feature:ident) => {
        link_static_plugin![$feature, $feature]
    };
    ($feature:ident, $global_var_name:ident) => {
        paste::paste! {
            #[cfg(feature = "" $feature)]
            {
                // If max log level doesn't include debug, need to explicitly reference this variable
                // or it won't be statically linked and the plugin won't be able to be loaded.
                #[cfg(any(
                    all(
                        debug_assertions,
                        all(not(feature = "max_level_debug"), not(feature = "max_level_trace"))
                    ),
                    all(
                        not(debug_assertions),
                        all(
                            not(feature = "release_max_level_debug"),
                            not(feature = "release_max_level_trace")
                        )
                    ),
                ))]
                ::std::hint::black_box(unsafe {
                    crate::bindings_static_plugins::[<$global_var_name Registration>]
                });
                log::debug!(
                    "{} has been statically loaded successfully",
                    stringify!($feature)
                )
            }
        }
    };
}

/// Initialize the sound engine.
///
/// *Warning* This function is not thread-safe.
///
/// *Remark* The initial settings should be initialized using [AkInitSettings::default]
/// and [AkPlatformInitSettings::default] to fill the structures with their
/// default settings. This is not mandatory, but it helps avoid backward compatibility problems.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) if the initialization was successful
/// > - [AK_MemManagerNotInitialized](AkResult::AK_MemManagerNotInitialized) if the memory manager is not available or not properly initialized
/// > - [AK_StreamMgrNotInitialized](AkResult::AK_StreamMgrNotInitialized) if the stream manager is not available or not properly initialized
/// > - [AK_SSEInstructionsNotSupported](AkResult::AK_SSEInstructionsNotSupported) if the machine does not support SSE instruction (only on the PC)
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory) or [AK_Fail](AkResult::AK_Fail) if there is not enough memory available to initialize the sound engine properly
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if some parameters are invalid
/// > - [AK_Fail](AkResult::AK_Fail) if the sound engine is already initialized, or if the provided settings result in insufficient resources for the initialization.
///
/// *See also*
/// > - [term]
/// > - [AkInitSettings::default]
/// > - [AkPlatformInitSettings::default]
pub fn init(
    init_settings: &mut AkInitSettings,
    platform_init_settings: &mut AkPlatformInitSettings,
) -> Result<(), AkResult> {
    let mut init_settings = init_settings.as_ak();
    let mut platform_init_settings = platform_init_settings.as_ak();
    ak_call_result![Init(&mut init_settings, &mut platform_init_settings)]?;

    link_static_plugin![AkVorbisDecoder];
    link_static_plugin![AkOggOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
    link_static_plugin![AkWemOpusDecoder]; // see Ak/Plugin/AkOpusDecoderFactory.h
    link_static_plugin![AkMeterFX];
    link_static_plugin![AkAudioInputSource];
    link_static_plugin![AkCompressorFX];
    link_static_plugin![AkDelayFX];
    link_static_plugin![AkExpanderFX];
    link_static_plugin![AkFlangerFX];
    link_static_plugin![AkGainFX];
    link_static_plugin![AkGuitarDistortionFX];
    link_static_plugin![AkHarmonizerFX];
    link_static_plugin![AkMatrixReverbFX];
    link_static_plugin![AkParametricEQFX];
    link_static_plugin![AkPeakLimiterFX];
    link_static_plugin![AkPitchShifterFX];
    link_static_plugin![AkRecorderFX];
    link_static_plugin![AkRoomVerbFX];
    link_static_plugin![AkSilenceSource];
    link_static_plugin![AkSineSource, SineSource];
    link_static_plugin![AkStereoDelayFX];
    link_static_plugin![AkSynthOneSource, AkSynthOne];
    link_static_plugin![AkTimeStretchFX];
    link_static_plugin![AkToneSource];
    link_static_plugin![AkTremoloFX];

    Ok(())
}

/// Query whether or not the sound engine has been successfully initialized.
///
/// *Warning* This function is not thread-safe. It should not be called at the same time as [init()] or [term()].
///
/// *Return* `True` if the sound engine has been initialized, `False` otherwise.
///
/// *See also*
/// > - [init]
/// > - [term]
pub fn is_initialized() -> bool {
    unsafe { IsInitialized() }
}

/// Terminates the sound engine.
///
/// If some sounds are still playing or events are still being processed when this function is
/// called, they will be stopped.
///
/// *Warning* This function is not thread-safe.
///
/// *Warning* Before calling `Term`, you must ensure that no other thread is accessing the sound engine.
///
/// *See also*
/// > - [init]
pub fn term() {
    unsafe {
        Term();
    }
}

/// Processes all commands in the sound engine's command queue.
///
/// This method has to be called periodically (usually once per game frame).
///
/// `allow_sync_render`: When AkInitSettings::b_use_lengine_thread is false, `RenderAudio` may generate
/// an audio buffer -- unless in_bAllowSyncRender is set to false. Use in_bAllowSyncRender=false
/// when calling RenderAudio from a Sound Engine callback.
///
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
///
/// *See also*
/// > - [PostEvent](struct@PostEvent)
pub fn render_audio(allow_sync_render: bool) -> Result<(), AkResult> {
    ak_call_result![RenderAudio(allow_sync_render)]
}

/// Unregister all game objects, or all game objects with a particular matching set of property flags.
///
/// This function to can be used to unregister all game objects.
///
/// *Return* AK_Success if successful
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered. Unregistering a game object while it is
/// in use is allowed, but the control over the parameters of this game object is lost.
/// For example, if a sound associated with this game object is a 3D moving sound, it will
/// stop moving once the game object is unregistered, and there will be no way to recover
/// the control over this game object.
///
/// *See also*
/// - [register_game_obj]
/// - [unregister_game_obj]
pub fn unregister_all_game_obj() -> Result<(), AkResult> {
    ak_call_result![UnregisterAllGameObj()]
}

/// Unregisters a game object.
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered. Unregistering a game object while it is
/// in use is allowed, but the control over the parameters of this game object is lost.
/// For example, say a sound associated with this game object is a 3D moving sound. This sound will
/// stop moving when the game object is unregistered, and there will be no way to regain control
/// over the game object.
///
/// *See also*
/// > - [register_game_obj]
/// > - [unregister_all_game_obj]
pub fn unregister_game_obj(game_object_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![UnregisterGameObj(game_object_id)]
}

/// Registers a game object.
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered.
///
/// *See also*
/// > - [unregister_game_obj]
/// > - [unregister_all_game_obj]
pub fn register_game_obj(game_object_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![RegisterGameObj(game_object_id)]
}

/// Registers a game object.
///
/// The name is just for monitoring purpose, and is not forwarded to Wwise when the `wwrelease`
/// cfg flag is on (in that case, calling this has the same effect as calling [register_game_obj()]).
///
/// *Return*
/// > - AK_Success if successful
/// > - AK_Fail if the specified AkGameObjectID is invalid (0 is an invalid ID)
///
/// *Remark* Registering a game object twice does nothing. Unregistering it once unregisters it no
/// matter how many times it has been registered.
///
/// *See also*
/// > - [register_game_obj]
/// > - [unregister_game_obj]
/// > - [unregister_all_game_obj]
pub fn register_named_game_obj<T: AsRef<str>>(
    game_object_id: AkGameObjectID,
    #[cfg_attr(wwrelease, allow(unused_variables))] name: T,
) -> Result<(), AkResult> {
    #[cfg(wwrelease)]
    return register_game_obj(game_object_id);

    #[cfg(not(wwrelease))]
    return with_cstring![name.as_ref() => cname {
        ak_call_result![RegisterGameObj1(game_object_id, cname.as_ptr())]
    }];
}

/// Sets the position of a game object.
///
/// *Warning* `position`'s orientation vectors must be normalized.
///
/// *Return*
/// > - [AK_Success](AkResult::AK_Success) when successful
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter) if parameters are not valid.
pub fn set_position<T: Into<AkSoundPosition>>(
    game_object_id: AkGameObjectID,
    position: T,
    is_listener: bool,
) -> Result<(), AkResult> {
    ak_call_result![SetPosition(
        game_object_id,
        &position.into(),
        if is_listener {
            AkSetPositionFlags_AkSetPositionFlags_Listener
        } else {
            AkSetPositionFlags_AkSetPositionFlags_Emitter
        }
    )]
}

pub fn set_game_object_output_bus_volume(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
    volume: f32,
) -> Result<(), AkResult> {
    ak_call_result![SetGameObjectOutputBusVolume(
        emitter_id,
        listener_id,
        volume,
    )]
}

pub fn set_listener_spatialization(
    listener_id: AkGameObjectID,
    spatialized: bool,
) -> Result<(), AkResult> {
    ak_call_result![SetListenerSpatialization(
        listener_id,
        spatialized,
        zeroed(),
        ::std::ptr::null_mut()
    )]
}

/// Sets the default set of associated listeners for game objects that have not explicitly overridden their listener sets. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
///
/// All default listeners that have previously been added via AddDefaultListener or set via SetDefaultListeners will be removed and replaced with the listeners in the array in_pListenerGameObjs.
///
/// *Return* Always returns [AK_Success](AkResult::AK_Success)
pub fn set_default_listeners(listener_ids: &[AkGameObjectID]) -> Result<(), AkResult> {
    ak_call_result![SetDefaultListeners(
        listener_ids.as_ptr(),
        listener_ids.len().try_into().unwrap()
    )]
}

/// Add a single listener to the default set of listeners. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
pub fn add_default_listener(listener_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![AddDefaultListener(listener_id)]
}

/// Remove a single listener from the default set of listeners. Upon registration, all game objects reference the default listener set, until
/// a call to [add_listener], [remove_listener], [set_listeners] or [set_game_object_output_bus_volume] is made on that game object.
pub fn remove_default_listener(listener_id: AkGameObjectID) -> Result<(), AkResult> {
    ak_call_result![RemoveDefaultListener(listener_id)]
}

/// Sets a game object's associated listeners.
///
/// All listeners that have previously been added via [add_listener] or set via [set_listeners] will be removed and replaced with the listeners in the array `listener_ids`.
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [set_listeners()]
/// > - [remove_listener()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn set_listeners(
    emitter_id: AkGameObjectID,
    listener_ids: &[AkGameObjectID],
) -> Result<(), AkResult> {
    ak_call_result![SetListeners(
        emitter_id,
        listener_ids.as_ptr(),
        listener_ids.len().try_into().unwrap(),
    )]
}

/// Add a single listener to a game object's set of associated listeners.
///
/// Any listeners that have previously been added or set via [add_listener] or [set_listeners] will remain as listeners and `listener_id` will be added as an additional listener.
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [set_listeners()]
/// > - [remove_listener()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn add_listener(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
) -> Result<(), AkResult> {
    ak_call_result![AddListener(emitter_id, listener_id)]
}

/// Remove a single listener from a game object's set of active listeners.
///
/// Calling this function will override the default set of listeners and `emitter_id` will now reference its own, unique set of listeners.
///
/// *See also*
/// > - [add_listener()]
/// > - [set_listeners()]
/// > - [add_default_listener()]
/// > - [remove_default_listener()]
/// > - [set_default_listeners()]
pub fn remove_listener(
    emitter_id: AkGameObjectID,
    listener_id: AkGameObjectID,
) -> Result<(), AkResult> {
    ak_call_result![RemoveListener(emitter_id, listener_id)]
}

/// Stops the current content playing associated to the specified game object ID.
///
/// If no game object is specified, all sounds will be stopped.
pub fn stop_all(game_object_id: Option<AkGameObjectID>) {
    unsafe {
        StopAll(game_object_id.unwrap_or(AK_INVALID_GAME_OBJECT));
    }
}

/// Load a bank synchronously (by Unicode string).
///
/// The bank name is passed to the Stream Manager.
///
/// A bank load request will be posted, and consumed by the Bank Manager thread.
///
/// The function returns when the request has been completely processed.
///
/// *Return*
/// The bank ID (see [get_id_from_string]). You may use this ID with [unload_bank].
/// > - [AK_Success](AkResult::AK_Success): Load or unload successful.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory): Insufficient memory to store bank data.
/// > - [AK_BankReadError](AkResult::AK_BankReadError): I/O error.
/// > - [AK_WrongBankVersion](AkResult::AK_WrongBankVersion): Invalid bank version: make sure the version of Wwise that you used to generate the SoundBanks matches that of the SDK you are currently using.
/// > - [AK_InvalidFile](AkResult::AK_InvalidFile): File specified could not be opened.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter): Invalid parameter, invalid memory alignment.
/// > - [AK_Fail](AkResult::AK_Fail): Load or unload failed for any other reason. (Most likely small allocation failure)
///
/// *Remarks*
/// > - The initialization bank must be loaded first.
/// > - All SoundBanks subsequently loaded must come from the same Wwise project as the
///   initialization bank. If you need to load SoundBanks from a different project, you
///   must first unload ALL banks, including the initialization bank, then load the
///   initialization bank from the other project, and finally load banks from that project.
/// > - Codecs and plug-ins must be registered before loading banks that use them.
/// > - Loading a bank referencing an unregistered plug-in or codec will result in a load bank success,
/// but the plug-ins will not be used. More specifically, playing a sound that uses an unregistered effect plug-in
/// will result in audio playback without applying the said effect. If an unregistered source plug-in is used by an event's audio objects,
/// posting the event will fail.
/// > - The sound engine internally calls get_id_from_string(name) to return the correct bank ID.
/// Therefore, in_pszString should be the real name of the SoundBank (with or without the BNK extension - it is trimmed internally),
/// not the name of the file (if you changed it), nor the full path of the file. The path should be resolved in
/// your implementation of the Stream Manager, or in the Low-Level I/O module if you use the default Stream Manager's implementation.
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [unload_bank_by_id]
/// > - [clear_banks]
/// > - [get_id_from_string]
pub fn load_bank_by_name<T: AsRef<str>>(name: T) -> Result<AkBankID, AkResult> {
    let mut bank_id = 0;
    with_cstring![name.as_ref() => cname {
        ak_call_result![LoadBank1(cname.as_ptr(), &mut bank_id, AkBankTypeEnum_AkBankType_User) => bank_id]
    }]
}

/// Load a bank synchronously (by Unicode string).
///
/// The bank name is passed to the Stream Manager.
///
/// A bank load request will be posted, and consumed by the Bank Manager thread.
///
/// The function returns when the request has been completely processed.
///
/// *Return*
/// The bank ID (see [get_id_from_string]). You may use this ID with [unload_bank].
/// > - [AK_Success](AkResult::AK_Success): Load or unload successful.
/// > - [AK_InsufficientMemory](AkResult::AK_InsufficientMemory): Insufficient memory to store bank data.
/// > - [AK_BankReadError](AkResult::AK_BankReadError): I/O error.
/// > - [AK_WrongBankVersion](AkResult::AK_WrongBankVersion): Invalid bank version: make sure the version of Wwise that you used to generate the SoundBanks matches that of the SDK you are currently using.
/// > - [AK_InvalidFile](AkResult::AK_InvalidFile): File specified could not be opened.
/// > - [AK_InvalidParameter](AkResult::AK_InvalidParameter): Invalid parameter, invalid memory alignment.
/// > - [AK_Fail](AkResult::AK_Fail): Load or unload failed for any other reason. (Most likely small allocation failure)
///
/// *Remarks*
/// > - The initialization bank must be loaded first.
/// > - All SoundBanks subsequently loaded must come from the same Wwise project as the
///   initialization bank. If you need to load SoundBanks from a different project, you
///   must first unload ALL banks, including the initialization bank, then load the
///   initialization bank from the other project, and finally load banks from that project.
/// > - Codecs and plug-ins must be registered before loading banks that use them.
/// > - Loading a bank referencing an unregistered plug-in or codec will result in a load bank success,
/// but the plug-ins will not be used. More specifically, playing a sound that uses an unregistered effect plug-in
/// will result in audio playback without applying the said effect. If an unregistered source plug-in is used by an event's audio objects,
/// posting the event will fail.
/// > - The sound engine internally calls get_id_from_string(name) to return the correct bank ID.
/// Therefore, in_pszString should be the real name of the SoundBank (with or without the BNK extension - it is trimmed internally),
/// not the name of the file (if you changed it), nor the full path of the file. The path should be resolved in
/// your implementation of the Stream Manager, or in the Low-Level I/O module if you use the default Stream Manager's implementation.
///
/// *See also*
/// > - [unload_bank_by_name]
/// > - [unload_bank_by_id]
/// > - [clear_banks]
/// > - [get_id_from_string]
pub fn load_bank_from_memory(data: &[u8]) -> Result<AkBankID, AkResult> {
    let mut bank_id = 0;
    ak_call_result![LoadBankMemoryView(data.as_ptr().cast(), data.len().try_into().unwrap(), &mut bank_id) => bank_id]
}

#[derive(Debug, Copy, Clone)]
/// Helper to post events to the sound engine.
///
/// Use [PostEvent::post] to post your event to the sound engine.
///
/// The callback function can be used to be noticed when markers are reached or when the event is finished.
///
/// An array of Wave file sources can be provided to resolve External Sources triggered by the event.
///
/// *Return* The playing ID of the event launched, or [AK_INVALID_PLAYING_ID] if posting the event failed
///
/// *Remarks*
/// > - If used, the array of external sources should contain the information for each external source triggered by the
/// event. When triggering an Event with multiple external sources, you need to differentiate each source
/// by using the cookie property in the External Source in the Wwise project and in AkExternalSourceInfo.
/// > - If an event triggers the playback of more than one external source, they must be named uniquely in the project
/// (therefore have a unique cookie) in order to tell them apart when filling the AkExternalSourceInfo structures.
///
/// *See also*
/// > - [render_audio]
/// > - [get_source_play_position]
pub struct PostEvent<'a> {
    game_obj_id: AkGameObjectID,
    event_id: AkID<'a>,
    flags: AkCallbackType,
    // external_sources: Vec<...>  // TODO
    playing_id: AkPlayingID,
}

impl<'a> PostEvent<'a> {
    /// Select an event by name or by ID, to play on a given game object.
    pub fn new<T: Into<AkID<'a>>>(game_obj_id: AkGameObjectID, event_id: T) -> PostEvent<'a> {
        PostEvent {
            game_obj_id,
            event_id: event_id.into(),
            flags: AkCallbackType(0),
            // external_sources: ...,
            playing_id: AK_INVALID_PLAYING_ID,
        }
    }

    /// Add flags before posting. Bitmask: see [AkCallbackType].
    ///
    /// *See also* [post_with_callback](Self::post_with_callback)
    pub fn add_flags(&mut self, flags: AkCallbackType) -> &mut Self {
        self.flags |= flags;
        self
    }

    /// Set flags before posting. Bitmask: see [AkCallbackType]
    ///
    /// *See also* [post_with_callback](Self::post_with_callback)
    pub fn flags(&mut self, flags: AkCallbackType) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Advanced users only. Specify the playing ID to target with the event. Will Cause active
    /// actions in this event to target an existing Playing ID. Let it be [AK_INVALID_PLAYING_ID]
    /// or do not specify any for normal playback.
    pub fn playing_id(&mut self, id: AkPlayingID) -> &mut Self {
        self.playing_id = id;
        self
    }

    /// Posts the event to the sound engine.
    pub fn post(&self) -> Result<AkPlayingID, AkResult> {
        if let AkID::Name(name) = self.event_id {
            let ak_playing_id = unsafe {
                with_cstring![name => cname {
                    PostEvent2(
                        cname.as_ptr(),
                        self.game_obj_id,
                        self.flags.0 as u32,
                        None,
                        ::std::ptr::null_mut(),
                        0,                      // TODO
                        ::std::ptr::null_mut(), // TODO
                        self.playing_id,
                    )
                }]
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else if let AkID::ID(id) = self.event_id {
            let ak_playing_id = unsafe {
                PostEvent(
                    id,
                    self.game_obj_id,
                    self.flags.0 as u32,
                    None,
                    ::std::ptr::null_mut(),
                    0,                      // TODO
                    ::std::ptr::null_mut(), // TODO
                    self.playing_id,
                )
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else {
            panic!("need at least an event ID or and an event name to post")
        }
    }

    /// Posts the event to the sound engine, calling `callback` according to [flags](Self::flags).
    ///
    /// `callback` can be a function or a closure.
    ///
    /// **⚡ ATTENTION ⚡**
    ///
    /// `callback` will be called on the audio thread, **not** on the current thread where you called
    /// this. This means your closure or function must access shared state in a thread-safe way.
    ///
    /// This also means the closure or function must not be long to return, or audio might sutter as
    /// it prevents the audio thread from processing buffers.
    pub fn post_with_callback<F>(&self, callback: F) -> Result<AkPlayingID, AkResult>
    where
        F: FnMut(crate::AkCallbackInfo) + 'static,
    {
        // see http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c
        let data = Box::into_raw(Box::new(callback));

        if let AkID::Name(name) = self.event_id {
            let ak_playing_id = unsafe {
                with_cstring![name => cname {
                    PostEvent2(
                        cname.as_ptr(),
                        self.game_obj_id,
                        (self.flags | AkCallbackType::AK_EndOfEvent).0 as u32,
                        Some(Self::call_callback_as_closure::<F>),
                        data as *mut _,
                        0,                      // TODO
                        ::std::ptr::null_mut(), // TODO
                        self.playing_id,
                    )
                }]
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else if let AkID::ID(id) = self.event_id {
            let ak_playing_id = unsafe {
                PostEvent(
                    id,
                    self.game_obj_id,
                    (self.flags | AkCallbackType::AK_EndOfEvent).0 as u32,
                    Some(Self::call_callback_as_closure::<F>),
                    data as *mut _,
                    0,                      // TODO
                    ::std::ptr::null_mut(), // TODO
                    self.playing_id,
                )
            };
            if ak_playing_id == AK_INVALID_PLAYING_ID {
                Err(AkResult::AK_Fail)
            } else {
                Ok(ak_playing_id)
            }
        } else {
            panic!("need at least an event ID or and an event name to post")
        }
    }

    unsafe extern "C" fn call_callback_as_closure<F>(
        cb_type: AkCallbackType,
        cb_info: *mut bindings::root::AkCallbackInfo,
    ) where
        F: FnMut(crate::AkCallbackInfo),
    {
        // see http://blog.sagetheprogrammer.com/neat-rust-tricks-passing-rust-closures-to-c

        let callback_ptr: *mut F;
        let wrapped_cb_type: crate::AkCallbackInfo;
        if cb_type.contains(AkCallbackType::AK_MusicSyncAll) {
            let cb_info = *(cb_info as *mut AkMusicSyncCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::MusicSync {
                game_obj_id: cb_info._base.gameObjID,
                playing_id: cb_info.playingID,
                segment_info: cb_info.segmentInfo,
                music_sync_type: cb_info.musicSyncType,

                user_cue_name: if cb_info.pszUserCueName.is_null() {
                    "".to_string()
                } else {
                    // Safety
                    // pszUserCueName will be valid until to_string(), which will copy the bytes from
                    // pszUserCueName onto the Rust-managed heap
                    CStr::from_ptr(cb_info.pszUserCueName as *const ::std::os::raw::c_char)
                        .to_str()
                        .unwrap()
                        .to_string()
                },
            };
        } else if cb_type.contains(AkCallbackType::AK_EndOfDynamicSequenceItem) {
            let cb_info = *(cb_info as *mut AkDynamicSequenceItemCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::DynamicSequenceItem {
                game_obj_id: cb_info._base.gameObjID,
                playing_id: cb_info.playingID,
                audio_node_id: cb_info.audioNodeID,
            };
        } else if cb_type.contains(
            AkCallbackType::AK_EndOfEvent
                | AkCallbackType::AK_MusicPlayStarted
                | AkCallbackType::AK_Starvation,
        ) {
            let cb_info = *(cb_info as *mut AkEventCallbackInfo);
            callback_ptr = cb_info._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Event {
                game_obj_id: cb_info._base.gameObjID,
                callback_type: cb_type,
                playing_id: cb_info.playingID,
                event_id: cb_info.eventID,
            };
        } else if cb_type.contains(AkCallbackType::AK_Duration) {
            let cb_info = *(cb_info as *mut AkDurationCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Duration {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                duration: cb_info.fDuration,
                estimated_duration: cb_info.fEstimatedDuration,
                audio_node_id: cb_info.audioNodeID,
                media_id: cb_info.mediaID,
                streaming: cb_info.bStreaming,
            };
        } else if cb_type.contains(AkCallbackType::AK_Marker) {
            let cb_info = *(cb_info as *mut AkMarkerCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Marker {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                identifier: cb_info.uIdentifier,
                position: cb_info.uPosition,
                label: if cb_info.strLabel.is_null() {
                    "".to_string()
                } else {
                    // Safety
                    // strLabel will be valid until to_string(), which will copy the bytes from
                    // strLabel onto the Rust-managed heap
                    CStr::from_ptr(cb_info.strLabel)
                        .to_str()
                        .unwrap()
                        .to_string()
                },
            }
        } else if cb_type.contains(AkCallbackType::AK_MIDIEvent) {
            let cb_info = *(cb_info as *mut AkMIDIEventCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Midi {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                midi_event: cb_info.midiEvent.into(),
            }
        } else if cb_type.contains(AkCallbackType::AK_MusicPlaylistSelect) {
            let cb_info = *(cb_info as *mut AkMusicPlaylistCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::MusicPlaylist {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                playlist_id: cb_info.playlistID,
                num_playlist_items: cb_info.uNumPlaylistItems,
                playlist_selection: cb_info.uPlaylistSelection,
                playlist_item_done: cb_info.uPlaylistItemDone,
            }
        } else if cb_type.contains(AkCallbackType::AK_SpeakerVolumeMatrix) {
            let cb_info = *(cb_info as *mut AkSpeakerVolumeMatrixCallbackInfo);
            callback_ptr = cb_info._base._base.pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::SpeakerMatrixVolume {
                game_obj_id: cb_info._base._base.gameObjID,
                playing_id: cb_info._base.playingID,
                event_id: cb_info._base.eventID,
                input_config: cb_info.inputConfig,
                output_config: cb_info.outputConfig,
            }
        } else {
            if !cb_type.contains(AkCallbackType::AK_CallbackBits) {
                // is it safe to panic here?
                panic!("Unexpected AkCallbackType encountered: {:?}", cb_type.0);
            }

            callback_ptr = (*cb_info).pCookie as *mut F;
            wrapped_cb_type = crate::AkCallbackInfo::Default {
                game_obj_id: (*cb_info).gameObjID,
                callback_type: cb_type,
            };
        }
        let callback = &mut *callback_ptr;

        // Info needed: is this safe if the callback panics? Should we do something with
        // catch_unwind? Is this undefined behavior?
        callback(wrapped_cb_type);

        if cb_type.contains(AkCallbackType::AK_EndOfEvent) {
            // No more callbacks to process! Cleanup memory
            drop(Box::from_raw(callback_ptr));
        }
    }
}
