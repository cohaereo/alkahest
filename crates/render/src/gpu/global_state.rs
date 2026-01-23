use std::fmt::Write;

use alkahest_data::tfx::render_globals::SRenderGlobals;
use anyhow::Context;
use d3d11::{DeviceChild, InputElementDesc, dxgi, sys::Foundation::BOOL};
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

use crate::util::byteutil;

const DEPTH_STENCIL_COMBO_COUNT: usize = DEPTH_STENCIL_COMBOS.len();
pub struct RenderStates {
    pub input_layouts: Vec<Option<d3d11::InputLayout>>,
    pub blend_states: Vec<d3d11::BlendState>,
    pub rasterizer_states: [[d3d11::RasterizerState; 9]; 9],
    /// 1st state is for reverse Z, 2nd state is for regular Z (commonly used for shadow maps)
    pub depth_stencil_states:
        [(d3d11::DepthStencilState, d3d11::DepthStencilState); DEPTH_STENCIL_COMBO_COUNT],
}

impl RenderStates {
    const BLEND_STATE_DESC_DATA: &[u8] = include_bytes!("../../builtin/gpu/blend_states.bin");
    const RASTERIZER_STATE_DATA: &[u8] = include_bytes!("../../builtin/gpu/rasterizer_states.bin");
    const DEPTH_BIAS_DATA: &[u8] = include_bytes!("../../builtin/gpu/depth_biases.bin");

    const DEPTH_STATE_DATA: &[u8] = include_bytes!("../../builtin/gpu/depth_states.bin");
    const STENCIL_STATE_DATA: &[u8] = include_bytes!("../../builtin/gpu/stencil_states.bin");

    const REVERSED_DEPTH_FUNCS: [d3d11::ComparisonFunc; 13] = [
        d3d11::ComparisonFunc::Always,
        d3d11::ComparisonFunc::Always,
        d3d11::ComparisonFunc::LessEqual,
        d3d11::ComparisonFunc::LessEqual,
        d3d11::ComparisonFunc::GreaterEqual,
        d3d11::ComparisonFunc::GreaterEqual,
        d3d11::ComparisonFunc::Greater,
        d3d11::ComparisonFunc::LessEqual,
        d3d11::ComparisonFunc::LessEqual,
        d3d11::ComparisonFunc::Always,
        d3d11::ComparisonFunc::Never,
        d3d11::ComparisonFunc::Always,
        d3d11::ComparisonFunc::LessEqual,
    ];

