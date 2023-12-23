use binrw::binread;

#[binread]
#[br(repr = u64)]
#[derive(Debug)]
pub enum BvTreeType {
    Mopp = 0,
    TrisampledHeightfield = 1,
    StaticCompound = 2,
    CompressedMesh = 3,
    User = 4,
    Max = 5,
}
