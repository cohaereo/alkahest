use binrw::Endian;

pub fn u64_from_endian(endian: Endian, bytes: [u8; 8]) -> u64 {
    match endian {
        Endian::Big => u64::from_be_bytes(bytes),
        Endian::Little => u64::from_le_bytes(bytes),
    }
}

pub fn u32_from_endian(endian: Endian, bytes: [u8; 4]) -> u32 {
    match endian {
        Endian::Big => u32::from_be_bytes(bytes),
        Endian::Little => u32::from_le_bytes(bytes),
    }
}
