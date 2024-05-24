use std::fmt::{Debug, Formatter, Write};

use binrw::BinRead;
use tiger_parse::TigerReadable;

#[derive(BinRead, Copy, Clone, PartialEq, Hash, Eq)]
pub struct ResourceHash(pub u32);

impl From<ResourceHash> for u32 {
    fn from(value: ResourceHash) -> Self {
        value.0
    }
}

impl From<u32> for ResourceHash {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl ResourceHash {
    pub fn is_none(&self) -> bool {
        self.0 == 0x811c9dc5
    }
}

impl TigerReadable for ResourceHash {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(ResourceHash(u32::read_ds_endian(reader, endian)?))
    }

    const SIZE: usize = u32::SIZE;
    const ZEROCOPY: bool = true;
}

impl Debug for ResourceHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            f.write_fmt(format_args!("ResourceHash(NONE)"))
        } else {
            f.write_str("ResourceHash(")?;

            self.0.fmt(f)?;

            f.write_char(')')
        }
    }
}
