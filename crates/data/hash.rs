use tiger_parse::FnvHash;

pub const FNV1_BASE: u32 = 0x811c9dc5;
pub const FNV1_PRIME: u32 = 0x01000193;
pub fn fnv1(data: impl AsRef<[u8]>) -> FnvHash {
    data.as_ref().iter().fold(FNV1_BASE, |acc, b| {
        acc.wrapping_mul(FNV1_PRIME) ^ (*b as u32)
    })
}
