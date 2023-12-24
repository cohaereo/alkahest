use std::io::{Read, Seek, SeekFrom};

use anyhow::Context;
use binrw::{binread, BinReaderExt, Endian, VecArgs};
use glam::Vec3;

use crate::{
    index::IndexItem,
    section::{TagSection, TagSectionSignature},
    types::{
        compound_shape::{hkpStaticCompoundShape, hkpStaticCompoundShapeInstance},
        convex_vertices::{hkFourTransposedPoints, hkpConvexVerticesShape},
        hkArrayIndex, hkPointerIndex,
        unknown::{Unk81, Unk84},
    },
};

#[binread]
#[derive(Debug)]
pub struct UnkShapeArrayParent {
    pub shapes: hkArrayIndex,
}

#[binread]
#[derive(Debug)]
pub struct UnkShapeArrayEntry {
    pub shape: hkPointerIndex,
}

#[derive(Default, Clone)]
pub struct Shape {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<u16>,
}

impl Shape {
    pub fn combine(&mut self, other: &Self) {
        let offset = self.vertices.len();
        self.vertices.extend(other.vertices.iter().cloned());
        self.indices
            .extend(other.indices.iter().map(|i| i + offset as u16));
    }

    pub fn apply_transform(&mut self, transform: glam::Mat4) {
        for v in self.vertices.iter_mut() {
            *v = transform.transform_point3(*v);
        }
    }

    pub fn center(&self) -> Vec3 {
        let mut center = Vec3::ZERO;
        for v in self.vertices.iter() {
            center += *v;
        }
        center / self.vertices.len() as f32
    }
}

pub fn read_shape_collection(f: &mut (impl Read + Seek)) -> anyhow::Result<Vec<Shape>> {
    // Destiny's havok files have 16 bytes of padding (?) at the start
    if f.read_be::<u32>()? == 0 {
        f.seek(SeekFrom::Start(0x10))?;
    } else {
        f.seek(SeekFrom::Start(0x0))?;
    }

    let tag0: TagSection = f.read_be()?;
    anyhow::ensure!(
        tag0.signature == TagSectionSignature::Tag0,
        "First tag must be TAG0",
    );

    f.seek(SeekFrom::Start(tag0.offset))?;

    let mut data_offset = 0_u64;
    while f.stream_position()? < tag0.end() {
        match f.read_be::<TagSection>() {
            Ok(section) => {
                match section.signature {
                    TagSectionSignature::Index => {
                        f.seek(SeekFrom::Start(section.offset))?;
                        let item = f.read_be::<TagSection>()?;
                        let endian = if item.is_le {
                            Endian::Little
                        } else {
                            Endian::Big
                        };
                        f.seek(SeekFrom::Start(item.offset))?;

                        let mut items = vec![];
                        while f.stream_position()? < item.end() {
                            let mut it: IndexItem = f.read_type(endian)?;
                            it.offset += data_offset as u32;
                            items.push(it);
                        }

                        for item in items.iter().skip(1) {
                            if item.typ == 0x74 {
                                f.seek(SeekFrom::Start(item.offset as u64))?;
                                let shape_array_index: hkArrayIndex = f.read_type(endian)?;
                                let shape_array_item = items
                                    .get(shape_array_index as usize)
                                    .context("Shape array parent references invalid index")?;

                                let shape_indices: Vec<UnkShapeArrayEntry> = f.save_pos_seek(
                                    SeekFrom::Start(shape_array_item.offset as u64),
                                    |f| {
                                        f.read_type_args(
                                            endian,
                                            VecArgs {
                                                count: shape_array_item.count as _,
                                                inner: (),
                                            },
                                        )
                                        .context("Failed to read shape array")
                                    },
                                )?;

                                let mut shapes = vec![];
                                for s in shape_indices {
                                    let shape = read_shape(&items, f, s.shape, endian)?;
                                    shapes.push(shape);
                                }

                                return Ok(shapes);
                            }
                        }
                    }
                    // TagSectionSignature::SdkVersion => {
                    //     f.seek(SeekFrom::Start(section.offset))?;
                    //     let mut data = vec![0u8; section.size];
                    //     f.read_exact(&mut data)?;
                    //     let version = String::from_utf8_lossy(&data);
                    // }
                    TagSectionSignature::Data => {
                        data_offset = section.offset;
                    }
                    _ => {}
                }

                f.seek(SeekFrom::Start(section.offset + section.size as u64))?;
            }
            Err(e) => {
                anyhow::bail!("Failed to read section: {}", e);
            }
        }
    }

    Err(anyhow::anyhow!(
        "No shape collections found in the given havok file"
    ))
}

