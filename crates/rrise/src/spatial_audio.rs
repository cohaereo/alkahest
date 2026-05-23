/*
 * Copyright (c) 2022 Contributors to the Rrise project
 */

use crate::{
    ak_call_result,
    bindings::root::{
        AkSpatialAudioInitSettings, AK::SpatialAudio::*, AK_MAX_SOUND_PROPAGATION_DEPTH,
    },
    AkResult,
};

pub fn init() -> Result<(), AkResult> {
    let spatial_settings = AkSpatialAudioInitSettings {
        uMaxSoundPropagationDepth: AK_MAX_SOUND_PROPAGATION_DEPTH,
        fMovementThreshold: 0.25,
        uNumberOfPrimaryRays: 35,
        uMaxReflectionOrder: 2,
        uMaxDiffractionOrder: 4,
        uMaxEmitterRoomAuxSends: 3,
        uDiffractionOnReflectionsOrder: 2,
        fMaxPathLength: 1000.0,
        fCPULimitPercentage: 0.0,
        uLoadBalancingSpread: 1,
        bEnableGeometricDiffractionAndTransmission: true,
        bCalcEmitterVirtualPosition: true,
    };
    ak_call_result![Init(&spatial_settings)]?;
    Ok(())
}

pub fn register_listener(listener_id: u64) -> Result<(), AkResult> {
    ak_call_result![RegisterListener(listener_id)]?;
    Ok(())
}

pub fn unregister_listener(listener_id: u64) -> Result<(), AkResult> {
    ak_call_result![UnregisterListener(listener_id)]?;
    Ok(())
}
