/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

//! The sound engine parameter query interface.
//!
//! *Remark* The functions in this namespace are thread-safe, unless stated otherwise.
//!
//! *Warnings*
//!
//! ⚡ Unless noted otherwise in the function definition that it will not acquire the main
//! audio lock, the functions in this namespace might stall for several milliseconds before returning
//! (as they cannot execute while the main sound engine thread is busy). They should therefore not
//! be called from any game critical thread, such as the main game loop.
//!
//! ⚡ There might be a significant delay between a Sound Engine call (such as PostEvent) and
//! the information being reflected in a Query (such as GetIsGameObjectActive).

use std::mem::zeroed;

use crate::{
    ak_call_result,
    bindings::root::AK::SoundEngine::Query::{
        GetListenerPosition, GetListeners, GetPosition, GetRTPCValue, GetRTPCValue2, GetState,
        GetState2, GetSwitch, GetSwitch2, RTPCValue_type,
    },
    with_cstring, AkGameObjectID, AkID, AkListenerPosition, AkPlayingID, AkResult, AkRtpcValue,
    AkSoundPosition, AkStateID, AkSwitchStateID, AK_INVALID_GAME_OBJECT, AK_INVALID_PLAYING_ID,
};

/// Enum used to request a specific RTPC Value.
///
/// Also used to inform the user of where the RTPC Value comes from.
///
/// For example, the user may request the GameObject specific value by specifying RTPCValue_GameObject
/// and can receive the Global Value if there was no GameObject specific value, and even the
/// default value is there was no Global value either.
///
/// *See also* [get_rtpc_value]
#[derive(Debug, Copy, Clone)]
pub enum RtpcValueType {
    /// The value is the Default RTPC.
    Default(AkRtpcValue),
    /// The value is the Global RTPC.
    Global(AkRtpcValue),
    /// The value is the game object specific RTPC.
    GameObject(AkRtpcValue),
    /// The value is the playing ID specific RTPC.
    PlayingID(AkRtpcValue),
    /// The value is not available for the RTPC specified.
    Unavailable(),
}

/// Get the position of a game object.
///
/// *Return* [AkResult::AK_Success] if succeeded, or [AkResult::AK_IDNotFound] if the game object
/// was not registered
pub fn get_position(game_obj: AkGameObjectID) -> Result<AkSoundPosition, AkResult> {
    let mut result = unsafe { zeroed() };
    ak_call_result![GetPosition(game_obj, &mut result)]?;
    Ok(result)
}

/// Get a game object's listeners count.
///
/// *Return* the number of listeners of `for_source` if succeeded, or [AkResult::AK_IDNotFound] if the game object was not registered.
pub fn get_listeners_count(for_source: AkGameObjectID) -> Result<u32, AkResult> {
    let mut result = 0;
    ak_call_result![GetListeners(for_source, std::ptr::null_mut(), &mut result)]?;
    Ok(result)
}

/// Get a game object's listeners.
///
/// *Return* at most `max_count` listeners of `for_source` if succeeded, or [AkResult::AK_IDNotFound] if the game object was not registered.
pub fn get_listeners(
    for_source: AkGameObjectID,
    max_count: usize,
) -> Result<Vec<AkGameObjectID>, AkResult> {
    let mut result = Vec::with_capacity(max_count);
    result.resize(max_count, AK_INVALID_GAME_OBJECT);
    ak_call_result![GetListeners(
        for_source,
        result.as_mut_ptr(),
        &mut (max_count as u32)
    )]?;
    result.resize(max_count, AK_INVALID_GAME_OBJECT);
    Ok(result)
}

/// Get a listener's position.
///
/// Listener index is 0 for first listener, 7 for 8th listener, etc.
///
/// *Return* the position if succeeded, or [AkResult::AK_InvalidParameter] if the index is out of range
pub fn get_listener_position(index: u64) -> Result<AkListenerPosition, AkResult> {
    let mut result = unsafe { zeroed() };
    ak_call_result![GetListenerPosition(index, &mut result)]?;
    Ok(result)
}

