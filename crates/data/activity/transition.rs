use tiger_parse::{tiger_type, FnvHash, NullString, Padding, Pointer};

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096BF)]
pub struct S808096BF {
    pub file_size: u64,
    pub destination_name: Pointer<NullString>,
    pub unk10: Vec<S808096CE>,
    pub bubbles: Vec<S808096C3>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096CE)]
pub struct S808096CE {
    pub unk0: u64,
    pub name: FnvHash,
    pub unkc: u16,
    pub unke: u16,
    pub unk10: u32,
    pub bubble_index: i32,
    pub unk18: u32,
    pub unk1c: FnvHash,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x808096C3)]
pub struct S808096C3 {
    pub name: FnvHash,
    pub index_into_unk10: u32,
    pub unk8: [u32; 4],
    pub transitions: Vec<S80809A4F>,
    pub dependencies: Vec<S80809A4F>,
    pub unk38: u32,
    pub unk3c: u32,
    pub unk40: [u8; 4],
    pub unk44: [u32; 7],
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x80809A4F)]
pub struct S80809A4F {
    pub unk0: u16,
}