    pub fn new(device: &d3d11::Device) -> anyhow::Result<Self> {
        let mut input_layouts = vec![None; 255];
        for (i, layout) in BASE_INPUT_LAYOUTS.iter().enumerate() {
            let layout = Self::create_input_layout(device, layout.elements)?;
            layout.set_debug_name(format!("Built-in Input Layout {i}"));
            input_layouts[i] = Some(layout);
        }

        let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
        let globs = &data.unk8.first().context("No render globals found")?.unk8.0;

        let element_set = &globs.input_layouts.elements_c;
        for l in globs.input_layouts.mapping.layouts.iter() {
            let mut layout_elements = vec![];
            for (buffer_index, &(element_index, is_instance_data)) in [
                (l.buffer_0, l.buffer_0_instanced),
                (l.buffer_1, l.buffer_1_instanced),
                (l.buffer_2, l.buffer_2_instanced),
                (l.buffer_3, l.buffer_3_instanced),
            ]
            .iter()
            .enumerate()
            {
                if element_index == u32::MAX {
                    continue;
                }
                for e in &element_set.sets[element_index as usize].elements {
                    let semantic = INPUT_SEMANTICS[e.semantic as usize];
                    let format = &INPUT_FORMATS[e.format as usize];
                    layout_elements.push(TigerInputLayoutElement {
                        hlsl_type: format.hlsl_type,
                        format: format.format,
                        _stride: format.stride,
                        semantic_name: semantic,
                        semantic_index: e.semantic_index as _,
                        buffer_index: buffer_index as _,
                        is_instance_data,
                    });
                }
            }

            // println!("Layout {}", l.index);
            // for (ei, element) in layout_elements.iter().enumerate() {
            //     println!(
            //         " - {} v{ei} : {}{}, // Format {:?} size {}{}",
            //         element.hlsl_type,
            //         element.semantic_name,
            //         element.semantic_index,
            //         element.format,
            //         element._stride,
            //         if element.is_instance_data {
            //             " (instanced)"
            //         } else {
            //             ""
            //         }
            //     );
            // }

            let layout = Self::create_input_layout(device, &layout_elements)?;
            layout.set_debug_name(format!("stream_input_layout_{}", l.index));
            input_layouts[l.index as usize] = Some(layout);
        }

        let mut blend_states = vec![];

        let blend_state_descs: &[QuadBlendState] =
            unsafe { byteutil::bytes_as_slice(Self::BLEND_STATE_DESC_DATA) };
        // let chroma = chroma_dbg::ChromaConfig {
        //     inline_struct: chroma_dbg::InlineThreshold::Always,
        //     ..Default::default()
        // };
        for desc in blend_state_descs.iter() {
            // println!("{}", chroma.format(desc));

            let render_targets = [
                desc.render_targets[0].clone(),
                desc.render_targets[1].clone(),
                desc.render_targets[2].clone(),
                desc.render_targets[3].clone(),
                // Extend to 8 render targets
                desc.render_targets[3].clone(),
                desc.render_targets[3].clone(),
                desc.render_targets[3].clone(),
                desc.render_targets[3].clone(),
            ];

            let state = device.create_blend_state(
                &d3d11::BlendDesc::builder()
                    .alpha_to_coverage_enable(false)
                    .independent_blend_enable(true)
                    .render_target(render_targets)
                    .build(),
            )?;
            blend_states.push(state);
        }

        let depth_biases: &[ShortDepthBias] =
            unsafe { byteutil::bytes_as_slice(Self::DEPTH_BIAS_DATA) };
        let rasterizer_states: &[PaddedRasterizerState] =
            unsafe { byteutil::bytes_as_slice(Self::RASTERIZER_STATE_DATA) };
        assert_eq!(rasterizer_states.len(), 9);
        assert_eq!(depth_biases.len(), 9);

        let rasterizer_states = std::array::try_from_fn(|bi| {
            std::array::try_from_fn(|ri| {
                let mut desc = rasterizer_states[ri].desc.clone();
                desc.multisample_enable = false.into();
                let ShortDepthBias(_, depth_bias, slope_scaled_depth_bias, depth_bias_clamp) =
                    depth_biases[bi];

                desc.depth_bias = depth_bias;
                desc.slope_scaled_depth_bias = slope_scaled_depth_bias;
                desc.depth_bias_clamp = depth_bias_clamp;

                device.create_rasterizer_state(&desc)
            })
        })
        .context("Failed to create rasterizer states")?;

        let depth_states: &[ShortDepthState] =
            unsafe { byteutil::bytes_as_slice(Self::DEPTH_STATE_DATA) };
        let stencil_states: &[ShortStencilDesc] =
            unsafe { byteutil::bytes_as_slice(Self::STENCIL_STATE_DATA) };

        warn!("!!! STENCIL TESTING IS DISABLED");
        let depth_stencil_states: anyhow::Result<_> = std::array::try_from_fn(|i| {
            let (depth_index, stencil_index) = DEPTH_STENCIL_COMBOS[i];

            let depth = depth_states[depth_index].clone();
            let stencil = stencil_states[stencil_index].clone();
            let reversed_depth_func = Self::REVERSED_DEPTH_FUNCS[depth_index].clone();

            let mut desc = d3d11::DepthStencilDesc {
                depth_enable: BOOL(depth.0),
                depth_write_mask: depth.1,
                depth_func: depth.2,
                stencil_enable: false.into(), //stencil.enabled,
                stencil_read_mask: stencil.read_mask,
                stencil_write_mask: stencil.write_mask,
                front_face: d3d11::DepthStencilOpDesc {
                    stencil_fail_op: stencil.front_face.2,
                    stencil_depth_fail_op: stencil.front_face.3,
                    stencil_pass_op: stencil.front_face.1,
                    stencil_func: stencil.front_face.0,
                },
                back_face: d3d11::DepthStencilOpDesc {
                    stencil_fail_op: stencil.back_face.2,
                    stencil_depth_fail_op: stencil.back_face.3,
                    stencil_pass_op: stencil.back_face.1,
                    stencil_func: stencil.back_face.0,
                },
            };

            let state = device.create_depth_stencil_state(&desc)?;

            desc.depth_func = reversed_depth_func;
            let state_reversed = device.create_depth_stencil_state(&desc)?;

            Ok((state, state_reversed))
        });

        Ok(Self {
            input_layouts,
            blend_states,
            rasterizer_states,
            depth_stencil_states: depth_stencil_states
                .context("Failed to create depth stencil states")?,
        })
    }

