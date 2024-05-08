pub mod common;
pub mod dynamic_geometry;
pub mod hierarchy;
pub mod light;
pub mod map;
pub mod resources;
pub mod scene_ext;
pub mod static_geometry;
pub mod tags;
pub mod terrain;
pub mod transform;
pub mod utility;

pub type Scene = hecs::World;
