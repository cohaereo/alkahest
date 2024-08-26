use glam::Vec4;

#[derive(Clone)]
pub struct GlobalChannel {
    pub name: Option<String>,
    pub value: Vec4,
    pub editor_type: ChannelType,
}

impl GlobalChannel {
    pub fn new(name: impl AsRef<str>, ty: ChannelType, initial_value: Vec4) -> Self {
        Self {
            name: Some(name.as_ref().to_string()),
            value: initial_value,
            editor_type: ty,
        }
    }
}

impl Default for GlobalChannel {
    fn default() -> Self {
        Self {
            name: None,
            value: Vec4::ONE,
            editor_type: ChannelType::Vec4,
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum ChannelType {
    Vec4,
    /// Uses the first value of the Vec4
    Float,
    /// Linear color-space. Usage of the alpha channel is different per channel
    Color,
}