    fn create_input_layout(
        device: &d3d11::Device,
        elements: &[TigerInputLayoutElement],
    ) -> anyhow::Result<d3d11::InputLayout> {
        let mut shader_input = "struct s_vs_in { ".to_string();
        let mut elements_dx11 = vec![];

        for (i, e) in elements.iter().enumerate() {
            write!(
                &mut shader_input,
                "{} v{i} : {}{}; ",
                e.hlsl_type, e.semantic_name, e.semantic_index
            )?;

            elements_dx11.push(
                InputElementDesc::builder()
                    .semantic_name(e.semantic_name)
                    .semantic_index(e.semantic_index)
                    .format(e.format)
                    .input_slot(e.buffer_index)
                    .input_slot_class(if e.is_instance_data {
                        d3d11::InputClassification::PerInstanceData
                    } else {
                        d3d11::InputClassification::PerVertexData
                    })
                    .instance_data_step_rate(e.is_instance_data as u32)
                    .build(),
            );
        }

        shader_input
            .push_str("};  float4 vs(s_vs_in input) : SV_POSITION { return float4(0, 0, 0, 0); }");

        let shader_blob = d3d11::fxc::compile(
            shader_input.as_bytes(),
            Some("create_vertex_declaration_inline"),
            &[],
            "vs",
            d3d11::fxc::ShaderTarget::Vertex,
        )
        .context("Failed to compile inline vertex declaration")?;

        let layout = device
            .create_input_layout(&elements_dx11, &shader_blob)
            .context("Failed to create input layout")?;

        Ok(layout)
    }
}

/// Regular d3d11 blend state, but with only 4 render targets
#[repr(C)]
#[derive(Debug)]
pub struct QuadBlendState {
    pub unk0: u32,
    pub render_targets: [d3d11::RenderTargetBlendDesc; 4],
}

