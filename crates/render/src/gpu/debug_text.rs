use std::{borrow::Cow, sync::Arc};

use alkahest_data::tfx::{PrimitiveType, ShaderStage};
use d3d11::{
    BindFlags, Blend, BlendDesc, BlendOp, D3D11_SUBRESOURCE_DATA, InputElementDesc,
    RenderTargetBlendDesc, SamplerDesc, Texture2dDesc, dxgi,
};
use glam::{IVec2, Mat4, Vec2, Vec3, Vec4};

use super::{Gpu, cbuffer::ConstantBuffer, command_list::CommandList};
use crate::gpu_span;

#[repr(C)]
struct DebugTextConstants {
    world_view_projection: Mat4,
    world_matrix: Mat4,
}

pub struct DebugTextRenderer {
    cbuffer: ConstantBuffer<DebugTextConstants>,
    mesh: DebugTextMesh,

    shader_vs: d3d11::VertexShader,
    shader_ps: d3d11::PixelShader,
    input_layout: d3d11::InputLayout,

    _texture: d3d11::Texture2D,
    texture_view: d3d11::ShaderResourceView,
    sampler: d3d11::SamplerState,
    blend_state: d3d11::BlendState,
}

impl DebugTextRenderer {
    pub fn create(gpu: &Arc<Gpu>) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create(
            gpu,
            Some(&DebugTextConstants {
                world_view_projection: Mat4::from_cols(
                    Vec4::new(2.0, 0.0, 0.0, 0.0),
                    Vec4::new(0.0, -2.0, 0.0, 0.0),
                    Vec4::new(0.0, 0.0, 1.0, 0.0),
                    Vec4::new(-1.0, 1.0, 0.0, 1.0),
                ),
                world_matrix: Mat4::IDENTITY,
            }),
        )?;

        let vs_data = include_bytes!("../../builtin/shaders/debug_text.vs.cso");
        let ps_data = include_bytes!("../../builtin/shaders/debug_text.ps.cso");
        let vertex_shader = gpu.create_vertex_shader(vs_data)?;
        let pixel_shader = gpu.create_pixel_shader(ps_data)?;

        let input_layout = gpu.create_input_layout(
            &[
                InputElementDesc::builder()
                    .semantic_name("POSITION")
                    .semantic_index(0)
                    .format(dxgi::Format::R32g32b32Float)
                    .input_slot(0)
                    .input_slot_class(d3d11::InputClassification::PerVertexData)
                    .build(),
                InputElementDesc::builder()
                    .semantic_name("TEXCOORD")
                    .semantic_index(0)
                    .format(dxgi::Format::R32g32Float)
                    .input_slot(0)
                    .input_slot_class(d3d11::InputClassification::PerVertexData)
                    .build(),
                InputElementDesc::builder()
                    .semantic_name("COLOR")
                    .semantic_index(0)
                    .format(dxgi::Format::R8g8b8a8Unorm)
                    .input_slot(0)
                    .input_slot_class(d3d11::InputClassification::PerVertexData)
                    .build(),
            ],
            vs_data,
        )?;

        let texture = gpu.create_texture2d(
            &Texture2dDesc::builder()
                .width(3072)
                .height(32)
                .mip_levels(1)
                .format(dxgi::Format::A8Unorm)
                .array_size(1)
                .bind_flags(BindFlags::SHADER_RESOURCE)
                .build(),
            Some(&[D3D11_SUBRESOURCE_DATA {
                pSysMem: include_bytes!("../../builtin/fonts/debug_3072x32.bin").as_ptr() as _,
                SysMemPitch: 3072,
                SysMemSlicePitch: 0,
            }]),
        )?;

        let texture_view = gpu.create_shader_resource_view(&texture, None)?;

        let sampler = gpu.create_sampler_state(&SamplerDesc::builder().build())?;

        let blend_state = gpu.create_blend_state(&BlendDesc::from_single_target(
            RenderTargetBlendDesc::builder()
                .blend_enable(true)
                .src_blend(Blend::SrcAlpha)
                .dest_blend(Blend::InvSrcAlpha)
                .blend_op(BlendOp::Add)
                .src_blend_alpha(Blend::Zero)
                .dest_blend_alpha(Blend::One)
                .blend_op_alpha(BlendOp::Add)
                .render_target_write_mask(0xF)
                .build(),
        ))?;

