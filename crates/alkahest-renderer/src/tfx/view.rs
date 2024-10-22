use alkahest_data::tfx::TfxRenderStage;
use glam::Vec3;
use std::ops::{BitOr, BitOrAssign};

use super::externs;
use crate::{camera::Viewport, ecs::culling::Frustum};

pub trait View {
    fn viewport(&self) -> Viewport;
    fn subscribed_views(&self) -> RenderStageSubscriptions;
    fn name(&self) -> String;
    fn frustum(&self) -> Frustum;

    fn update_extern(&self, x: &mut externs::View);
}

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct RenderStageSubscriptions: u32 {
        const GENERATE_GBUFFER                  = 1 << TfxRenderStage::GenerateGbuffer as u32;
        const DECALS                            = 1 << TfxRenderStage::Decals as u32;
        const INVESTMENT_DECALS                 = 1 << TfxRenderStage::InvestmentDecals as u32;
        const SHADOW_GENERATE                   = 1 << TfxRenderStage::ShadowGenerate as u32;
        const LIGHTING_APPLY                    = 1 << TfxRenderStage::LightingApply as u32;
        const LIGHT_PROBE_APPLY                 = 1 << TfxRenderStage::LightProbeApply as u32;
        const DECALS_ADDITIVE                   = 1 << TfxRenderStage::DecalsAdditive as u32;
        const TRANSPARENTS                      = 1 << TfxRenderStage::Transparents as u32;
        const DISTORTION                        = 1 << TfxRenderStage::Distortion as u32;
        const LIGHT_SHAFT_OCCLUSION             = 1 << TfxRenderStage::LightShaftOcclusion as u32;
        const SKIN_PREPASS                      = 1 << TfxRenderStage::SkinPrepass as u32;
        const LENS_FLARES                       = 1 << TfxRenderStage::LensFlares as u32;
        const DEPTH_PREPASS                     = 1 << TfxRenderStage::DepthPrepass as u32;
        const WATER_REFLECTION                  = 1 << TfxRenderStage::WaterReflection as u32;
        const POSTPROCESS_TRANSPARENT_STENCIL   = 1 << TfxRenderStage::PostprocessTransparentStencil as u32;
        const IMPULSE                           = 1 << TfxRenderStage::Impulse as u32;
        const RETICLE                           = 1 << TfxRenderStage::Reticle as u32;
        const WATER_RIPPLES                     = 1 << TfxRenderStage::WaterRipples as u32;
        const MASK_SUN_LIGHT                    = 1 << TfxRenderStage::MaskSunLight as u32;
        const VOLUMETRICS                       = 1 << TfxRenderStage::Volumetrics as u32;
        const CUBEMAPS                          = 1 << TfxRenderStage::Cubemaps as u32;
        const POSTPROCESS_SCREEN                = 1 << TfxRenderStage::PostprocessScreen as u32;
        const WORLD_FORCES                      = 1 << TfxRenderStage::WorldForces as u32;
        const COMPUTE_SKINNING                  = 1 << TfxRenderStage::ComputeSkinning as u32;
    }
}

impl RenderStageSubscriptions {
    pub fn is_subscribed(&self, stage: TfxRenderStage) -> bool {
        self.contains(Self::from_bits_truncate(1 << stage as u32))
    }

    pub fn from_partrange_list(part_ranges: &[u16]) -> Self {
        let mut flags = Self::empty();
        for i in 0..24 {
            if part_ranges[i] != part_ranges[i + 1] {
                flags |= Self::from_bits_truncate(1 << i);
            }
        }
        flags
    }
}

impl From<TfxRenderStage> for RenderStageSubscriptions {
    fn from(stage: TfxRenderStage) -> Self {
        Self::from_bits_truncate(1 << stage as u32)
    }
}

impl BitOr<TfxRenderStage> for RenderStageSubscriptions {
    type Output = Self;

    fn bitor(self, rhs: TfxRenderStage) -> Self::Output {
        self | Self::from(rhs)
    }
}

impl BitOrAssign<TfxRenderStage> for RenderStageSubscriptions {
    fn bitor_assign(&mut self, rhs: TfxRenderStage) {
        *self |= Self::from(rhs);
    }
}
