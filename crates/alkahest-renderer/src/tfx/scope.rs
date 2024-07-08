// pub trait Scope {
//     fn size(&self) -> usize;
// }

use std::io::Write;

use alkahest_data::{
    render_globals::{SScope, SScopeStage},
    tfx::TfxShaderStage,
};
use alkahest_pm::package_manager;
use glam::{Mat4, Vec2, Vec3, Vec4};
use windows::Win32::Graphics::Direct3D11::ID3D11SamplerState;

use crate::{
    gpu::{buffer::ConstantBufferCached, SharedGpuContext},
    renderer::Renderer,
    tfx::{
        bytecode::{interpreter::TfxBytecodeInterpreter, opcodes::TfxBytecodeOp},
        externs,
    },
};

/// A scope generated through TFX expressions.
pub struct TfxScope {
    scope: SScope,

    pub stage_pixel: Option<TfxScopeStage>,
    pub stage_vertex: Option<TfxScopeStage>,
    pub stage_geometry: Option<TfxScopeStage>,
    pub stage_compute: Option<TfxScopeStage>,
}

impl TfxScope {
    pub fn load(scope: SScope, gctx: SharedGpuContext) -> anyhow::Result<TfxScope> {
        let sscope = scope.clone();
        let stage_vertex = if sscope.stage_vertex.constants.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_vertex, TfxShaderStage::Vertex, gctx.clone())?
        } else {
            None
        };

        let stage_pixel = if sscope.stage_pixel.constants.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_pixel, TfxShaderStage::Pixel, gctx.clone())?
        } else {
            None
        };

        let stage_geometry = if sscope.stage_geometry.constants.constant_buffer_slot != -1 {
            TfxScopeStage::load(
                sscope.stage_geometry,
                TfxShaderStage::Geometry,
                gctx.clone(),
            )?
        } else {
            None
        };

        let stage_compute = if sscope.stage_compute.constants.constant_buffer_slot != -1 {
            TfxScopeStage::load(sscope.stage_compute, TfxShaderStage::Compute, gctx.clone())?
        } else {
            None
        };

        Ok(TfxScope {
            scope,
            stage_pixel,
            stage_vertex,
            stage_geometry,
            stage_compute,
        })
    }

    pub fn bind(&self, renderer: &Renderer) -> anyhow::Result<()> {
        if let Some(stage) = &self.stage_vertex {
            stage.bind(renderer)?;
        }

        if let Some(stage) = &self.stage_pixel {
            stage.bind(renderer)?;
        }

        if let Some(stage) = &self.stage_geometry {
            stage.bind(renderer)?;
        }

        if let Some(stage) = &self.stage_compute {
            stage.bind(renderer)?;
        }

        Ok(())
    }

    pub fn vertex_slot(&self) -> i32 {
        self.scope.stage_vertex.constants.constant_buffer_slot
    }
}

pub struct TfxScopeStage {
    pub stage: SScopeStage,
    shader_stage: TfxShaderStage,

    samplers: Vec<Option<ID3D11SamplerState>>,

    pub cbuffer: Option<ConstantBufferCached<Vec4>>,
    bytecode: Option<TfxBytecodeInterpreter>,
}

impl TfxScopeStage {
    pub fn load(
        stage: SScopeStage,
        shader_stage: TfxShaderStage,
        gctx: SharedGpuContext,
    ) -> anyhow::Result<Option<TfxScopeStage>> {
        let cbuffer = if stage.constants.constant_buffer.is_some() {
            let buffer_header_ref = package_manager()
                .get_entry(stage.constants.constant_buffer)
                .unwrap()
                .reference;

            let data_raw = package_manager().read_tag(buffer_header_ref).unwrap();

            let data = bytemuck::cast_slice(&data_raw);
            let buf = ConstantBufferCached::create_array_init(gctx.clone(), data).unwrap();

            Some(buf)
        } else if !stage.constants.unk38.is_empty() {
            let buf = ConstantBufferCached::create_array_init(
                gctx.clone(),
                bytemuck::cast_slice(&stage.constants.unk38),
            )
            .unwrap();

            Some(buf)
        } else {
            None
        };

        let bytecode =
            match TfxBytecodeOp::parse_all(&stage.constants.bytecode, binrw::Endian::Little) {
                Ok(opcodes) => Some(TfxBytecodeInterpreter::new(opcodes)),
                Err(e) => {
                    debug!(
                        "Failed to parse VS TFX bytecode: {e:?} (data={})",
                        hex::encode(&stage.constants.bytecode)
                    );
                    None
                }
            };

        let mut samplers = vec![];
        for sampler in stage.constants.samplers.iter() {
            samplers.push(crate::loaders::technique::load_sampler(&gctx, sampler.hash32()).ok());
        }

        Ok(Some(TfxScopeStage {
            stage,
            shader_stage,
            samplers,
            cbuffer,
            bytecode,
        }))
    }

