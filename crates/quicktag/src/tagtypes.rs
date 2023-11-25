use std::fmt::Display;

use destiny_pkg::PackageVersion;
use eframe::epaint::Color32;

use crate::packages::package_manager;

pub enum TagType {
    Texture2D { is_header: bool },
    TextureCube { is_header: bool },
    Texture3D { is_header: bool },
    TextureSampler { is_header: bool },
    TextureLargeBuffer,

    VertexBuffer { is_header: bool },
    IndexBuffer { is_header: bool },
    ConstantBuffer { is_header: bool },
    PixelShader { is_header: bool },
    VertexShader { is_header: bool },
    ComputeShader { is_header: bool },

    WwiseBank,
    WwiseStream,

    Havok,
    UmbraTome,
    CriwareUsm,

    Tag,
    TagGlobal,

    Unknown { ftype: u8, fsubtype: u8 },
}

impl TagType {
    pub fn is_texture(&self) -> bool {
        matches!(
            self,
            TagType::Texture2D { .. } | TagType::TextureCube { .. } | TagType::Texture3D { .. }
        )
    }

    pub fn display_color(&self) -> Color32 {
        match self {
            TagType::Texture2D { .. }
            | TagType::TextureCube { .. }
            | TagType::Texture3D { .. }
            | TagType::TextureSampler { .. }
            | TagType::TextureLargeBuffer { .. } => Color32::GREEN,

            TagType::VertexBuffer { .. }
            | TagType::IndexBuffer { .. }
            | TagType::ConstantBuffer { .. } => Color32::LIGHT_BLUE,

            TagType::PixelShader { .. }
            | TagType::VertexShader { .. }
            | TagType::ComputeShader { .. } => Color32::from_rgb(249, 168, 71),

            TagType::WwiseBank | TagType::WwiseStream => Color32::from_rgb(191, 106, 247),
            TagType::Havok | TagType::UmbraTome | TagType::CriwareUsm => Color32::YELLOW,

            TagType::Tag | TagType::TagGlobal => Color32::GRAY,

            TagType::Unknown { .. } => Color32::LIGHT_RED,
        }
    }
}

impl Display for TagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagType::Texture2D { is_header } => f.write_fmt(format_args!(
                "Texture2D{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::TextureCube { is_header } => f.write_fmt(format_args!(
                "TextureCube{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::Texture3D { is_header } => f.write_fmt(format_args!(
                "Texture3D{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::TextureLargeBuffer => f.write_str("TextureLargeBuffer"),
            TagType::TextureSampler { is_header } => f.write_fmt(format_args!(
                "TextureSampler{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::VertexBuffer { is_header } => f.write_fmt(format_args!(
                "VertexBuffer{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::IndexBuffer { is_header } => f.write_fmt(format_args!(
                "IndexBuffer{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::ConstantBuffer { is_header } => f.write_fmt(format_args!(
                "ConstantBuffer{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::PixelShader { is_header } => f.write_fmt(format_args!(
                "PixelShader{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::VertexShader { is_header } => f.write_fmt(format_args!(
                "VertexShader{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::ComputeShader { is_header } => f.write_fmt(format_args!(
                "ComputeShader{}",
                if *is_header { "" } else { " (Data)" }
            )),
            TagType::Tag => f.write_str("Tag"),
            TagType::TagGlobal => f.write_str("TagGlobal"),
            TagType::WwiseBank => f.write_str("WwiseBank"),
            TagType::WwiseStream => f.write_str("WwiseStream"),
            TagType::Havok => f.write_str("Havok"),
            TagType::UmbraTome => f.write_str("UmbraTome"),
            TagType::CriwareUsm => f.write_str("CriwareUsm"),
            TagType::Unknown { ftype, fsubtype } => {
                f.write_fmt(format_args!("Unk{ftype}+{fsubtype}"))
            }
        }
    }
}

impl TagType {
    pub fn from_type_subtype(t: u8, st: u8) -> TagType {
        match package_manager().version {
            PackageVersion::Destiny2Shadowkeep => Self::from_type_subtype_sk(t, st),
            PackageVersion::Destiny2BeyondLight
            | PackageVersion::Destiny2WitchQueen
            | PackageVersion::Destiny2Lightfall => Self::from_type_subtype_lf(t, st),
            _ => TagType::Unknown {
                ftype: t,
                fsubtype: st,
            },
        }
    }

    pub fn from_type_subtype_sk(t: u8, st: u8) -> TagType {
        let is_header = matches!(t, 32..=34);

        match (t, st) {
            (8, 0) => TagType::Tag,
            (16, 0) => TagType::TagGlobal,
            (26, 5) => TagType::WwiseBank,
            (26, 6) => TagType::WwiseStream,
            (32 | 40, _) => match st {
                1 => TagType::Texture2D { is_header },
                2 => TagType::TextureCube { is_header },
                3 => TagType::Texture3D { is_header },
                4 => TagType::VertexBuffer { is_header },
                6 => TagType::IndexBuffer { is_header },
                7 => TagType::ConstantBuffer { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (33 | 41, _) => match st {
                0 => TagType::PixelShader { is_header },
                1 => TagType::VertexShader { is_header },
                6 => TagType::ComputeShader { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (34 | 42, _) => match st {
                1 => TagType::TextureSampler { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (ftype, fsubtype) => TagType::Unknown { ftype, fsubtype },
        }
    }

    pub fn from_type_subtype_lf(t: u8, st: u8) -> TagType {
        let is_header = matches!(t, 32..=34);

        match (t, st) {
            (8, 0) => TagType::Tag,
            (16, 0) => TagType::TagGlobal,
            (24, 0) => TagType::UmbraTome,
            (26, 6) => TagType::WwiseBank,
            (26, 7) => TagType::WwiseStream,
            (27, 0) => TagType::Havok,
            (27, 1) => TagType::CriwareUsm,
            (32 | 40, _) => match st {
                1 => TagType::Texture2D { is_header },
                2 => TagType::TextureCube { is_header },
                3 => TagType::Texture3D { is_header },
                4 => TagType::VertexBuffer { is_header },
                6 => TagType::IndexBuffer { is_header },
                7 => TagType::ConstantBuffer { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (33 | 41, _) => match st {
                0 => TagType::PixelShader { is_header },
                1 => TagType::VertexShader { is_header },
                6 => TagType::ComputeShader { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (34 | 42, _) => match st {
                1 => TagType::TextureSampler { is_header },
                fsubtype => TagType::Unknown { ftype: t, fsubtype },
            },
            (48, 1) => TagType::TextureLargeBuffer,
            (ftype, fsubtype) => TagType::Unknown { ftype, fsubtype },
        }
    }
}
