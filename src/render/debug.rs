use std::rc::Rc;

use anyhow::Context;
use glam::{Mat4, Quat, Vec3, Vec4};
use windows::{
    core::PCSTR,
    Win32::Graphics::{
        Direct3D::D3D11_PRIMITIVE_TOPOLOGY_LINELIST,
        Direct3D11::{
            ID3D11Buffer, ID3D11InputLayout, ID3D11PixelShader, ID3D11VertexShader,
            D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
            D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_SUBRESOURCE_DATA,
            D3D11_USAGE_IMMUTABLE,
        },
        Dxgi::Common::{DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32B32A32_FLOAT},
    },
};

use crate::types::AABB;

use super::{color::Color, drawcall::ShaderStages, shader, ConstantBuffer, DeviceContextSwapchain};

#[derive(Debug, Copy, Clone)]
pub enum DebugShape {
    Cube {
        cube: AABB,
        rotation: Quat,
        sides: bool,
    },
    Line {
        start: Vec3,
        end: Vec3,
    },
}

#[derive(Default)]
pub struct DebugShapes {
    shapes: Vec<(DebugShape, Color)>,
}

impl DebugShapes {
    pub fn cube_extents<C: Into<Color>>(
        &mut self,
        center: Vec3,
        extents: Vec3,
        rotation: Quat,
        color: C,
        sides: bool,
    ) {
        let min = center - extents;
        let max = center + extents;

        self.shapes.push((
            DebugShape::Cube {
                cube: AABB {
                    min: min.into(),
                    max: max.into(),
                },
                rotation,
                sides,
            },
            color.into(),
        ))
    }

    pub fn cube_aabb<C: Into<Color>>(&mut self, aabb: AABB, rotation: Quat, color: C, sides: bool) {
        self.shapes.push((
            DebugShape::Cube {
                cube: aabb,
                rotation,
                sides,
            },
            color.into(),
        ))
    }

    pub fn line_2point<C: Into<Color>>(&mut self, start: Vec3, end: Vec3, color: C) {
        self.shapes
            .push((DebugShape::Line { start, end }, color.into()))
    }

    pub fn line_orientation<C: Into<Color>>(
        &mut self,
        point: Vec3,
        orientation: Quat,
        length: f32,
        color: C,
    ) {
        self.shapes.push((
            DebugShape::Line {
                start: point,
                end: point + (orientation * Vec3::Y) * length,
            },
            color.into(),
        ))
    }

    /// Returns the drawlist. The internal list is cleared after this call
    pub fn shape_list(&mut self) -> Vec<(DebugShape, Color)> {
        let v = self.shapes.clone();
        self.shapes.clear();

        v
    }
}

// TODO(cohae): We can improve performance by instancing each type of shape using instance buffers
pub struct DebugShapeRenderer {
    dcs: Rc<DeviceContextSwapchain>,
    scope: ConstantBuffer<ScopeAlkDebugShape>,
    vshader: ID3D11VertexShader,
    pshader: ID3D11PixelShader,

    input_layout: ID3D11InputLayout,
    vb_cube: ID3D11Buffer,
    ib_cube: ID3D11Buffer,
    cube_index_count: u32,
}

impl DebugShapeRenderer {
    const VERTICES_CUBE: &[Vec4] = &[
        // Bottom
        Vec4::new(-0.5, -0.5, -0.5, 1.0), // 0 - Bottom left
        Vec4::new(-0.5, 0.5, -0.5, 1.0),  // 1 - Top left
        Vec4::new(0.5, 0.5, -0.5, 1.0),   // 2 - Top right
        Vec4::new(0.5, -0.5, -0.5, 1.0),  // 3 - Bottom right
        // Top
        Vec4::new(-0.5, -0.5, 0.5, 1.0), // 4 - Bottom left
        Vec4::new(-0.5, 0.5, 0.5, 1.0),  // 5 - Top left
        Vec4::new(0.5, 0.5, 0.5, 1.0),   // 6 - Top right
        Vec4::new(0.5, -0.5, 0.5, 1.0),  // 7 - Bottom right
    ];

    const INDICES_CUBE: &[u16] = &[
        0, 4, 1, 5, 2, 6, 3, 7, // Vertical
        0, 1, 1, 2, 2, 3, 3, 0, // Bottom horizontal
        4, 5, 5, 6, 6, 7, 7, 4, // Top horizontal
    ];

    pub fn new(dcs: Rc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        let data = shader::compile_hlsl(
            include_str!("../../assets/shaders/debug.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let vshader = shader::load_vshader(&dcs, &data)?;

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
                &data,
            )
        }
        .unwrap();

        let data = shader::compile_hlsl(
            include_str!("../../assets/shaders/debug.hlsl"),
            "PShader",
            "ps_5_0",
        )
        .unwrap();
        let pshader = shader::load_pshader(&dcs, &data)?;

        let ib_cube = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (Self::INDICES_CUBE.len() * 2) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_INDEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: Self::INDICES_CUBE.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .context("Failed to create index buffer")?
        };

        let vb_cube = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (Self::VERTICES_CUBE.len() * 16) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_VERTEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: Self::VERTICES_CUBE.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .context("Failed to create combined vertex buffer")?
        };

        Ok(Self {
            scope: ConstantBuffer::create(dcs.clone(), None)?,
            dcs,
            vshader,
            pshader,
            input_layout,
            vb_cube,
            ib_cube,
            cube_index_count: Self::INDICES_CUBE.len() as _,
        })
    }

    pub fn draw_all(&self, shapes: &mut DebugShapes) {
        for (shape, color) in shapes.shape_list() {
            match shape {
                DebugShape::Cube {
                    cube,
                    rotation,
                    sides,
                } => {
                    // TODO(cohae): Sides
                    self.scope
                        .write(&ScopeAlkDebugShape {
                            model: Mat4::from_scale_rotation_translation(
                                cube.dimensions(),
                                rotation,
                                cube.center(),
                            ),
                            color,
                        })
                        .unwrap();

                    self.scope.bind(1, ShaderStages::all());
                    unsafe {
                        self.dcs.context.IASetInputLayout(&self.input_layout);
                        self.dcs.context.VSSetShader(&self.vshader, None);
                        self.dcs.context.PSSetShader(&self.pshader, None);

                        self.dcs.context.IASetVertexBuffers(
                            0,
                            1,
                            Some([Some(self.vb_cube.clone())].as_ptr()),
                            Some([16].as_ptr()),
                            Some(&0),
                        );

                        self.dcs.context.IASetIndexBuffer(
                            Some(&self.ib_cube),
                            DXGI_FORMAT_R16_UINT,
                            0,
                        );

                        self.dcs
                            .context
                            .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_LINELIST);

                        self.dcs.context.DrawIndexed(self.cube_index_count, 0, 0);
                    }
                }
                DebugShape::Line { start, end } => todo!(),
            }
        }
    }
}

pub struct ScopeAlkDebugShape {
    pub model: Mat4,
    pub color: Color,
}
