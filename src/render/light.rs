use std::sync::Arc;

use crate::map::{SLight, SShadowingLight};

use super::drawcall::ShaderStages;
use super::renderer::Renderer;
use super::{shader, DeviceContextSwapchain};
use anyhow::Context;
use genmesh::generators::IndexedPolygon;
use genmesh::generators::SharedVertex;
use genmesh::Triangulate;
use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11DepthStencilState, D3D11_COMPARISON_ALWAYS, D3D11_DEPTH_STENCILOP_DESC,
    D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ZERO, D3D11_STENCIL_OP_DECR,
    D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R16_UINT;
use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Buffer, ID3D11InputLayout, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::DXGI_FORMAT_R32G32B32A32_FLOAT,
};

pub struct LightRenderer {
    dcs: Arc<DeviceContextSwapchain>,

    depth_state: ID3D11DepthStencilState,

    input_layout: ID3D11InputLayout,
    vb_cube: ID3D11Buffer,
    ib_cube: ID3D11Buffer,
    cube_index_count: u32,
}

impl LightRenderer {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        let input_sig_vs = shader::compile_hlsl(
            "struct s_vs_in { float3 v0 : POSITION; };  float4 vs(s_vs_in input) : SV_POSITION { return float4(0, 0, 0, 0); }",
            "vs",
            "vs_5_0",
        )
        .unwrap();

        let input_layout = unsafe {
            dcs.device.CreateInputLayout(
                &[D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                }],
                &input_sig_vs,
            )
        }
        .unwrap();

        let mesh = genmesh::generators::Cube::new();
        let vertices: Vec<[f32; 4]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2], 1.0]
            })
            .collect();
        let mut indices = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        let ib_cube = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (indices.len() * 2) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_INDEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: indices.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .context("Failed to create index buffer")?
        };

        let vb_cube = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (vertices.len() * 16) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_VERTEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: vertices.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .context("Failed to create vertex buffer")?
        };

        let depth_state = unsafe {
            dcs.device
                .CreateDepthStencilState(&D3D11_DEPTH_STENCIL_DESC {
                    DepthEnable: false.into(),
                    DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
                    DepthFunc: D3D11_COMPARISON_ALWAYS,
                    StencilEnable: false.into(),
                    StencilReadMask: 0xff,
                    StencilWriteMask: 0xff,
                    FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                    BackFace: D3D11_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D11_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                        StencilPassOp: D3D11_STENCIL_OP_KEEP,
                        StencilFunc: D3D11_COMPARISON_ALWAYS,
                    },
                })
                .context("Failed to create light renderer depth state")?
        };

        Ok(Self {
            dcs,
            depth_state,
            input_layout,
            vb_cube,
            ib_cube,
            cube_index_count: indices.len() as _,
        })
    }

    pub fn draw_normal(&self, renderer: &Renderer, light: &SLight) {
        let render_data = renderer.render_data.data();

        if let Some(mat) = render_data.materials.get(&light.technique_shading) {
            mat.evaluate_bytecode(renderer, &render_data);
            if mat
                .bind(&self.dcs, &render_data, ShaderStages::SHADING)
                .is_err()
            {
                return;
            }
        } else {
            return;
        }

        self.draw(renderer)
    }

    pub fn draw_shadowing(&self, renderer: &Renderer, light: &SShadowingLight) {
        let render_data = renderer.render_data.data();

        if let Some(mat) = render_data.materials.get(&light.technique_shading) {
            mat.evaluate_bytecode(renderer, &render_data);
            if mat
                .bind(&self.dcs, &render_data, ShaderStages::SHADING)
                .is_err()
            {
                return;
            }
        } else {
            return;
        }

        self.draw(renderer)
    }

    fn draw(&self, renderer: &Renderer) {
        unsafe {
            self.dcs
                .context()
                .OMSetDepthStencilState(Some(&self.depth_state), 0);

            self.dcs.context().OMSetBlendState(
                &renderer.blend_state_additive,
                Some(&[1f32, 1., 1., 1.] as _),
                0xffffffff,
            );

            self.dcs.context().IASetInputLayout(&self.input_layout);
            self.dcs.context().IASetVertexBuffers(
                0,
                1,
                Some([Some(self.vb_cube.clone())].as_ptr()),
                Some([16].as_ptr()),
                Some(&0),
            );

            self.dcs
                .context()
                .IASetIndexBuffer(Some(&self.ib_cube), DXGI_FORMAT_R16_UINT, 0);

            self.dcs
                .context()
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            self.dcs.context().DrawIndexed(self.cube_index_count, 0, 0);
        }
    }
}
