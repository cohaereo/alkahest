use tiger_pkg::TagHash;

pub mod map;
pub mod pattern;
pub mod permutations;
pub mod render_objects;
pub mod transform;

pub struct UnimplementedTigerComponent {
    pub class_id: u32,
    pub hash: TagHash,
    pub name: Option<String>,
}

pub struct UnimplementedTigerComponents(pub Vec<UnimplementedTigerComponent>);