pub fn read_shape(
    items: &[IndexItem],
    f: &mut (impl Read + Seek),
    item: hkPointerIndex,
    endian: Endian,
) -> anyhow::Result<Shape> {
    let item = items
        .get(item as usize)
        .context("Shape references invalid index")?;

    match item.typ {
        // Some alternative kind of static compound shape?
        0x81 => {
            f.seek(SeekFrom::Start(item.offset as u64))?;

            let unk81: Unk81 = f.read_type(endian)?;

            let unk84_item = items
                .get(unk81.unk38 as usize)
                .context("unk81 references invalid index")?;

            let unk84: Vec<Unk84> =
                f.save_pos_seek(SeekFrom::Start(unk84_item.offset as u64), |f| {
                    f.read_type_args(
                        endian,
                        VecArgs {
                            count: unk84_item.count as _,
                            inner: (),
                        },
                    )
                    .context("Failed to read compound shape instances array")
                })?;

            let mut shape = Shape::default();
            for v in unk84 {
                let s = read_shape(items, f, v.shape, endian)?;
                shape.combine(&s);
            }

            Ok(shape)
        }
        0x88 => {
            f.seek(SeekFrom::Start(item.offset as u64))?;

            let convex_shape: hkpConvexVerticesShape = f.read_type(endian)?;

            let vertices_item = items
                .get(convex_shape.rotated_vertices as usize)
                .context("convex shape references invalid index")?;

            let vertices: Vec<hkFourTransposedPoints> =
                f.save_pos_seek(SeekFrom::Start(vertices_item.offset as u64), |f| {
                    f.read_type_args(
                        endian,
                        VecArgs {
                            count: vertices_item.count as _,
                            inner: (),
                        },
                    )
                    .context("Failed to read compound shape instances array")
                })?;

            let vertices_corrected = vertices
                .iter()
                .flat_map(|v| v.transpose())
                .collect::<Vec<_>>();

            let points_raw: Vec<Vec<f32>> = vertices_corrected
                .iter()
                .map(|v| v.to_array().to_vec())
                .collect();

            let hull = chull::ConvexHullWrapper::try_new(&points_raw, None)
                .context("Failed to create convex hull")?;

            let (vertices, indices) = hull.vertices_indices();
            let shape = Shape {
                vertices: vertices
                    .into_iter()
                    .map(|v| Vec3::new(v[0], v[1], v[2]))
                    .collect(),

                indices: indices.into_iter().map(|i| i as u16).collect(),
            };

            Ok(shape)
        }
        0xaf => {
            f.seek(SeekFrom::Start(item.offset as u64))?;

            let compound_shape: hkpStaticCompoundShape = f.read_type(endian)?;

            let instances_item = items
                .get(compound_shape.instances as usize)
                .context("compound shape references invalid index")?;

            let instances: Vec<hkpStaticCompoundShapeInstance> =
                f.save_pos_seek(SeekFrom::Start(instances_item.offset as u64), |f| {
                    f.read_type_args(
                        endian,
                        VecArgs {
                            count: instances_item.count as _,
                            inner: (),
                        },
                    )
                    .context("Failed to read compound shape instances array")
                })?;

            let mut shape = Shape::default();
            for instance in instances {
                let mut s = read_shape(items, f, instance.shape, endian)?;
                s.apply_transform(instance.transform.to_mat4());
                shape.combine(&s);
            }

            Ok(shape)
        }
        u => anyhow::bail!("read_shape: Unhandled shape type 0x{u:x}"),
    }
}

trait SeekSaveExt: Seek {
    fn save_pos<F, T>(&mut self, inner: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut Self) -> anyhow::Result<T>,
    {
        let pos = self.stream_position()?;

        let r = inner(self)?;

        self.seek(SeekFrom::Start(pos))?;

        Ok(r)
    }

    fn save_pos_seek<F, T>(&mut self, seek: SeekFrom, inner: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut Self) -> anyhow::Result<T>,
    {
        let pos = self.stream_position()?;

        self.seek(seek)?;
        let r = inner(self)?;

        self.seek(SeekFrom::Start(pos))?;

        Ok(r)
    }
}

impl<T: Read + Seek> SeekSaveExt for T {}