/// Get the value of a real-time parameter control.
///
/// An RTPC can have a any combination of a global value, a unique value for each game object, or a unique value for each playing ID.
/// The value requested is determined by `requested_scope`, `on_game_obj` and `on_playing_id`.
///
/// If a value at the requested scope is not found, the value that is available at the the next broadest scope will be returned.
///
/// *Remark*
/// > When looking up RTPC values via playing ID (ie. `requested_scope` is [RtpcValueType::PlayingID]), `on_game_obj` can be set to a specific game object (if it is available to the caller) to use as a fall back value.
/// If the game object is unknown or unavailable, [None] or [AK_INVALID_GAME_OBJECT] can be passed in `on_game_obj`, and the game object will be looked up via `on_playing_id`.
/// However in this case, it is not possible to retrieve a game object value as a fall back value if the playing id does not exist. It is best to pass in the game object if possible.
///
/// > `on_game_obj` is ignored if `requested_scope` is [RtpcValueType::Global]
///
/// > `on_playing_id` is ignored if `requested_scope` is not [RtpcValueType::PlayingID]
///
/// *Return*
/// - The scoped requested value if succeeded.
/// - [AkResult::AK_IDNotFound] if the game object was not registered or the RTPC name could not be found.
/// - [AkResult::AK_Fail] if the RTPC value could not be obtained.
///
/// *See also* [RtpcValueType]
pub fn get_rtpc_value<'a, T: Into<AkID<'a>>>(
    rtpc_id: T,
    on_game_obj: Option<AkGameObjectID>,
    on_playing_id: Option<AkPlayingID>,
    requested_scope: RtpcValueType,
) -> Result<RtpcValueType, AkResult> {
    let on_game_obj = match on_game_obj {
        Some(game_obj) => game_obj,
        None => AK_INVALID_GAME_OBJECT,
    };
    let on_playing_id = match on_playing_id {
        Some(playing_id) => playing_id,
        None => AK_INVALID_PLAYING_ID,
    };
    let mut value_type = match requested_scope {
        RtpcValueType::Default(_) => RTPCValue_type::RTPCValue_Default,
        RtpcValueType::Global(_) => RTPCValue_type::RTPCValue_Global,
        RtpcValueType::GameObject(_) => RTPCValue_type::RTPCValue_GameObject,
        RtpcValueType::PlayingID(_) => RTPCValue_type::RTPCValue_PlayingID,
        RtpcValueType::Unavailable() => RTPCValue_type::RTPCValue_Unavailable,
    };
    let mut value = AkRtpcValue::default();

    match rtpc_id.into() {
        AkID::Name(name) => with_cstring![name => cname {
            ak_call_result![GetRTPCValue2(
                cname.as_ptr(),
                on_game_obj,
                on_playing_id,
                &mut value,
                &mut value_type
            )
        ]}]?,
        AkID::ID(id) => ak_call_result![GetRTPCValue(
            id,
            on_game_obj,
            on_playing_id,
            &mut value,
            &mut value_type
        )]?,
    };

    let result = match value_type {
        RTPCValue_type::RTPCValue_Default => RtpcValueType::Default(value),
        RTPCValue_type::RTPCValue_Global => RtpcValueType::Global(value),
        RTPCValue_type::RTPCValue_GameObject => RtpcValueType::GameObject(value),
        RTPCValue_type::RTPCValue_PlayingID => RtpcValueType::PlayingID(value),
        RTPCValue_type::RTPCValue_Unavailable => RtpcValueType::Unavailable(),
    };

    Ok(result)
}

/// Get the state of a switch group.
///
/// *Return* the active switch state if succeeded, or [AkResult::AK_IDNotFound] if the game object was not registered
pub fn get_switch<'a, T: Into<AkID<'a>>>(
    switch_group: T,
    on_game_obj: AkGameObjectID,
) -> Result<AkSwitchStateID, AkResult> {
    let mut result = AkSwitchStateID::default();

    match switch_group.into() {
        AkID::Name(name) => with_cstring![name => cname {
            ak_call_result![GetSwitch2(
                cname.as_ptr(),
                on_game_obj,
                &mut result
            )
        ]}]?,
        AkID::ID(id) => ak_call_result![GetSwitch(id, on_game_obj, &mut result)]?,
    };

    Ok(result)
}

/// Get the state of a state group.
///
/// *Return* the active state if succeeded, or [AkResult::AK_IDNotFound] if the state group name can not be found
pub fn get_state<'a, T: Into<AkID<'a>>>(state_group: T) -> Result<AkStateID, AkResult> {
    let mut result = AkStateID::default();

    match state_group.into() {
        AkID::Name(name) => with_cstring![name => cname {
            ak_call_result![GetState2(
                cname.as_ptr(),
                &mut result
            )
        ]}]?,
        AkID::ID(id) => ak_call_result![GetState(id, &mut result)]?,
    };

    Ok(result)
}