    pub fn bind(&self, renderer: &Renderer) -> anyhow::Result<()> {
        if let Some(bytecode) = &self.bytecode {
            bytecode.evaluate(
                &renderer.gpu,
                &renderer.data.lock().externs,
                self.cbuffer.as_ref(),
                &self.stage.constants.bytecode_constants,
                &self.samplers,
            )?;
        }

        if self.stage.constants.constant_buffer_slot != -1 {
            if let Some(cbuffer) = &self.cbuffer {
                renderer.gpu.bind_cbuffer(
                    self.stage.constants.constant_buffer_slot as u32,
                    Some(cbuffer.buffer().clone()),
                    self.shader_stage,
                );
            }
        }

        Ok(())
    }
}

#[repr(C)]
pub struct ScopeFrame {
    // pub time: Vec4,               // c0
    pub game_time: f32,
    pub render_time: f32,
    pub delta_game_time: f32,
    pub exposure_time: f32,

    // pub exposure: Vec4,           // c1
    pub exposure_scale: f32,
    pub exposure_illum_relative_glow: f32,
    pub exposure_scale_for_shading: f32,
    pub exposure_illum_relative: f32,

    pub random_seed_scales: Vec4, // c2
    pub overrides: Vec4,          // c3

    // Light related
    pub unk4: Vec4, // c4
    pub unk5: Vec4, // c5

    pub unk6: Vec4, // c6
    pub unk7: Vec4, // c7

    pub unk8: Vec4, // c8
    pub unk9: Vec4, // c9
    pub unka: Vec4, // ca
}

impl From<&externs::Frame> for ScopeFrame {
    fn from(x: &externs::Frame) -> Self {
        ScopeFrame {
            game_time: x.game_time,
            render_time: x.render_time,
            delta_game_time: x.delta_game_time,
            exposure_time: x.exposure_time,

            exposure_scale: x.exposure_scale,
            exposure_illum_relative_glow: x.exposure_illum_relative * 16.0,
            exposure_scale_for_shading: x.exposure_scale,
            exposure_illum_relative: x.exposure_illum_relative,

            random_seed_scales: Vec4::new(
                (x.render_time + 33.75) * 1.258699,
                (x.render_time + 60.0) * 0.9583125,
                (x.render_time + 60.0) * 8.789123,
                (x.render_time + 33.75) * 2.311535,
            ),

            unk4: x.unk1c0,

            unk6: Vec4::new(0.0, 1.0, (x.render_time * 6.0).sin() * 0.5 + 0.5, 0.0),

            ..Default::default()
        }
    }
}

impl Default for ScopeFrame {
    fn default() -> Self {
        ScopeFrame {
            game_time: Default::default(),
            render_time: Default::default(),
            delta_game_time: Default::default(),
            exposure_time: 1. / 60.,
            exposure_scale: 1.0, // 0.5674781799316406,
            exposure_illum_relative_glow: 23.386_974,
            exposure_scale_for_shading: 0.567_478_2,
            exposure_illum_relative: 1.461_685_9,
            random_seed_scales: Vec4::new(102.850_5, 102.048_53, 943.289_06, 187.406_77),
            overrides: Vec4::new(0.5, 0.5, 0.0, 0.0),
            unk4: Vec4::new(1.0, 1.0, 0.0, 1.0),
            unk5: Vec4::new(0.0, f32::NAN, 512.0, 0.0),
            unk6: Vec4::new(0.0, 1.0, 0.966_787_6, 0.0),
            unk7: Vec4::new(0.0, 0.5, 180.0, 0.0),
            unk8: Vec4::ZERO,
            unk9: Vec4::ZERO,
            unka: Vec4::new(f32::NAN, 0.0, 0.0, 0.0),
        }
    }
}

#[repr(C)]
#[derive(Clone, Default)]
pub struct ScopeInstances {
    pub mesh_offset: Vec3, // c0
    pub mesh_scale: f32,
    pub uv_scale: f32, // c1
    pub uv_offset: Vec2,
    pub max_color_index: u32,

    pub transforms: Vec<Mat4>, // c2-c5
}

