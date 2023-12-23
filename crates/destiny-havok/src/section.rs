use binrw::binread;

#[derive(Debug)]
#[binread(big)]
pub struct TagSection {
    pub flags_and_size: u32,

    #[br(calc((flags_and_size as usize & 0x3fffffff) - 8))]
    pub size: usize,

    #[br(calc((flags_and_size & 0x40000000) != 0))]
    /// Is this section little endian?
    pub is_le: bool,

    pub signature: TagSectionSignature,

    #[br(try_calc(__binrw_generated_var_reader.stream_position()))]
    pub offset: u64,
}

impl TagSection {
    pub fn end(&self) -> u64 {
        self.offset + self.size as u64
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binread(big)]
pub enum TagSectionSignature {
    #[br(magic = b"TAG0")]
    Tag0,

    #[br(magic = b"SDKV")]
    SdkVersion,

    #[br(magic = b"DATA")]
    Data,

    #[br(magic = b"INDX")]
    Index,

    #[br(magic = b"ITEM")]
    IndexItem,

    #[br(magic = b"PTCH")]
    Ptch,

    #[br(magic = b"TCRF")]
    Tcrf,
}
