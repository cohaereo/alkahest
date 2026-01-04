use std::mem::size_of;

use alkahest_render::{gpu::command_list::CommandList, Gpu};
use d3d11::{dxgi, input_layout::ElementOffset};
use egui::{epaint::Primitive, Context};

use crate::{
    mesh::{create_index_buffer, create_vertex_buffer, GpuMesh, GpuVertex},
    shader::CompiledShaders,
    texture::TextureAllocator,
    RenderError,
};

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook or before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`.
pub struct D3D11Renderer {
    render_view: Option<d3d11::RenderTargetView>,
    tex_alloc: TextureAllocator,
    input_layout: d3d11::InputLayout,
    shaders: CompiledShaders,
    // backup: BackupState,
    // hwnd: HWND,
    samplers: [d3d11::SamplerState; 2],
    blend_state: d3d11::BlendState,
    raster_state: d3d11::RasterizerState,
}

// impl DirectX11Renderer {
//     const INPUT_ELEMENTS_DESC: [d3d11::InputElementDesc; 3] = [
//         d3d11::InputElementDesc::builder()
//             .semantic_name("POSITION")
//             .semantic_index(0)
//             .format(dxgi::Format::R32g32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Absolute(0))
//             .input_slot_class(d3d11::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//         d3d11::InputElementDesc::builder()
//             .semantic_name("TEXCOORD")
//             .semantic_index(0)
//             .format(dxgi::Format::R32g32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Append)
//             .input_slot_class(d3d11::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//         d3d11::InputElementDesc::builder()
//             .semantic_name("COLOR")
//             .semantic_index(0)
//             .format(dxgi::Format::R32g32b32a32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Append)
//             .input_slot_class(d3d11::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//     ];
// }

impl D3D11Renderer {
    /// Create a new directx11 renderer from a swapchain
    pub fn new(gpu: &Gpu) -> Result<Self, RenderError> {
        let backbuffer = gpu.swapchain.lock().get_buffer();

        let render_view = gpu.create_render_target_view(&backbuffer, None)?;

        let shaders = CompiledShaders::new(gpu)?;
        let input_layout = gpu.create_input_layout(
            &[
                d3d11::InputElementDesc::builder()
                    .semantic_name("POSITION")
                    .semantic_index(0)
                    .format(dxgi::Format::R32g32Float)
                    .input_slot(0)
                    .aligned_byte_offset(ElementOffset::Absolute(0))
                    .instance_data_step_rate(0)
                    .build(),
                d3d11::InputElementDesc::builder()
                    .semantic_name("TEXCOORD")
                    .semantic_index(0)
                    .format(dxgi::Format::R32g32Float)
                    .input_slot(0)
                    .aligned_byte_offset(ElementOffset::Append)
                    .instance_data_step_rate(0)
                    .build(),
                d3d11::InputElementDesc::builder()
                    .semantic_name("COLOR")
                    .semantic_index(0)
                    .format(dxgi::Format::R32g32b32a32Float)
                    .input_slot(0)
                    .aligned_byte_offset(ElementOffset::Append)
                    .instance_data_step_rate(0)
                    .build(),
            ],
            shaders.vs_bytecode(),
        )?;

        let samplers = {
            let desc = d3d11::SamplerDesc::builder()
                .filter(d3d11::Filter::MinMagMipLinear)
                .address_u(d3d11::TextureAddress::Border)
                .address_v(d3d11::TextureAddress::Border)
                .address_w(d3d11::TextureAddress::Border)
                .border_color([1., 1., 1., 1.])
                .build();

            let sampler_linear = gpu.create_sampler_state(&desc)?;
            let sampler_point = gpu.create_sampler_state(&d3d11::SamplerDesc {
                filter: d3d11::Filter::MinMagMipPoint,
                ..desc
            })?;

            [sampler_linear, sampler_point]
        };

        let blend_desc = d3d11::BlendDesc::from_single_target(
            d3d11::RenderTargetBlendDesc::builder()
                .blend_enable(true)
                .src_blend(d3d11::Blend::SrcAlpha)
                .dest_blend(d3d11::Blend::InvSrcAlpha)
                .blend_op(d3d11::BlendOp::Add)
                .src_blend_alpha(d3d11::Blend::One)
                .dest_blend_alpha(d3d11::Blend::InvSrcAlpha)
                .blend_op_alpha(d3d11::BlendOp::Add)
                .render_target_write_mask(15)
                .build(),
        );
        let raster_desc = d3d11::RasterizerDesc::builder()
            .fill_mode(d3d11::FillMode::Solid)
            .cull_mode(d3d11::CullMode::None)
            .front_counter_clockwise(false)
            .depth_bias(0)
            .depth_bias_clamp(0.)
            .slope_scaled_depth_bias(0.)
            .depth_clip_enable(false)
            .scissor_enable(true)
            .multisample_enable(false)
            .antialiased_line_enable(false)
            .build();

        let blend_state = gpu.create_blend_state(&blend_desc)?;
        let raster_state = gpu.create_rasterizer_state(&raster_desc)?;

        Ok(Self {
            tex_alloc: TextureAllocator::default(),
            // backup: BackupState::default(),
            input_layout,
            render_view: Some(render_view),
            shaders,
            // hwnd,
            samplers,
            blend_state,
            raster_state,
        })
    }
}

impl D3D11Renderer {
    /// Present call. Should be called once per original present call, before or inside of hook.
    #[allow(invalid_reference_casting)]
    pub fn paint(
        &mut self,
        cmd: &mut CommandList,
        output: egui::FullOutput,
        context: &Context,
        // mut paint: PaintFn,
    ) -> Result<egui::FullOutput, RenderError>
// where
    //     PaintFn: FnMut(&mut Self, &Context),
    {
        // self.backup.save(ctx);
        let screen = cmd.gpu().swapchain_resolution();

        if !output.textures_delta.is_empty() {
            self.tex_alloc
                .process_deltas(cmd.gpu(), cmd, &output.textures_delta)?;
        }

        if output.shapes.is_empty() {
            // self.backup.restore(ctx);
            return Ok(output);
        }

        let primitives = context
            .tessellate(output.shapes.clone(), output.pixels_per_point)
            .into_iter()
            .filter_map(|prim| {
                if let Primitive::Mesh(mesh) = prim.primitive {
                    GpuMesh::from_mesh(
                        (screen.0 as f32, screen.1 as f32),
                        mesh,
                        prim.clip_rect,
                        output.pixels_per_point,
                    )
                } else {
                    panic!("Paint callbacks are not yet supported")
                }
            })
            .collect::<Vec<_>>();

        cmd.output_merger_set_blend_state(&self.blend_state, Some(&[0., 0., 0., 0.]), 0xffffffff);
        cmd.rasterizer_set_state(&self.raster_state);

        cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
            .width(screen.0 as f32)
            .height(screen.1 as f32)
            .build()]);
        cmd.output_merger_set_render_targets(&[self.render_view.as_ref()], None);
        #[allow(deprecated)]
        cmd.input_assembler_set_input_layout(&self.input_layout);
        #[allow(deprecated)]
        cmd.input_assembler_set_primitive_topology(d3d11::PrimitiveTopology::TriangleList);

        for mesh in primitives {
            let idx = create_index_buffer(cmd.gpu(), &mesh)?;
            let vtx = create_vertex_buffer(cmd.gpu(), &mesh)?;

            let texture = self.tex_alloc.get_by_id(mesh.texture_id);

            cmd.rasterizer_set_scissor_rects(&[d3d11::Rect {
                left: mesh.clip.left() as _,
                top: mesh.clip.top() as _,
                right: mesh.clip.right() as _,
                bottom: mesh.clip.bottom() as _,
            }]);

            let mut use_alpha = false;
            if let Some((texture, texture_filter, texture_uses_alpha)) = &texture {
                use_alpha = *texture_uses_alpha;
                self.set_sampler_state(cmd, texture_filter.unwrap_or(egui::TextureFilter::Linear))?;
                cmd.pixel_set_shader_resources(0, &[Some(texture)]);
            }

            cmd.input_assembler_set_vertex_buffers(
                0,
                &[Some(&vtx)],
                Some(&[size_of::<GpuVertex>() as _]),
                Some(&[0]),
            )?;
            cmd.input_assembler_set_index_buffer(&idx, dxgi::Format::R32Uint, 0);
            cmd.vertex_set_shader(&self.shaders.vertex);
            cmd.pixel_set_shader(if use_alpha {
                &self.shaders.pixel
            } else {
                &self.shaders.pixel_no_alpha
            });

            cmd.draw_indexed(mesh.indices.len() as _, 0, 0);

            if texture.is_some() {
                self.set_sampler_state(cmd, egui::TextureFilter::Linear)?;
            }
        }

        // self.backup.restore(ctx);
        self.textures_mut().clear_temporaries();

        Ok(output)
    }

    /// Call when resizing buffers.
    /// Do not call the original function before it, instead call it inside of the `original` closure.
    /// # Behavior
    /// In `origin` closure make sure to call the original `ResizeBuffers`.
    pub fn resize_buffers(
        &mut self,
        gpu: &Gpu,
        original: impl FnOnce() -> d3d11::Result<()>,
    ) -> Result<(), RenderError> {
        drop(self.render_view.take());
        let result = original();
        let backbuffer: d3d11::Texture2D = gpu.swapchain.lock().get_buffer();
        self.render_view = Some(gpu.create_render_target_view(&backbuffer, None)?);
        Ok(result?)
    }

    pub fn textures(&self) -> &TextureAllocator {
        &self.tex_alloc
    }

    pub fn textures_mut(&mut self) -> &mut TextureAllocator {
        &mut self.tex_alloc
    }
}

impl D3D11Renderer {
    fn set_sampler_state(
        &self,
        ctx: &d3d11::DeviceContext,
        filter: egui::TextureFilter,
    ) -> Result<(), RenderError> {
        ctx.pixel_set_samplers(
            0,
            &[Some(match filter {
                egui::TextureFilter::Nearest => &self.samplers[0],
                egui::TextureFilter::Linear => &self.samplers[1],
            })],
        );
        Ok(())
    }
}