        Ok(Self {
            cbuffer,
            mesh: DebugTextMesh::create(gpu)?,

            shader_vs: vertex_shader,
            shader_ps: pixel_shader,
            input_layout,

            _texture: texture,
            texture_view,
            sampler,
            blend_state,
        })
    }

    #[profiling::function]
    pub fn draw(&mut self, cmd: &mut CommandList) {
        gpu_span!();

        self.mesh.update(cmd);
        self.cbuffer.bind(cmd, ShaderStage::Vertex, 0);

        cmd.output_merger_set_blend_state(
            &self.blend_state,
            Some(&[1.0, 1.0, 1.0, 1.0]),
            0xFFFF_FFFF,
        );
        cmd.vertex_set_shader(&self.shader_vs);
        cmd.pixel_set_shader(&self.shader_ps);
        cmd.set_input_layout_custom(&self.input_layout);
        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(&self.mesh.vb)],
            Some(&[24u32]),
            Some(&[0u32]),
        )
        .unwrap();
        cmd.pixel_set_shader_resources(0, &[Some(&self.texture_view)]);
        cmd.pixel_set_samplers(0, &[Some(&self.sampler)]);
        cmd.input_assembler_set_index_buffer(&self.mesh.ib, dxgi::Format::R32Uint, 0);
        cmd.draw_indexed(self.mesh.index_count, 0, 0);
    }

    pub fn add_string<'a, S: Into<Cow<'a, str>>>(
        &mut self,
        text: S,
        position: IVec2,
        color: [u8; 4],
        align: DebugTextAlign,
    ) {
        self.mesh.add_string(text, position, color, align);
    }

    pub fn clear(&mut self) {
        self.mesh.strings.clear();
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DebugTextAlign {
    TopLeft = 0,
    TopRight = 1,
    BottomLeft = 2,
    BottomRight = 3,
}

impl DebugTextAlign {
    pub fn origin(self) -> IVec2 {
        match self {
            DebugTextAlign::TopLeft => IVec2::ZERO,
            DebugTextAlign::TopRight => IVec2::new(1, 0),
            DebugTextAlign::BottomLeft => IVec2::new(0, 1),
            DebugTextAlign::BottomRight => IVec2::ONE,
        }
    }
}

#[repr(C)]
#[derive(Clone)]
struct DebugTextVertex {
    position: Vec3,
    uv: Vec2,
    color: [u8; 4],
}

struct DebugString {
    text: String,
    position: IVec2,
    color: [u8; 4],
    align: DebugTextAlign,
}

pub struct DebugTextMesh {
    pub vb: d3d11::Buffer,
    pub ib: d3d11::Buffer,
    pub index_count: u32,
    strings: Vec<DebugString>,

    vertices: Vec<DebugTextVertex>,
    indices: Vec<u32>,
}

impl DebugTextMesh {
    const CHARACTER_COUNT: usize = 96;
    const UV_STEP: f32 = 1.0 / Self::CHARACTER_COUNT as f32;
    // TODO(cohae): Actually 46, but we're squeezing each character by 1.125. This value should just be 46, but without changing the current margins.
    const MAX_CHARACTERS_VERTICAL: usize = 41;
    const CRAMMING_FACTOR: f32 = 1.125;

    fn create(gpu: &Gpu) -> anyhow::Result<Self> {
        let vb = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(
                    (Self::CHARACTER_COUNT
                        * Self::MAX_CHARACTERS_VERTICAL
                        * 4
                        * std::mem::size_of::<DebugTextVertex>()) as u32,
                )
                .usage(d3d11::Usage::Dynamic)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .cpu_access_flags(d3d11::CpuAccessFlags::WRITE)
                .build(),
            None,
        )?;

        let ib = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(
                    (Self::CHARACTER_COUNT
                        * Self::MAX_CHARACTERS_VERTICAL
                        * 6
                        * std::mem::size_of::<u32>()) as u32,
                )
                .usage(d3d11::Usage::Dynamic)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .cpu_access_flags(d3d11::CpuAccessFlags::WRITE)
                .build(),
            None,
        )?;

        Ok(Self {
            vb,
            ib,
            index_count: 0,
            strings: vec![],
            vertices: vec![],
            indices: vec![],
        })
    }

    fn add_string<'a, S: Into<Cow<'a, str>>>(
        &mut self,
        text: S,
        position: IVec2,
        color: [u8; 4],
        align: DebugTextAlign,
    ) {
        self.strings.push(DebugString {
            text: text.into().into_owned(),
            position,
            color,
            align,
        });
    }

    #[profiling::function]
    fn update(&mut self, cmd: &mut CommandList) {
        let resolution = cmd.gpu().swapchain_resolution();
        let aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        let char_recip_vertical = 1. / Self::MAX_CHARACTERS_VERTICAL as f32;
        let char_recip_horizontal = 1. / (Self::MAX_CHARACTERS_VERTICAL as f32 * aspect_ratio * 2.);
        let characters_horizontal = (1. / char_recip_horizontal) as i32;
        let characters_vertical = ((1. / char_recip_vertical) * Self::CRAMMING_FACTOR) as i32;

        self.vertices.clear();
        let mut character_count = 0;
        for string in std::mem::take(&mut self.strings) {
            // Alignment point coordinates, in characters
            let align_origin =
                string.align.origin() * IVec2::new(characters_horizontal, characters_vertical);

            // Calculate the dimensions of the string in characters
            let string_dims =
                IVec2::new(string.text.len() as i32, string.text.lines().count() as i32);

            let text_origin = match string.align {
                DebugTextAlign::TopLeft => align_origin,
                DebugTextAlign::TopRight => align_origin - IVec2::new(string_dims.x, 0),
                DebugTextAlign::BottomLeft => align_origin - IVec2::new(0, string_dims.y),
                DebugTextAlign::BottomRight => align_origin - string_dims,
            };

            for (i, line) in string.text.lines().enumerate() {
                let pos = text_origin + string.position + IVec2::new(0, i as i32);
                for (ci, char) in line.chars().enumerate() {
                    if char.is_whitespace() {
                        continue;
                    }

                    let char_pos = pos + IVec2::new(ci as i32, 0);
                    let uv_origin = Self::get_atlas_uv_origin(char);

                    let base_position = Vec3::new(
                        char_pos.x as f32 * char_recip_horizontal,
                        char_pos.y as f32 * char_recip_vertical / Self::CRAMMING_FACTOR,
                        0.10,
                    );

                    self.vertices.extend_from_slice(&[
                        // Top left
                        DebugTextVertex {
                            position: base_position,
                            uv: Vec2::new(uv_origin.x, uv_origin.y),
                            color: string.color,
                        },
                        // Top right
                        DebugTextVertex {
                            position: base_position + Vec3::X * char_recip_horizontal * 2.,
                            uv: Vec2::new(uv_origin.x + Self::UV_STEP, uv_origin.y),
                            color: string.color,
                        },
                        // Bottom left
                        DebugTextVertex {
                            position: base_position + Vec3::Y * char_recip_vertical,
                            uv: Vec2::new(uv_origin.x, uv_origin.y + 1.),
                            color: string.color,
                        },
                        // Bottom right
                        DebugTextVertex {
                            position: base_position
                                + Vec3::Y * char_recip_vertical
                                + Vec3::X * char_recip_horizontal * 2.,
                            uv: Vec2::new(uv_origin.x + Self::UV_STEP, uv_origin.y + 1.),
                            color: string.color,
                        },
                    ]);

                    character_count += 1;
                }
            }
        }

        self.indices.clear();
        for i in 0..character_count as u32 {
            let offset = i * 4;
            self.indices.extend_from_slice(&[
                offset,
                offset + 1,
                offset + 3,
                offset,
                offset + 3,
                offset + 2,
            ]);
        }

        unsafe {
            profiling::scope!("DebugTextMesh::update (upload)");
            // let mut ptr = Default::default();
            // gpu
            //     .context
            //     .Map(&self.vb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut ptr))
            //     .unwrap();
            // std::ptr::copy_nonoverlapping(
            //     self.vertices.as_ptr(),
            //     ptr.pData as *mut DebugTextVertex,
            //     self.vertices.len(),
            // );
            // gpu.context.Unmap(&self.vb, 0);
            let ptr = cmd
                .map(&self.vb, 0, d3d11::MapType::WriteDiscard, false)
                .unwrap();

            std::ptr::copy_nonoverlapping(
                self.vertices.as_ptr(),
                ptr.data as *mut DebugTextVertex,
                self.vertices.len(),
            );

            let ptr = cmd
                .map(&self.ib, 0, d3d11::MapType::WriteDiscard, false)
                .unwrap();
            std::ptr::copy_nonoverlapping(
                self.indices.as_ptr(),
                ptr.data as *mut u32,
                self.indices.len(),
            );
        }

        self.index_count = self.indices.len() as u32;
    }

    fn get_atlas_uv_origin(c: char) -> Vec2 {
        const ASCII_START: char = '!';
        const ASCII_END: char = '~';
        // Sanity check to make sure we don't recurse infinitely when encountering an invalid character
        debug_assert!((ASCII_START..=ASCII_END).contains(&'?'));
        match c {
            ASCII_START..=ASCII_END => {
                let c = c as u32 - ASCII_START as u32;
                Vec2::new(c as f32 * Self::UV_STEP, 0.0)
            }
            _ => Self::get_atlas_uv_origin('?'),
        }
    }
}
