use std::any::Any;

use assert_offset::AssertOffsets;
use glam::{Mat4, Vec3, Vec4};

use crate::{object::RenderObjectHandle, renderer::submit::atmosphere::AtmosphereData};

#[derive(Default)]
pub struct FramePacket {
    // pub alloc: Bump,
    pub frame_nodes: Vec<FrameNode>,
    pub views: Vec<ExtractedView>,
    pub misc: FramePacketMisc,
}

impl FramePacket {
    pub fn begin_frame(&mut self, misc: FramePacketMisc) {
        self.reset();
        self.misc = misc;
    }

    pub fn reset(&mut self) {
        // self.alloc.reset();
        self.frame_nodes.clear();
        // self.view_nodes.clear();
    }

    pub fn push_static_render_object(&mut self, render_object_handle: RenderObjectHandle) {
        self.frame_nodes.push(FrameNode {
            render_object_handle,
            data: Box::new(()),
            distance: 0.0,
            visible: VisibilityMask::default(),
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
            distance: 0.0,
            visible: VisibilityMask::default(),
        });
    }

    pub fn iter_visible(&self, view: usize) -> impl Iterator<Item = &FrameNode> {
        self.frame_nodes
            .iter()
            .filter(move |node| node.visible.get(view))
    }
}

#[repr(C)]
#[derive(AssertOffsets)]
pub struct FrameNode {
    pub render_object_handle: RenderObjectHandle,
    pub data: Box<dyn Any>,
    /// Distance between the main view and the object
    pub distance: f32,
    pub visible: VisibilityMask,
}

unsafe impl Send for FrameNode {}

pub struct ExtractedView {
    pub source: ViewSource,
}

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

#[derive(Default)]
pub struct FramePacketMisc {
    pub time: f32,
    pub delta_time: f32,

    pub atmosphere: AtmosphereData,
    pub time_of_day: f32,
}

pub enum ViewSource {
    Main,
    Sun,
    ShadowLight(u32),
}

#[derive(Debug, Default)]
pub struct VisibilityMask(u32);

impl VisibilityMask {
    pub const ALL: VisibilityMask = VisibilityMask(!0);

    #[inline]
    pub fn set(&mut self, view_index: usize, value: bool) {
        if value {
            self.0 |= 1 << view_index;
        } else {
            self.0 &= !(1 << view_index);
        }
    }

    #[inline]
    pub fn set_or(&mut self, view_index: usize, value: bool) {
        if value {
            self.0 |= 1 << view_index;
        }
    }

    #[inline]
    pub fn get(&self, view_index: usize) -> bool {
        self.0 & (1 << view_index) != 0
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}
