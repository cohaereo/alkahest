// Contains the error mesh used when an object fails to render for whatever reason

use std::{io::Cursor, rc::Rc};

use glam::Mat4;
use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*};

use crate::{dxgi::DxgiFormat, render::shader, texture::Texture};

use super::{ConstantBuffer, DeviceContextSwapchain};

#[allow(unused)]
pub struct ErrorRenderer {
    vertex_buffer: ID3D11Buffer,
    vertex_count: usize,
    vertex_layout: ID3D11InputLayout,

    texture: Texture,
    vshader: ID3D11VertexShader,
    pshader: ID3D11PixelShader,

    scope: ConstantBuffer<AlkScopeError>,
}

impl ErrorRenderer {
    pub fn load(dcs: Rc<DeviceContextSwapchain>) -> Self {
        const MATCAP_DATA: &[u8] = include_bytes!("../../assets/textures/error.data");
        let matcap = Texture::load_2d_raw(
            &dcs,
            128,
            128,
            MATCAP_DATA,
            DxgiFormat::R8G8B8A8_UNORM,
            Some("Error matcap"),
        )
        .expect("Failed to load error texture");

        let obj_data = include_bytes!("../../assets/models/error.obj");
        let reader = Cursor::new(obj_data);
        let obj = obj::ObjData::load_buf(reader).unwrap();

        let mut vertices = vec![];
        for vb in &obj.objects[0].groups[0].polys {
            for p in &vb.0 {
                let vi = p.0;
                let vni = p.2.unwrap_or_default();

                vertices.push([
                    obj.position[vi][0],
                    obj.position[vi][1],
                    obj.position[vi][2],
                    obj.normal[vni][0],
                    obj.normal[vni][1],
                    obj.normal[vni][2],
                ]);
            }
        }
        let vertex_buffer = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (std::mem::size_of::<[f32; 6]>() * vertices.len()) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_VERTEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: vertices.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .expect("Failed to create error vertex buffer")
        };

        let vshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/error.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let pshader_blob = shader::compile_hlsl(
            include_str!("../../assets/shaders/error.hlsl"),
            "PShader",
            "ps_5_0",
        )
        .unwrap();

        let vshader = shader::load_vshader(&dcs, &vshader_blob).unwrap();
        let pshader = shader::load_pshader(&dcs, &pshader_blob).unwrap();

        let vertex_layout = unsafe {
            dcs.device
                .CreateInputLayout(
                    &[
                        D3D11_INPUT_ELEMENT_DESC {
                            SemanticName: s!("POSITION"),
                            SemanticIndex: 0,
                            Format: DXGI_FORMAT_R32G32B32_FLOAT,
                            InputSlot: 0,
                            AlignedByteOffset: 0,
                            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                            InstanceDataStepRate: 0,
                        },
                        D3D11_INPUT_ELEMENT_DESC {
                            SemanticName: s!("NORMAL"),
                            SemanticIndex: 0,
                            Format: DXGI_FORMAT_R32G32B32_FLOAT,
                            InputSlot: 0,
                            AlignedByteOffset: 12,
                            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                            InstanceDataStepRate: 0,
                        },
                    ],
                    &vshader_blob,
                )
                .expect("Failed to create error vertex layout")
        };

        Self {
            vertex_buffer,
            vertex_count: vertices.len(),
            vertex_layout,
            texture: matcap,
            vshader,
            pshader,
            scope: ConstantBuffer::create(dcs, None).unwrap(),
        }
    }

    // TODO(cohae): Get this to work with the new renderer
    // pub fn draw(
    //     &self,
    //     renderer: &mut Renderer,
    //     transform: ID3D11Buffer,
    //     proj_view: Mat4,
    //     view: Mat4,
    // ) {
    //     self.scope
    //         .write(&AlkScopeError {
    //             proj_view,
    //             view,
    //             model: transform,
    //         })
    //         .unwrap();

    //     unsafe {
    //         dcs.context.IASetVertexBuffers(
    //             0,
    //             1,
    //             Some([Some(self.vertex_buffer.clone())].as_ptr()),
    //             Some([6 * 4].as_ptr()),
    //             Some(&0),
    //         );

    //         dcs.context
    //             .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

    //         dcs.context
    //             .VSSetConstantBuffers(7, Some(&[Some(self.scope.buffer().clone())]));

    //         dcs.context.IASetInputLayout(&self.vertex_layout);
    //         dcs.context.VSSetShader(&self.vshader, None);

    //         dcs.context.PSSetShader(&self.pshader, None);

    // TODO(cohae): use new texture bind API
    //         dcs.context
    //             .PSSetShaderResources(0, Some(&[Some(self.texture.view.clone())]));

    //         dcs.context.Draw(self.vertex_count as u32, 0);
    //     }
    // }
}

#[allow(unused)]
#[repr(C)]
struct AlkScopeError {
    pub proj_view: Mat4,
    pub view: Mat4,
    pub model: Mat4,
}
