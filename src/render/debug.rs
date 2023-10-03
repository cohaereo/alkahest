use std::sync::Arc;

use super::{color::Color, drawcall::ShaderStages, shader, ConstantBuffer, DeviceContextSwapchain};
use crate::types::AABB;
use anyhow::Context;
use genmesh::generators::IndexedPolygon;
use genmesh::generators::SharedVertex;
use genmesh::Triangulate;
use glam::{Mat4, Quat, Vec3};
use windows::Win32::Graphics::{
    Direct3D::{D3D11_PRIMITIVE_TOPOLOGY_LINELIST, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST},
    Direct3D11::{
        ID3D11Buffer, ID3D11InputLayout, ID3D11PixelShader, ID3D11VertexShader,
        D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
        D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_SUBRESOURCE_DATA,
        D3D11_USAGE_IMMUTABLE,
    },
    Dxgi::Common::{DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32G32B32A32_FLOAT},
};

#[derive(Debug, Copy, Clone)]
pub enum DebugShape {
    Cube {
        cube: AABB,
        rotation: Quat,
        sides: bool,
    },
    // Line {
    //     start: Vec3,
    //     end: Vec3,
    // },
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

    // pub fn line_2point<C: Into<Color>>(&mut self, start: Vec3, end: Vec3, color: C) {
    //     self.shapes
    //         .push((DebugShape::Line { start, end }, color.into()))
    // }

    // pub fn line_orientation<C: Into<Color>>(
    //     &mut self,
    //     point: Vec3,
    //     orientation: Quat,
    //     length: f32,
    //     color: C,
    // ) {
    //     self.shapes.push((
    //         DebugShape::Line {
    //             start: point,
    //             end: point + (orientation * Vec3::Y) * length,
    //         },
    //         color.into(),
    //     ))
    // }

    /// Returns the drawlist. The internal list is cleared after this call
    pub fn shape_list(&mut self) -> Vec<(DebugShape, Color)> {
        let v = self.shapes.clone();
        self.shapes.clear();

        v
    }
}

// TODO(cohae): We can improve performance by instancing each type of shape using instance buffers
pub struct DebugShapeRenderer {
    dcs: Arc<DeviceContextSwapchain>,
    scope: ConstantBuffer<ScopeAlkDebugShape>,
    vshader: ID3D11VertexShader,
    pshader: ID3D11PixelShader,

    input_layout: ID3D11InputLayout,
    vb_cube: ID3D11Buffer,
    ib_cube: ID3D11Buffer,
    ib_cube_sides: ID3D11Buffer,
    cube_outline_index_count: u32,
    cube_index_count: u32,
}

impl DebugShapeRenderer {
    pub fn new(dcs: Arc<DeviceContextSwapchain>) -> anyhow::Result<Self> {
        let data = shader::compile_hlsl(
            include_str!("../../assets/shaders/debug.hlsl"),
            "VShader",
            "vs_5_0",
        )
        .unwrap();
        let (vshader, _) = shader::load_vshader(&dcs, &data)?;

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
        let (pshader, _) = shader::load_pshader(&dcs, &data)?;

        let mesh = genmesh::generators::Cube::new();
        let vertices: Vec<[f32; 4]> = mesh
            .shared_vertex_iter()
            .map(|v| {
                let v = <[f32; 3]>::from(v.pos);
                [v[0], v[1], v[2], 1.0]
            })
            .collect();
        let mut indices = vec![];
        let mut indices_outline = vec![];
        for i in mesh.indexed_polygon_iter().triangulate() {
            indices.extend_from_slice(&[i.x as u16, i.y as u16, i.z as u16]);
        }

        for i in mesh.indexed_polygon_iter() {
            indices_outline.extend_from_slice(&[
                i.x as u16, i.y as u16, i.y as u16, i.z as u16, i.z as u16, i.w as u16, i.w as u16,
                i.x as u16,
            ]);
        }

        let ib_cube = unsafe {
            dcs.device
                .CreateBuffer(
                    &D3D11_BUFFER_DESC {
                        ByteWidth: (indices_outline.len() * 2) as _,
                        Usage: D3D11_USAGE_IMMUTABLE,
                        BindFlags: D3D11_BIND_INDEX_BUFFER,
                        ..Default::default()
                    },
                    Some(&D3D11_SUBRESOURCE_DATA {
                        pSysMem: indices_outline.as_ptr() as _,
                        ..Default::default()
                    }),
                )
                .context("Failed to create index buffer")?
        };

        let ib_cube_sides = unsafe {
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
            ib_cube_sides,
            cube_index_count: indices.len() as _,
            cube_outline_index_count: indices_outline.len() as _,
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
                        self.dcs.context().IASetInputLayout(&self.input_layout);
                        self.dcs.context().VSSetShader(&self.vshader, None);
                        self.dcs.context().PSSetShader(&self.pshader, None);

                        self.dcs.context().IASetVertexBuffers(
                            0,
                            1,
                            Some([Some(self.vb_cube.clone())].as_ptr()),
                            Some([16].as_ptr()),
                            Some(&0),
                        );

                        self.dcs.context().IASetIndexBuffer(
                            Some(&self.ib_cube),
                            DXGI_FORMAT_R16_UINT,
                            0,
                        );

                        self.dcs
                            .context()
                            .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_LINELIST);

                        self.dcs
                            .context()
                            .DrawIndexed(self.cube_outline_index_count as _, 0, 0);
                    }

                    if sides {
                        self.scope
                            .write(&ScopeAlkDebugShape {
                                model: Mat4::from_scale_rotation_translation(
                                    cube.dimensions(),
                                    rotation,
                                    cube.center(),
                                ),
                                color: Color(color.0.truncate().extend(0.25)),
                            })
                            .unwrap();

                        unsafe {
                            self.dcs.context().IASetVertexBuffers(
                                0,
                                1,
                                Some([Some(self.vb_cube.clone())].as_ptr()),
                                Some([16].as_ptr()),
                                Some(&0),
                            );

                            self.dcs.context().IASetIndexBuffer(
                                Some(&self.ib_cube_sides),
                                DXGI_FORMAT_R16_UINT,
                                0,
                            );

                            self.dcs
                                .context()
                                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

                            self.dcs.context().DrawIndexed(self.cube_index_count, 0, 0);
                        }
                    }
                } // DebugShape::Line { .. } => todo!(),
            }
        }
    }
}

pub struct ScopeAlkDebugShape {
    pub model: Mat4,
    pub color: Color,
}
