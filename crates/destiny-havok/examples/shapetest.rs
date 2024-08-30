use std::{
    fs::File,
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    str::FromStr,
};

use binrw::{BinReaderExt, Endian};
use destiny_havok::{
    index::IndexItem,
    section::{TagSection, TagSectionSignature},
};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;

fn main() -> anyhow::Result<()> {
    let mut f = File::open(std::env::args().nth(1).unwrap())?;
    let path = PathBuf::from_str(&std::env::args().nth(1).unwrap())?;
    let filename = path.file_stem().unwrap().to_string_lossy().to_string();

    // Destiny's havok files have 16 bytes of padding (?) at the start
    if f.read_be::<u32>()? == 0 {
        f.seek(SeekFrom::Start(0x10))?;
    } else {
        f.seek(SeekFrom::Start(0x0))?;
    }

    let tag0: TagSection = f.read_be()?;
    println!("{tag0:#?}");
    assert_eq!(
        tag0.signature,
        TagSectionSignature::Tag0,
        "First tag must be TAG0"
    );

    f.seek(SeekFrom::Start(tag0.offset))?;

    let mut data_offset = 0_u64;
    while f.stream_position()? < tag0.end() {
        match f.read_be::<TagSection>() {
            Ok(section) => {
                let _endian = if section.is_le {
                    Endian::Little
                } else {
                    Endian::Big
                };
                println!("{section:#x?}");

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
                            let it: IndexItem = f.read_type(endian)?;
                            items.push(it);
                        }
                        items.sort_by_key(|i| i.offset);

                        let mut points: Vec<Vec3> = vec![];
                        // let mut base_transform = Mat4::IDENTITY;
                        let current_transform = Mat4::IDENTITY;
                        for it in &items {
                            println!("{it:x?} 0x{:x}", data_offset + it.offset as u64);

                            match it.typ {
                                0x74 | 0x81 | 0x8b => {
                                    f.save_pos(|f| {
                                        f.seek(SeekFrom::Start(
                                            data_offset + it.offset as u64 + 0x50,
                                        ))?;
                                        let _scale: [f32; 4] = f.read_type(endian)?;
                                        let _translation: [f32; 4] = f.read_type(endian)?;
                                        // dbg!(&scale);
                                        // dbg!(&translation);

                                        // current_transform = Mat4::from_scale_rotation_translation(
                                        //     Vec4::from_array(scale).truncate(),
                                        //     Quat::IDENTITY,
                                        //     Vec4::from_array(translation).truncate(),
                                        // );

                                        Ok(())
                                    })?;
                                }
                                0x88 => {
                                    f.save_pos(|f| {
                                        f.seek(SeekFrom::Start(
                                            data_offset + it.offset as u64 + 0x30,
                                        ))?;
                                        let aabb_extents =
                                            Vec4::from_array(f.read_type::<[f32; 4]>(endian)?);
                                        let aabb_center =
                                            Vec4::from_array(f.read_type::<[f32; 4]>(endian)?);

                                        println!("aabb_extents: {aabb_extents:?}");
                                        println!("aabb_center: {aabb_center:?}");

                                        let aabb_min = aabb_center - aabb_extents;
                                        let aabb_max = aabb_center + aabb_extents;

                                        println!("aabb_min: {aabb_min:?}");
                                        println!("aabb_max: {aabb_max:?}");

                                        Ok(())

                                        // current_transform = base_transform
                                        //     * Mat4::from_scale_rotation_translation(
                                        //         Vec4::from_array(scale).truncate(),
                                        //         Quat::IDENTITY,
                                        //         Vec4::from_array(translation).truncate(),
                                        //     );

                                        // Ok(())
                                    })?;
                                }
                                0x1b => {
                                    f.save_pos(|f| {
                                        f.seek(SeekFrom::Start(data_offset + it.offset as u64))?;
                                        for _ in 0..it.count {
                                            let point: [f32; 4] = f.read_type(endian)?;

                                            println!("{point:?}");

                                            let p = Vec4::from_array(point);
                                            points.push(
                                                current_transform.project_point3(p.truncate()),
                                            );
                                        }

                                        Ok(())
                                    })?;
                                }
                                0x20 => {
                                    f.seek(SeekFrom::Start(data_offset + it.offset as u64))?;
                                    for _ in 0..it.count {
                                        let rv: [[f32; 4]; 3] = f.read_type(endian)?;

                                        let vertices = [
                                            Vec3::new(rv[0][0], rv[1][0], rv[2][0]),
                                            Vec3::new(rv[0][1], rv[1][1], rv[2][1]),
                                            Vec3::new(rv[0][2], rv[1][2], rv[2][2]),
                                            Vec3::new(rv[0][3], rv[1][3], rv[2][3]),
                                        ];

                                        println!("{vertices:#?}");
                                        println!();
                                    }
                                }
                                // 0x9c => {
                                //     let mut obj = BufWriter::new(File::create(format!(
                                //         "stuff/{filename}.obj"
                                //     ))?);
                                //     let mut index_base = 1;
                                //     writeln!(&mut obj, "o Havok bounds")?;
                                //     f.save_pos(|f| {
                                //         f.seek(SeekFrom::Start(data_offset + it.offset as u64))?;
                                //         for _ in 0..it.count {
                                //             let e: HktUnk9c = f.read_type(endian)?;

                                //             let min = Vec4::from_array(e.min);
                                //             let max = Vec4::from_array(e.max);
                                //             println!("{min} {max}");

                                //             let center = (min + max) / 2.0;
                                //             let extents = (max - min) / 2.0;

                                //             let transform = Mat4::from_scale_rotation_translation(
                                //                 extents.truncate(),
                                //                 Quat::IDENTITY,
                                //                 center.truncate(),
                                //             );

                                //             for v in CUBE_VERTICES
                                //                 .iter()
                                //                 .map(|p| transform.transform_point3(*p))
                                //             {
                                //                 writeln!(&mut obj, "v {} {} {}", v.x, v.y, v.z)?;
                                //             }

                                //             for i in CUBE_INDICES.chunks_exact(3) {
                                //                 writeln!(
                                //                     &mut obj,
                                //                     "f {} {} {}",
                                //                     index_base + i[0],
                                //                     index_base + i[1],
                                //                     index_base + i[2]
                                //                 )?;
                                //             }

                                //             index_base += CUBE_VERTICES.len() as u32;
                                //         }

                                //         Ok(())
                                //     })?;
                                // }
                                _ => {}
                            }

                            // let type_and_flags: u32 = f.read_type(endian)?;

                            // let typ = type_and_flags & 0xFFFFFF;
                            // let flags = (type_and_flags & 0xFF000000) >> 24;
                            // let offset: u32 = f.read_type(endian)?;
                            // let offset = data_offset + offset as u64;
                            // let count: u32 = f.read_type(endian)?;

                            // println!(" - type=0x{typ:X} flags=0x{flags:X} offset=0x{offset:X} count={count}");
                        }
                        if !points.is_empty() {
                            let mut obj =
                                BufWriter::new(File::create(format!("stuff/{filename}_pts.obj"))?);
                            writeln!(&mut obj, "o Havok pointies {filename}")?;

                            let points_raw =
                                points.iter().map(|v| v.to_array().to_vec()).collect_vec();
                            let hull =
                                chull::ConvexHullWrapper::try_new(&points_raw, None).unwrap();

                            let (vertices, indices) = hull.vertices_indices();
                            for v in vertices {
                                writeln!(&mut obj, "v {} {} {}", v[0], v[1], v[2])?;
                            }

                            for i in indices.chunks_exact(3) {
                                writeln!(&mut obj, "f {} {} {}", i[0] + 1, i[1] + 1, i[2] + 1)?;
                            }
                        }
                    }
                    TagSectionSignature::SdkVersion => {
                        f.seek(SeekFrom::Start(section.offset))?;
                        let mut data = vec![0u8; section.size];
                        f.read_exact(&mut data)?;
                        let version = String::from_utf8_lossy(&data);
                        println!("SDK Version: {version}");
                    }
                    TagSectionSignature::Data => {
                        println!(
                            "Data: {0}/0x{0:X} bytes @ 0x{1:X}",
                            section.size, section.offset
                        );
                        data_offset = section.offset;
                    }
                    _ => {}
                }

                f.seek(SeekFrom::Start(section.offset + section.size as u64))?;
            }
            Err(e) => {
                panic!("Failed to read section: {}", e);
            }
        }
    }

    Ok(())
}

trait SeekSaveExt: Seek {
    fn save_pos<F>(&mut self, inner: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Self) -> anyhow::Result<()>,
    {
        let pos = self.stream_position()?;

        inner(self)?;

        self.seek(SeekFrom::Start(pos))?;

        Ok(())
    }
}

impl<T: Read + Seek> SeekSaveExt for T {}
