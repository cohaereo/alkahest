/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

//! Everything related to RTPC, Switch, States and Triggers.

use crate::bindings::root::AK::SoundEngine::{
    PostTrigger, PostTrigger2, ResetRTPCValue, ResetRTPCValue2, SetRTPCValue, SetRTPCValue2,
    SetRTPCValueByPlayingID, SetRTPCValueByPlayingID2, SetState, SetState2, SetSwitch, SetSwitch2,
};
use crate::{
    ak_call_result, with_cstring, AkCurveInterpolation, AkGameObjectID, AkID, AkPlayingID,
    AkResult, AkRtpcValue, AkTimeMs, AK_INVALID_GAME_OBJECT, AK_INVALID_PLAYING_ID,
};

/// Helper to set or reset RTPCs.
///
/// Use [SetRtpcValue::set] or [SetRtpcValue::reset] to post the change to Wwise.
#[derive(Debug, Copy, Clone)]
pub struct SetRtpcValue<'a> {
    rtpc_id: AkID<'a>,
    value: AkRtpcValue,
    target_game_obj_id: AkGameObjectID,
    target_playing_id: AkPlayingID,
    interp_ms: AkTimeMs,
    fade_curve: AkCurveInterpolation,
    bypass_designer_interp: bool,
}

impl<'a> SetRtpcValue<'a> {
    pub fn new<T: Into<AkID<'a>>>(rtpc_id: T, value: AkRtpcValue) -> Self {
        Self {
            rtpc_id: rtpc_id.into(),
            value,
            target_game_obj_id: AK_INVALID_GAME_OBJECT,
            target_playing_id: AK_INVALID_PLAYING_ID,
            interp_ms: 0,
            fade_curve: AkCurveInterpolation::AkCurveInterpolation_Linear,
            bypass_designer_interp: false,
        }
    }

    /// Value to set.
    ///
    /// *Note* Doesn't consume self so that it's easier to reuse the same SetRTPCValue struct with
    /// different values over time.
    pub fn with_value(&mut self, value: AkRtpcValue) -> &mut Self {
        self.value = value;
        self
    }

    /// Associated game object ID.
    ///
    /// With this function, you may set a game parameter value with global scope (default) or with game object scope.
    /// Game object scope superseeds global scope. (Once a value is set for the game object scope, it will not be affected by changes to the global scope value.)
    /// Game parameter values set with global scope are applied to all game objects that not yet registered, or already registered but not overridden with a value
    /// with game object scope.
    /// To set a game parameter value with global scope, leave the default or pass [AK_INVALID_GAME_OBJECT] as the game object.
    ///
    /// Default: [AK_INVALID_GAME_OBJECT]
    pub fn for_target(mut self, game_obj_id: AkGameObjectID) -> Self {
        self.target_game_obj_id = game_obj_id;
        self
    }

    /// Associated playing ID.
    ///
    /// With this function, you may set a game parameter value on playing ID scope.
    /// Playing id scope superseeds both game object scope and global scope.
    /// Ignores any target game object set with [for_target](Self::for_target).
    ///
    /// Default: [AK_INVALID_PLAYING_ID]
    pub fn for_playing_id(mut self, playing_id: AkPlayingID) -> Self {
        self.target_playing_id = playing_id;
        self
    }

    /// Duration during which the game parameter is interpolated towards in_value.
    ///
    /// With this function, you may change the value of a game parameter over time. To do so, specify a non-zero
    /// value for `ms`. At each audio frame, the game parameter value will be updated internally
    /// according to the interpolation curve (set with [with_interp_curve](Self::with_interp_curve)).
    /// If you call [set](Self::set) with `ms = 0` in the middle of an interpolation, the interpolation stops and
    /// the new value is set directly. Thus, if you call [set](Self::set) at every game frame, you
    /// should not use `with_interp_millis`, as it would have no effect and it is less efficient.
    ///
    /// Default: `0`
    pub fn with_interp_millis(mut self, ms: AkTimeMs) -> Self {
        self.interp_ms = ms;
        self
    }

    /// When [with_interp_millis](Self::with_interp_millis) is used and non-zero, describes the Curve
    /// type to be used for the game parameter interpolation.
    ///
    /// Default: [AkCurveInterpolation::AkCurveInterpolation_Linear]
    pub fn with_interp_curve(mut self, curve: AkCurveInterpolation) -> Self {
        self.fade_curve = curve;
        self
    }

    /// `True` if you want to bypass the internal "slew rate" or "over time filtering" specified by the sound designer.
    /// This is meant to be used when for example loading a level and you dont want the values to interpolate.
    ///
    /// Default: `false`
    pub fn should_bypass_designer_interp(mut self, should: bool) -> Self {
        self.bypass_designer_interp = should;
        self
    }

    /// Post this RTPC configuration to Wwise.
    pub fn set(&self) -> Result<(), AkResult> {
        if self.target_playing_id != AK_INVALID_PLAYING_ID {
            match self.rtpc_id {
                AkID::Name(name) => with_cstring![name => cname {
                    ak_call_result![SetRTPCValueByPlayingID2(
                        cname.as_ptr(),
                        self.value,
                        self.target_playing_id,
                        self.interp_ms,
                        self.fade_curve,
                        self.bypass_designer_interp
                    )
                ]}],
                AkID::ID(id) => ak_call_result![SetRTPCValueByPlayingID(
                    id,
                    self.value,
                    self.target_playing_id,
                    self.interp_ms,
                    self.fade_curve,
                    self.bypass_designer_interp
                )],
            }
        } else {
            match self.rtpc_id {
                AkID::Name(name) => with_cstring![name => cname {
                    ak_call_result![SetRTPCValue2(
                        cname.as_ptr(),
                        self.value,
                        self.target_game_obj_id,
                        self.interp_ms,
                        self.fade_curve,
                        self.bypass_designer_interp
                    )
                ]}],
                AkID::ID(id) => ak_call_result![SetRTPCValue(
                    id,
                    self.value,
                    self.target_game_obj_id,
                    self.interp_ms,
                    self.fade_curve,
                    self.bypass_designer_interp
                )],
            }
        }
    }

    /// Post a reset of this RTPC configuration to Wwise.
    ///
    /// Resets the value of the game parameter to its default value, as specified in the Wwise project.
    ///
    /// Incompatible with [for_playing_id](Self::for_playing_id); only works at global or game object
    /// scope, set with [for_target](Self::for_target).
    pub fn reset(&self) -> Result<(), AkResult> {
        match self.rtpc_id {
            AkID::Name(name) => with_cstring![name => cname {
                ak_call_result![ResetRTPCValue2(
                    cname.as_ptr(),
                    self.target_game_obj_id,
                    self.interp_ms,
                    self.fade_curve,
                    self.bypass_designer_interp
                )
            ]}],
            AkID::ID(id) => ak_call_result![ResetRTPCValue(
                id,
                self.target_game_obj_id,
                self.interp_ms,
                self.fade_curve,
                self.bypass_designer_interp
            )],
        }
    }
}

