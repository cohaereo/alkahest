#![allow(non_camel_case_types)]

pub mod bvtree;
pub mod compound_shape;
pub mod convex_vertices;
pub mod unknown;

// cohae: Temporary types until we have a custom reader to handle inline arrays
pub type hkArrayIndex = u64;
pub type hkPointerIndex = u64;
