use tiger_parse::{tiger_type, FnvHash, Padding, ResourcePointer, ResourcePointerWithClass};

use crate::{
    map::SComponentDataListPtr,
    tag::{Tag, WideHash},
    tfx::sequencer::SExpression,
};

#[tiger_type(id = 0x80809AD8)]
pub struct SPattern {
    pub file_size: u64,
    pub components: Vec<SComponentRef>,
}

#[tiger_type(id = 0x80809ACD)]
pub struct SComponentRef {
    pub unk0: Tag<SComponent>,
    pub unk4: u32,
    pub unk8: u32,
}

#[tiger_type(id = 0x80809B06, size = 0x88)]
pub struct SComponent {
    pub file_size: u64,
    // cohae: This field isn't a list, but it uses the same layout as ComponentDataListPtr
    pub dynamic_data: SComponentDataListPtr,
    pub default_instance: ResourcePointer,
    pub definition: ResourcePointer,

    pub unk20: Vec<ResourcePointerWithClass>,

    #[tiger(offset = 0x30)]
    pub resource_table1: Vec<()>,
}

#[tiger_type(id = 0x8080841b, size = 0x40)]
#[derive(Debug)]
pub struct S8080841B {
    #[tiger(offset = 0x30)]
    pub unk30: Vec<S8080841D>,
}

#[tiger_type(id = 0x8080841d, size = 0x18)]
#[derive(Debug)]
pub struct S8080841D {
    pub unk0: FnvHash,
    pub unk4: u32,
    pub entity: WideHash,
}

#[tiger_type(id = 0x80809597, size = 0x1A8)]
pub struct SObjectChannelComponent {
    // #[tiger(offset = 0x128)]
    // pub m_providers: Vec<()>,
    #[tiger(offset = 0x138)]
    pub m_channels: Vec<SObjectChannel>,
}

#[tiger_type(id = 0x808095A9, size = 0x70)]
pub struct SObjectChannel {
    pub name: FnvHash,
    _pad4: Padding<4>,
    pub expression: SExpression,
    #[tiger(offset = 0x60)]
    pub interpolation: u64,
}
