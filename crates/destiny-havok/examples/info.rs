use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    str::FromStr,
};

use binrw::{BinReaderExt, Endian, VecArgs};
use colored::Colorize;
use destiny_havok::{
    index::IndexItem,
    section::{TagSection, TagSectionSignature},
    types::convex_vertices::hkFourTransposedPoints,
};

fn main() -> anyhow::Result<()> {
    let mut f = File::open(std::env::args().nth(1).unwrap())?;
    let path = PathBuf::from_str(&std::env::args().nth(1).unwrap())?;
    let _filename = path.file_stem().unwrap().to_string_lossy().to_string();

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
                            items.push((items.len(), it));
                        }
                        items.sort_by_key(|(_, i)| i.offset);

                        for (index, item) in items.iter().skip(1) {
                            println!(
                                "{index}: flags={:?} type=0x{:x} count={} 0x{:x}",
                                item.flags,
                                item.typ,
                                item.count,
                                data_offset + item.offset as u64
                            );
                            match item.typ {
                                0x17 => println!("uint16"),
                                0x18 => println!("uint32"),
                                0x1b => println!("Vector4"),
                                0x20 => {
                                    println!("hkFourTransposedPoints");

                                    f.seek(SeekFrom::Start(data_offset + item.offset as u64))?;
                                    let _verts: Vec<hkFourTransposedPoints> = f.read_type_args(
                                        endian,
                                        VecArgs {
                                            count: item.count as _,
                                            inner: (),
                                        },
                                    )?;

                                    // for v in verts {
                                    //     println!("{:#x?}", v.transpose());
                                    // }
                                }
                                0x3f => println!("s_physics_component_havok_data"),
                                0x48 => println!("s_hkpShape_array_data"),
                                0x74 => println!("s_hkpShape_array"),
                                0x7d => println!("hkpBoxShape"),
                                0x88 => println!("hkpConvexVerticesShape"),
                                0x8b => println!("hkpBvCompressedMeshShape"),
                                0x99 => println!("hkpBvCompressedMeshShapeTreeDataRun"),
                                0x9c => println!("hkcdStaticMeshTreeBaseSection"),
                                0x9e => println!("hkcdStaticMeshTreeBasePrimitive"),
                                0xac => println!("hkcdStaticTreeCodec3Axis4"),
                                0xaf => println!("hkpStaticCompoundShape"),
                                0xb3 => println!("hkpStaticCompoundShapeInstance"),
                                0xb9 => println!("unkb9"),
                                u => eprintln!("{}", format!("Unknown type 0x{u:x}").red()),
                            }

                            println!();
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