#[repr(C)]
#[derive(Debug)]
pub struct PaddedRasterizerState {
    pub desc: d3d11::RasterizerDesc,
    _pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ShortDepthBias(u32, i32, f32, f32);

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ShortDepthState(i32, d3d11::DepthWriteMask, d3d11::ComparisonFunc, u32);

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ShortStencilDesc {
    pub unk0: u32,
    pub enabled: BOOL,
    pub read_mask: u8,
    pub write_mask: u8,
    _pad0: u16,
    pub front_face: (
        d3d11::ComparisonFunc,
        d3d11::StencilOp,
        d3d11::StencilOp,
        d3d11::StencilOp,
    ),
    pub back_face: (
        d3d11::ComparisonFunc,
        d3d11::StencilOp,
        d3d11::StencilOp,
        d3d11::StencilOp,
    ),
}

const DEPTH_STENCIL_COMBOS: [(usize, usize); 83] = [
    (0, 0),
    (1, 1),
    (2, 1),
    (7, 1),
    (2, 2),
    (1, 3),
    (1, 4),
    (2, 5),
    (2, 6),
    (2, 9),
    (2, 10),
    (2, 0xb),
    (2, 0xc),
    (4, 1),
    (5, 1),
    (3, 1),
    (6, 1),
    (3, 0x10),
    (8, 0x10),
    (3, 0x11),
    (3, 0x12),
    (6, 0x13),
    (6, 0x1b),
    (3, 0x13),
    (3, 0x19),
    (3, 0x1b),
    (5, 0x14),
    (2, 0x15),
    (3, 0x15),
    (3, 0x18),
    (3, 0x1a),
    (1, 0x1d),
    (1, 0x12),
    (1, 0x13),
    (9, 1),
    (10, 1),
    (3, 0x1e),
    (0xb, 0x1f),
    (1, 0x1f),
    (1, 0x20),
    (1, 0x21),
    (3, 0x21),
    (2, 0x21),
    (5, 0x20),
    (3, 0x20),
    (3, 6),
    (3, 10),
    (3, 0xb),
    (3, 0xc),
    (3, 9),
    (0xc, 0x22),
    (1, 0x23),
    (3, 0x1c),
    (6, 0x1c),
    (0xc, 0x10),
    (0xc, 0x25),
    (8, 0x24),
    (3, 0x26),
    (1, 0x26),
    (3, 0x27),
    (1, 0x27),
    (3, 0x14),
    (1, 0x14),
    (3, 0x28),
    (3, 8),
    (2, 8),
    (1, 2),
    (1, 8),
    (3, 7),
    (3, 0x17),
    (3, 0xd),
    (3, 0xe),
    (3, 0xf),
    (1, 0x29),
    (1, 0x2a),
    (1, 0x2b),
    (1, 0x2c),
    (1, 0x2d),
    (1, 0x2e),
    (1, 0x2f),
    (1, 0x30),
    (1, 0x1a),
    (2, 0x16),
];

struct TigerInputLayout {
    pub elements: &'static [TigerInputLayoutElement],
}

struct TigerInputLayoutElement {
    pub hlsl_type: &'static str,
    pub format: dxgi::Format,
    pub _stride: u32,
    pub semantic_name: &'static str,
    pub semantic_index: u32,
    pub buffer_index: u32,
    pub is_instance_data: bool,
}

struct InputElementFormat {
    hlsl_type: &'static str,
    stride: u32,
    format: dxgi::Format,
}

//region Built-in input layouts
const INPUT_SEMANTICS: [&str; 9] = [
    "POSITION",
    "BLENDWEIGHT",
    "BLENDINDICES",
    "NORMAL",
    "PSIZE",
    "TEXCOORD",
    "TANGENT",
    "BINORMAL",
    "COLOR",
];

#[rustfmt::skip]
const INPUT_FORMATS: [InputElementFormat; 34] = [
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 0
    InputElementFormat { hlsl_type: "float", stride: 4, format: dxgi::Format::R32Float }, // 1
    InputElementFormat { hlsl_type: "float2", stride: 8, format: dxgi::Format::R32g32Float }, // 2
    InputElementFormat { hlsl_type: "float3", stride: 12, format: dxgi::Format::R32g32b32Float }, // 3
    InputElementFormat { hlsl_type: "float4", stride: 16, format: dxgi::Format::R32g32b32a32Float }, // 4
    InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8Unorm }, // 5
    InputElementFormat { hlsl_type: "uint4", stride: 4, format: dxgi::Format::R8g8b8a8Uint }, // 6
    InputElementFormat { hlsl_type: "int2", stride: 4, format: dxgi::Format::R16g16Sint }, // 7
    InputElementFormat { hlsl_type: "int4", stride: 8, format: dxgi::Format::R16g16b16a16Sint }, // 8
    InputElementFormat { hlsl_type: "uint4", stride: 8, format: dxgi::Format::R16g16b16a16Uint }, // 9
    InputElementFormat { hlsl_type: "float2", stride: 4, format: dxgi::Format::R16g16Snorm }, // 10
    InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Snorm }, // 11
    InputElementFormat { hlsl_type: "float2", stride: 4, format: dxgi::Format::R16g16Float }, // 12
    InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Float }, // 13
    InputElementFormat { hlsl_type: "int4", stride: 4, format: dxgi::Format::R8g8b8a8Sint }, // 14
    InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8Snorm }, // 15
    InputElementFormat { hlsl_type: "uint4", stride: 4, format: dxgi::Format::R10g10b10a2Uint }, // 16
    InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R10g10b10a2Unorm }, // 17
    InputElementFormat { hlsl_type: "int", stride: 4, format: dxgi::Format::R32Sint }, // 18
    InputElementFormat { hlsl_type: "int2", stride: 8, format: dxgi::Format::R32g32Sint }, // 19
    InputElementFormat { hlsl_type: "int4", stride: 16, format: dxgi::Format::R32g32b32a32Sint }, // 20
    InputElementFormat { hlsl_type: "int", stride: 4, format: dxgi::Format::R32Uint }, // 21
    InputElementFormat { hlsl_type: "int2", stride: 8, format: dxgi::Format::R32g32Uint }, // 22
    InputElementFormat { hlsl_type: "int4", stride: 16, format: dxgi::Format::R32g32b32a32Uint }, // 23
    InputElementFormat { hlsl_type: "int", stride: 2, format: dxgi::Format::R16Sint }, // 24
    InputElementFormat { hlsl_type: "float", stride: 1, format: dxgi::Format::R8Unorm }, // 25
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 26
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 27
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 28
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 29
    InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 30
    InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8UnormSrgb }, // 31
    InputElementFormat { hlsl_type: "float3", stride: 4, format: dxgi::Format::R11g11b10Float }, // 32
    InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Snorm }, // 33
];

const BASE_INPUT_LAYOUTS: [TigerInputLayout; 7] = [
    // Layout 0
    TigerInputLayout {
        elements: &[TigerInputLayoutElement {
            hlsl_type: "float3",
            format: dxgi::Format::R32g32b32Float,
            _stride: 12,
            semantic_name: "POSITION",
            semantic_index: 0,
            buffer_index: 0,
            is_instance_data: false,
        }],
    },
    // Layout 1
    TigerInputLayout {
        elements: &[TigerInputLayoutElement {
            hlsl_type: "float3",
            format: dxgi::Format::R32g32b32Float,
            _stride: 12,
            semantic_name: "POSITION",
            semantic_index: 0,
            buffer_index: 0,
            is_instance_data: false,
        }],
    },
    // Layout 2
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                _stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 3
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                _stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                _stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 4
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                _stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                _stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 5
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 6
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                _stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                _stride: 12,
                semantic_name: "NORMAL",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R32g32b32a32Float,
                _stride: 16,
                semantic_name: "TANGENT",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                _stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
];
//endregion
