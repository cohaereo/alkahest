use tiger_parse::tiger_tag;

#[derive(Debug, Clone)]
#[tiger_tag(etype = 32, esubtype = 4)]
pub struct VertexBufferHeader {
    pub data_size: u32,
    pub stride: u16,
    pub vtype: u16,
    // pub deadbeef: DeadBeefMarker,
    pub deadbeef: u32,
}

#[derive(Debug, Clone)]
#[tiger_tag(etype = 32, esubtype = 6)]
pub struct IndexBufferHeader {
    pub unk0: i8,
    pub is_32bit: bool,
    // Probably padding
    pub unk1: u16,
    pub zero: u32,
    pub data_size: u64,
    // pub deadbeef: DeadBeefMarker,
    pub deadbeef: u32,
    pub zero1: u32,
}