impl ScopeInstances {
    pub fn write(&self) -> Vec<u8> {
        let mut buffer = vec![];

        buffer
            .write_all(bytemuck::cast_slice(&[
                self.mesh_offset.x,
                self.mesh_offset.y,
                self.mesh_offset.z,
                self.mesh_scale,
                self.uv_scale,
                self.uv_offset.x,
                self.uv_offset.y,
                f32::from_bits(self.max_color_index),
            ]))
            .unwrap();

        buffer
            .write_all(bytemuck::cast_slice(&self.transforms))
            .unwrap();

        buffer
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ScopeTransparentAdvanced {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Vec4,
    pub unk6: Vec4,
    pub unk7: Vec4,
    pub unk8: Vec4,
    pub unk9: Vec4,
    pub unk10: Vec4,
    pub unk11: Vec4,
    pub unk12: Vec4,
    pub unk13: Vec4,
    pub unk14: Vec4,
    pub unk15: Vec4,
    pub unk16: Vec4,
    pub unk17: Vec4,
    pub unk18: Vec4,
    pub unk19: Vec4,
    pub unk20: Vec4,
    pub unk21: Vec4,
    pub unk22: Vec4,
    pub unk23: Vec4,
    pub unk24: Vec4,
    pub unk25: Vec4,
    pub unk26: Vec4,
    pub unk27: Vec4,
    pub unk28: Vec4,
    pub unk29: Vec4,
    pub unk30: Vec4,
    pub unk31: Vec4,
    pub unk32: Vec4,
    pub unk33: Vec4,
    pub unk34: Vec4,
    pub unk35: Vec4,
    pub unk36: Vec4,
}

impl Default for ScopeTransparentAdvanced {
    fn default() -> Self {
        ScopeTransparentAdvanced {
            unk0: Vec4::new(
                0.000_984_931_4,
                0.001_983_686_8,
                0.000_778_356_7,
                0.001_558_671_2,
            ),
            unk1: Vec4::new(
                0.000_986_04,
                0.002_085_914,
                0.000_983_823_9,
                0.001_886_469_8,
            ),
            unk2: Vec4::new(
                0.001_186_082_4,
                0.002_434_628_8,
                0.000_946_840_8,
                0.001_850_187,
            ),
            unk3: Vec4::new(0.790_346_6, 0.731_906_4, 0.562_136_95, 0.0),
            unk4: Vec4::new(0.0, 1.0, 0.109375, 0.046875),
            unk5: Vec4::new(0.0, 0.0, 0.0, 0.000_869_452_95),
            unk6: Vec4::new(0.55, 0.410_910_52, 0.226_709_46, 0.503_812_73),
            unk7: Vec4::new(1.0, 1.0, 1.0, 0.999_777_8),
            unk8: Vec4::new(132.928_85, 66.404_44, 56.853_416, 0.0),
            unk9: Vec4::new(132.928_85, 66.404_44, 1000.0, 1e-4),
            unk10: Vec4::new(131.928_85, 65.404_44, 55.853_416, 0.678_431_4),
            unk11: Vec4::new(131.928_85, 65.404_44, 999.0, 5.5),
            unk12: Vec4::new(0.0, 0.5, 25.575_994, 0.0),
            unk13: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk14: Vec4::new(0.025, 10000.0, -9999.0, 1.0),
            unk15: Vec4::new(1.0, 1.0, 1.0, 0.0),
            unk16: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk17: Vec4::new(10.979_255, 7.148_235_3, 6.303_493_5, 0.0),
            unk18: Vec4::new(0.003_761_407_2, 0.0, 0.0, 0.0),
            unk19: Vec4::new(0.0, 0.007_529_612_6, 0.0, 0.0),
            unk20: Vec4::new(0.0, 0.0, 0.017_589_089, 0.0),
            unk21: Vec4::new(0.272_664_84, -0.314_738_18, -0.156_036_81, 1.0),
            unk22: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk23: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk24: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk25: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk26: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk27: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk28: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk29: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk30: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk31: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk32: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk33: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk34: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk35: Vec4::new(0.0, 0.0, 0.0, 0.0),
            unk36: Vec4::new(1.0, 0.0, 0.0, 0.0),
        }
    }
}

#[repr(C)]
pub struct ScopeSkinning {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub offset_scale: Vec4, // XYZ = offset, W = scale
    pub texcoord0_scale_offset: Vec4,
    pub dynamic_sh_ao_values: Vec4,

    pub nodes: [Vec4; 16],
}

impl Default for ScopeSkinning {
    fn default() -> Self {
        Self {
            unk0: Vec4::W,
            unk1: Vec4::W,
            unk2: Vec4::W,
            unk3: Vec4::W,
            unk4: Vec4::W,
            offset_scale: Vec4::W,
            texcoord0_scale_offset: Vec4::W,
            dynamic_sh_ao_values: Vec4::W,
            nodes: [Vec4::ONE; 16],
        }
    }
}
