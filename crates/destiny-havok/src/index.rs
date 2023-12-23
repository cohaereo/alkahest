use binrw::binread;
use bitflags::bitflags;

#[derive(Debug)]
#[binread]
pub struct IndexItem {
    pub type_and_flags: u32,

    #[br(calc(type_and_flags & 0xFFFFFF))]
    pub typ: u32,

    #[br(calc(ItemFlags::from_bits_retain((type_and_flags & 0xFF000000) >> 24)))]
    pub flags: ItemFlags,

    pub offset: u32,
    pub count: u32,
}

bitflags! {
    #[derive(Debug)]
    pub struct ItemFlags: u32 {
        const POINTER = 0x10;
        const ARRAY = 0x20;
    }
}
