use crate::dxbc::{DxbcInputElement, SemanticType};
use crate::dxgi::DxgiFormat;
use windows::Win32::Graphics::Direct3D11::{D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

#[derive(PartialEq, Clone, Debug)]
pub struct InputElement {
    pub format: DxgiFormat,
    pub semantic_index: u32,
    pub semantic_type: SemanticType,
}

impl InputElement {
    pub fn from_dxbc(e: &DxbcInputElement, interpolated: bool, is_float: bool) -> InputElement {
        let ty = match e.component_mask.iter().count() {
            1 => InputType::Float,
            2 => InputType::Float2,
            3 => InputType::Float3,
            4 => InputType::Float4,
            _ => unreachable!(),
        };

        InputElement {
            format: ty.to_dxgi_type(&e.semantic_name.to_string(), interpolated, is_float),
            semantic_index: e.semantic_index,
            semantic_type: SemanticType::from_str(&e.semantic_name.to_string())
                .unwrap_or(SemanticType::TexCoord),
        }
    }
}

pub enum InputType {
    Float,
    Float2,
    Float3,
    Float4,
}

impl InputType {
    /// Align type to be usable with 16-bit formats
    pub fn align_16(self) -> InputType {
        match self {
            InputType::Float => InputType::Float,
            InputType::Float2 => InputType::Float2,
            InputType::Float3 => InputType::Float4,
            InputType::Float4 => InputType::Float4,
        }
    }

    /// Convert to a compatible DXGI_FORMAT
    /// This function aligns 16-bit types to 32-bit where necessary
    pub fn to_dxgi_type(
        self,
        semantic_name: &str,
        interpolated: bool,
        is_float: bool,
    ) -> DxgiFormat {
        match if !is_float { self.align_16() } else { self } {
            InputType::Float => {
                if is_float {
                    DxgiFormat::R32_FLOAT
                } else {
                    if interpolated {
                        DxgiFormat::R16_SNORM
                    } else {
                        DxgiFormat::R16_SINT
                    }
                }
            }
            InputType::Float2 => {
                if is_float {
                    DxgiFormat::R32G32_FLOAT
                } else {
                    if interpolated {
                        DxgiFormat::R16G16_SNORM
                    } else {
                        DxgiFormat::R16G16_SINT
                    }
                }
            }
            InputType::Float3 => {
                if is_float {
                    DxgiFormat::R32G32B32_FLOAT
                } else {
                    unreachable!()
                }
            }
            InputType::Float4 => {
                if semantic_name.starts_with("COLOR") {
                    DxgiFormat::R8G8B8A8_UNORM
                } else {
                    if is_float {
                        DxgiFormat::R32G32B32A32_FLOAT
                    } else {
                        if interpolated {
                            DxgiFormat::R16G16B16A16_SNORM
                        } else {
                            DxgiFormat::R16G16B16A16_SINT
                        }
                    }
                }
            }
        }
    }
}

pub fn build_input_layout(elements: &[InputElement]) -> Vec<D3D11_INPUT_ELEMENT_DESC> {
    let mut map = vec![];
    let mut offset = 0;
    for e in elements
        .iter()
        .filter(|e| !e.semantic_type.is_system_value())
    {
        map.push(D3D11_INPUT_ELEMENT_DESC {
            SemanticName: e.semantic_type.to_pcstr(),
            SemanticIndex: e.semantic_index,
            Format: DXGI_FORMAT(e.format.into()),
            InputSlot: 0,
            AlignedByteOffset: offset,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        });

        offset += (e.format.bpp() / 8) as u32;
    }

    map
}
