use tiger_parse::{
    tiger_type, FnvHash, OptionalVariantPointer, ResourcePointer, ResourcePointerWithClass,
    VariantPointer,
};

use crate::{
    map::ComponentData,
    tag::{Tag, WideHash},
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
    pub dynamic_data: OptionalVariantPointer<ComponentData>,
    pub unk10: ResourcePointer,
    pub unk18: ResourcePointer,

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
