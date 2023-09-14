use crate::dxbc::{DxbcInputElement, DxbcInputType, DxbcSemanticType};
use crate::dxgi::DxgiFormat;
use windows::Win32::Graphics::Direct3D11::{D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

#[derive(PartialEq, Clone, Debug, Hash)]
pub struct InputElement {
    pub format: DxgiFormat,
    pub input_slot: u32,
    pub semantic_index: u32,
    pub semantic_type: DxbcSemanticType,
    pub component_count: usize,
    pub component_type: DxbcInputType,
}

pub type OutputElement = InputElement;

impl InputElement {
    pub fn from_dxbc(e: &DxbcInputElement, interpolated: bool, is_float: bool) -> InputElement {
        let ty = match e.component_mask.iter().count() {
            1 => InputType::Scalar,
            2 => InputType::Scalar2,
            3 => InputType::Scalar3,
            4 => InputType::Scalar4,
            _ => unreachable!(),
        };

        InputElement {
            format: ty.into_dxgi_type(&e.semantic_name.to_string(), interpolated, is_float),
            input_slot: 0,
            semantic_index: e.semantic_index,
            semantic_type: DxbcSemanticType::from_str(&e.semantic_name.to_string())
                .unwrap_or_else(|| panic!("Unknown semantic type '{}'", *e.semantic_name)),
            component_count: e.component_mask.bits().count_ones() as usize,
            component_type: e.component_type,
        }
    }
}

pub enum InputType {
    Scalar,
    Scalar2,
    Scalar3,
    Scalar4,
}

impl InputType {
    /// Align type to be usable with 16-bit formats
    pub fn align_16(self) -> InputType {
        match self {
            InputType::Scalar => InputType::Scalar,
            InputType::Scalar2 => InputType::Scalar2,
            InputType::Scalar3 => InputType::Scalar4,
            InputType::Scalar4 => InputType::Scalar4,
        }
    }

    /// Convert to a compatible DXGI_FORMAT
    /// This function aligns 16-bit types to 32-bit where necessary
    pub fn into_dxgi_type(
        self,
        semantic_name: &str,
        interpolated: bool,
        is_float: bool,
    ) -> DxgiFormat {
        match if !is_float { self.align_16() } else { self } {
            InputType::Scalar => {
                if is_float {
                    DxgiFormat::R32_FLOAT
                } else if interpolated {
                    DxgiFormat::R16_SNORM
                } else {
                    DxgiFormat::R16_SINT
                }
            }
            InputType::Scalar2 => {
                if is_float {
                    DxgiFormat::R32G32_FLOAT
                } else if interpolated {
                    DxgiFormat::R16G16_SNORM
                } else {
                    DxgiFormat::R16G16_SINT
                }
            }
            InputType::Scalar3 => {
                if is_float {
                    DxgiFormat::R32G32B32_FLOAT
                } else {
                    unreachable!()
                }
            }
            InputType::Scalar4 => {
                if semantic_name.starts_with("COLOR") {
                    DxgiFormat::R8G8B8A8_UNORM
                } else if is_float {
                    DxgiFormat::R32G32B32A32_FLOAT
                } else if interpolated {
                    DxgiFormat::R16G16B16A16_SNORM
                } else {
                    DxgiFormat::R16G16B16A16_SINT
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
            InputSlot: e.input_slot,
            AlignedByteOffset: offset,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        });

        offset += (e.format.bpp() / 8) as u32;
    }

    map
}
