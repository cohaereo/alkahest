use std::fmt::{Display, Formatter};

// Using a u32 so we can pass this option to the composite shader directly
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum CompositorMode {
    /// Rendered output
    Combined = 0,

    /// RT0
    Albedo = 1,
    /// RT1
    Normal = 2,
    /// RT2
    PbrStack = 3,

    SmoothnessFuzz = 4,
    Metalicness = 5,
    TextureAO = 6,
    Emission = 7,
    Transmission = 8,
    VertexAO = 9,
    Iridescence = 10,
}

pub const COMPOSITOR_MODES: &[CompositorMode] = &[
    CompositorMode::Combined,
    CompositorMode::Albedo,
    CompositorMode::Normal,
    CompositorMode::PbrStack,
    CompositorMode::SmoothnessFuzz,
    CompositorMode::Metalicness,
    CompositorMode::Emission,
    CompositorMode::Transmission,
    CompositorMode::Iridescence,
    CompositorMode::TextureAO,
    CompositorMode::VertexAO,
];

impl Display for CompositorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositorMode::Combined => f.write_str("Combined"),
            CompositorMode::Albedo => f.write_str("Albedo (RT0)"),
            CompositorMode::Normal => f.write_str("Normal (RT1)"),
            CompositorMode::PbrStack => f.write_str("PBR Stack (RT2)"),
            CompositorMode::SmoothnessFuzz => f.write_str("Smoothness/Fuzz"),
            CompositorMode::Metalicness => f.write_str("Metalicness"),
            CompositorMode::TextureAO => f.write_str("Texture AO"),
            CompositorMode::Emission => f.write_str("Emission"),
            CompositorMode::Transmission => f.write_str("Transmission"),
            CompositorMode::VertexAO => f.write_str("Vertex AO"),
            CompositorMode::Iridescence => f.write_str("Iridescence"),
        }
    }
}