/// Sets the State of a Switch Group.
///
/// Always returns [AkResult::AK_Success].
///
/// Panics if switch_group and switch_id are not of the same variant.
pub fn set_switch<'a, T: Into<AkID<'a>>>(
    switch_group: T,
    switch_id: T,
    game_obj: AkGameObjectID,
) -> Result<(), AkResult> {
    match (switch_group.into(), switch_id.into()) {
        (AkID::Name(group), AkID::Name(switch)) => {
            with_cstring![group => groupc, switch => switchc {
                ak_call_result![SetSwitch2(
                    groupc.as_ptr(),
                    switchc.as_ptr(),
                    game_obj,
                )
            ]}]
        }
        (AkID::ID(group), AkID::ID(switch)) => {
            ak_call_result![SetSwitch(group, switch, game_obj)]
        }
        _ => panic!("Args switch_group and switch_id should be of the same variant"),
    }
}

/// Post the specified trigger.
///
/// Always returns [AkResult::AK_Success].
pub fn post_trigger<'a, T: Into<AkID<'a>>>(
    trigger: T,
    game_obj: AkGameObjectID,
) -> Result<(), AkResult> {
    match trigger.into() {
        AkID::Name(name) => {
            with_cstring![name => cname {
                ak_call_result![PostTrigger2(
                    cname.as_ptr(),
                    game_obj,
                )
            ]}]
        }
        AkID::ID(id) => {
            ak_call_result![PostTrigger(id, game_obj)]
        }
    }
}

/// Sets the state of a State Group.
///
/// *Return*
/// > - [AkResult::AK_Success] if successful
/// > - [AkResult::AK_IDNotFound] if the state or State Group name was not resolved to an existing ID.
/// Make sure that the banks were generated with the "include string" option.
///
/// Panics if state_group and state_id are not of the same variant.
pub fn set_state<'a, T: Into<AkID<'a>>>(state_group: T, state_id: T) -> Result<(), AkResult> {
    match (state_group.into(), state_id.into()) {
        (AkID::Name(group), AkID::Name(state)) => {
            with_cstring![group => groupc, state => statec {
                ak_call_result![SetState2(
                    groupc.as_ptr(),
                    statec.as_ptr()
                )
            ]}]
        }
        (AkID::ID(group), AkID::ID(state)) => {
            ak_call_result![SetState(group, state)]
        }
        _ => panic!("Args state_group and state_id should be of the same variant"),
    }
}
