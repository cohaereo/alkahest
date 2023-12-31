use anyhow::anyhow;
use binrw::{BinRead, BinReaderExt, BinResult, Endian, FilePtr32, NullString};
use bitflags::bitflags;
use std::{
    fmt::Display,
    io::{Read, Seek, SeekFrom},
};
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
pub struct DxbcIoSignature {
    pub chunk_size: u32,

    #[br(try_calc(__binrw_generated_var_reader.stream_position()))]
    _string_base_offset: u64,

    pub element_count: u32,
    pub _unkc: u32,

    #[br(count = element_count, args { inner: (_string_base_offset,) })]
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

#[derive(BinRead, Debug, PartialEq, Copy, Clone, Hash)]
#[br(repr(u32))]
pub enum DxbcInputType {
    Uint = 1,
    Int = 2,
    Float = 3,
}

impl Display for DxbcInputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxbcInputType::Uint => f.write_str("uint"),
            DxbcInputType::Int => f.write_str("int"),
            DxbcInputType::Float => f.write_str("float"),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone, Hash)]
pub enum DxbcSemanticType {
    Position,
    TexCoord,
    Normal,
    Tangent,
    Binormal,
    Color,
    BlendWeight,
    BlendIndices,

    SystemVertexId,
    SystemInstanceId,
    SystemTarget,
    SystemPosition,
    SystemIsFrontFace,
}

impl DxbcSemanticType {
    pub fn from_str(s: &str) -> Option<DxbcSemanticType> {
        Some(match s {
            "POSITION" => DxbcSemanticType::Position,
            "TEXCOORD" => DxbcSemanticType::TexCoord,
            "NORMAL" => DxbcSemanticType::Normal,
            "TANGENT" => DxbcSemanticType::Tangent,
            "BINORMAL" => DxbcSemanticType::Binormal,
            "COLOR" => DxbcSemanticType::Color,
            "BLENDWEIGHT" => DxbcSemanticType::BlendWeight,
            "BLENDINDICES" => DxbcSemanticType::BlendIndices,
            "SV_VERTEXID" => DxbcSemanticType::SystemVertexId,
            "SV_VertexID" => DxbcSemanticType::SystemVertexId,
            "SV_InstanceID" => DxbcSemanticType::SystemInstanceId,
            "SV_TARGET" => DxbcSemanticType::SystemTarget,
            "SV_POSITION" => DxbcSemanticType::SystemPosition,
            "SV_isFrontFace" => DxbcSemanticType::SystemIsFrontFace,
            "SV_Target" => DxbcSemanticType::SystemTarget,
            _ => return None,
        })
    }

    pub fn to_pcstr(self) -> PCSTR {
        match self {
            DxbcSemanticType::Position => s!("POSITION"),
            DxbcSemanticType::TexCoord => s!("TEXCOORD"),
            DxbcSemanticType::Normal => s!("NORMAL"),
            DxbcSemanticType::Tangent => s!("TANGENT"),
            DxbcSemanticType::Binormal => s!("BINORMAL"),
            DxbcSemanticType::Color => s!("COLOR"),
            DxbcSemanticType::BlendWeight => s!("BLENDWEIGHT"),
            DxbcSemanticType::BlendIndices => s!("BLENDINDICES"),

            DxbcSemanticType::SystemVertexId => s!("SV_VERTEXID"),
            DxbcSemanticType::SystemInstanceId => s!("SV_InstanceID"),
            DxbcSemanticType::SystemTarget => s!("SV_TARGET"),
            DxbcSemanticType::SystemPosition => s!("SV_POSITION"),
            DxbcSemanticType::SystemIsFrontFace => s!("SV_isFrontFace"),
        }
    }

    pub fn is_system_value(&self) -> bool {
        matches!(
            self,
            DxbcSemanticType::SystemVertexId
                | DxbcSemanticType::SystemInstanceId
                | DxbcSemanticType::SystemTarget
                | DxbcSemanticType::SystemPosition
                | DxbcSemanticType::SystemIsFrontFace
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
) -> anyhow::Result<DxbcIoSignature> {
    for chunk_offset in &header.chunk_offsets {
        reader.seek(SeekFrom::Start(*chunk_offset as _))?;

        let chunk_magic: [u8; 4] = reader.read_le()?;
        if &chunk_magic == b"ISGN" {
            return Ok(reader.read_le()?);
        }
    }

    Err(anyhow!("Could not find ISGN chunk"))
}

/// Find OSGN chunk and read it
pub fn get_output_signature<R: Read + Seek>(
    reader: &mut R,
    header: &DxbcHeader,
) -> anyhow::Result<DxbcIoSignature> {
    for chunk_offset in &header.chunk_offsets {
        reader.seek(SeekFrom::Start(*chunk_offset as _))?;

        let chunk_magic: [u8; 4] = reader.read_le()?;
        if &chunk_magic == b"OSGN" {
            return Ok(reader.read_le()?);
        }
    }

    Err(anyhow!("Could not find OSGN chunk"))
}
