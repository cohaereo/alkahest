use std::any::Any;

use assert_offset::AssertOffsets;
use glam::{Mat4, Vec3, Vec4};

use crate::object::RenderObjectHandle;

#[derive(Default)]
pub struct FramePacket {
    // pub alloc: Bump,
    pub frame_nodes: Vec<FrameNode>,
    // pub view_nodes: Vec<ViewNode>,
}

impl FramePacket {
    pub fn reset(&mut self) {
        // self.alloc.reset();
        self.frame_nodes.clear();
        // self.view_nodes.clear();
    }

    pub fn push_static_render_object(&mut self, render_object_handle: RenderObjectHandle) {
        self.frame_nodes.push(FrameNode {
            render_object_handle,
            data: Box::new(()),
            distance: f32::MAX,
            visible: true,
        });
    }

    pub fn push_dynamic_render_object(
        &mut self,
        render_object_handle: RenderObjectHandle,
        transform: CompactTransform,
        permutation: usize,
    ) {
        self.frame_nodes.push(FrameNode {
            render_object_handle,
            data: Box::new((transform, permutation)),
            distance: f32::MAX,
            visible: true,
        });
    }

    pub fn iter_visible(&self) -> impl Iterator<Item = &FrameNode> {
        self.frame_nodes.iter().filter(|node| node.visible)
    }
}

#[repr(C)]
#[derive(AssertOffsets)]
pub struct FrameNode {
    pub render_object_handle: RenderObjectHandle,
    pub data: Box<dyn Any>,
    pub distance: f32, // TODO: Needs to be on view node
    pub visible: bool,
}

unsafe impl Send for FrameNode {}

// #[repr(C)]
// struct ViewNode {
//     pub frame_node: u32,
//     // TODO
//     // /// Distance between the view and the object
//     // pub distance: f32,
// }

#[repr(C)]
/// Compact representation of a local->world transform
#[derive(AssertOffsets, Debug, Clone)]
pub struct CompactTransform {
    #[offset(0)]
    x: Vec4,
    #[offset(16)]
    y: Vec4,
    #[offset(32)]
    z: Vec4,
}

impl CompactTransform {
    pub const IDENTITY: Self = Self {
        x: Vec4::X,
        y: Vec4::Y,
        z: Vec4::Z,
    };

    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_cols(
            self.x.with_w(0.0),
            self.y.with_w(0.0),
            self.z.with_w(0.0),
            Vec4::new(self.x.w, self.y.w, self.z.w, 1.0),
        )
    }

    pub fn from_mat4(mat: Mat4) -> Self {
        let pos = mat.w_axis;
        Self {
            x: mat.x_axis.with_w(pos.x),
            y: mat.y_axis.with_w(pos.y),
            z: mat.z_axis.with_w(pos.z),
        }
    }

    pub fn translation(&self) -> Vec3 {
        Vec3::new(self.x.w, self.y.w, self.z.w)
    }
}

impl From<Mat4> for CompactTransform {
    fn from(mat: Mat4) -> Self {
        Self::from_mat4(mat)
    }
}

// #[repr(C)]
// pub struct UniformTransform {
//     pub rotation: Quat,
//     pub translation: Vec3,
//     pub scale: f32,
// }
