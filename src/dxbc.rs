use anyhow::anyhow;
use binrw::{BinRead, BinReaderExt, BinResult, Endian, FilePtr32, NullString};
use bitflags::{bitflags, Flags};
use std::io::{Read, Seek, SeekFrom};
use windows::core::PCSTR;

#[derive(BinRead, Debug)]
#[br(magic = b"DXBC")]
pub struct DxbcHeader {
    pub checksum: [u8; 16],
    pub _unk14: u32,
    pub file_size: u32,

    pub chunk_count: u32,
    #[br(count = chunk_count)]
    pub chunk_offsets: Vec<u32>,
}

#[derive(BinRead, Debug)]
// #[br(magic = b"ISGN")]
pub struct DxbcInputSignature {
    pub chunk_size: u32,

    #[br(try_calc(__binrw_generated_var_reader.stream_position()))]
    string_base_offset: u64,

    pub element_count: u32,
    pub _unkc: u32,

    #[br(count = element_count, args { inner: (string_base_offset,) })]
    pub elements: Vec<DxbcInputElement>,
}

#[derive(BinRead, Debug)]
#[br(import(string_base_offset: u64))]
pub struct DxbcInputElement {
    #[br(offset = string_base_offset)]
    pub semantic_name: FilePtr32<NullString>,
    pub semantic_index: u32,
    pub system_value_type: u32,
    pub component_type: DxbcInputType,
    pub register: u32,
    pub component_mask: ComponentMask,
    pub component_mask_rw: ComponentMask,
    _pad: u16,
}

#[derive(BinRead, Debug, PartialEq)]
#[br(repr(u32))]
pub enum DxbcInputType {
    Uint = 1,
    Int = 2,
    Float = 3,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SemanticType {
    Position,
    TexCoord,
    Normal,
    Tangent,
    Color,

    SystemVertexId,
    SystemInstanceId,
}

impl SemanticType {
    pub fn from_str(s: &str) -> Option<SemanticType> {
        Some(match s {
            "POSITION" => SemanticType::Position,
            "TEXCOORD" => SemanticType::TexCoord,
            "NORMAL" => SemanticType::Normal,
            "TANGENT" => SemanticType::Tangent,
            "COLOR" => SemanticType::Color,
            "SV_VERTEXID" => SemanticType::SystemVertexId,
            "SV_InstanceID" => SemanticType::SystemInstanceId,
            _ => return None,
        })
    }

    pub fn to_pcstr(&self) -> PCSTR {
        match self {
            SemanticType::Position => s!("POSITION"),
            SemanticType::TexCoord => s!("TEXCOORD"),
            SemanticType::Normal => s!("NORMAL"),
            SemanticType::Tangent => s!("TANGENT"),
            SemanticType::Color => s!("COLOR"),

            SemanticType::SystemVertexId => s!("SV_VERTEXID"),
            SemanticType::SystemInstanceId => s!("SV_InstanceID"),
        }
    }

    pub fn is_system_value(&self) -> bool {
        matches!(
            self,
            SemanticType::SystemVertexId | SemanticType::SystemInstanceId
        )
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ComponentMask: u8 {
        const X = (1 << 0);
        const Y = (1 << 1);
        const Z = (1 << 2);
        const W = (1 << 3);

        const XY = Self::X.bits() | Self::Y.bits();
        const XYZ = Self::XY.bits() | Self::Z.bits();
        const XYZW = Self::XYZ.bits() | Self::W.bits();
    }
}

impl BinRead for ComponentMask {
    type Args<'a> = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        _args: Self::Args<'_>,
    ) -> BinResult<Self> {
        Ok(ComponentMask::from_bits(reader.read_type::<u8>(endian)?).unwrap())
    }
}

/// Find ISGN chunk and read it
pub fn get_input_signature<R: Read + Seek>(
    reader: &mut R,
    header: &DxbcHeader,
) -> anyhow::Result<DxbcInputSignature> {
    for chunk_offset in &header.chunk_offsets {
        reader.seek(SeekFrom::Start(*chunk_offset as _))?;

        let chunk_magic: [u8; 4] = reader.read_le()?;
        if &chunk_magic == b"ISGN" {
            return Ok(reader.read_le()?);
        }
    }

    Err(anyhow!("Could not find ISGN chunk"))
}
