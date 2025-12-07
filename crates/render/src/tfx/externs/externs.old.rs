use std::{collections::HashMap, fmt::Debug};

use alkahest_data::tfx::ExternIndex;
use d3d11::dxgi;
use glam::{Mat3, Mat4, Vec3, Vec4, Vec4Swizzles};
use parking_lot::RwLock;

use crate::{
    asset::{
        texture::{Texture, TextureHandle},
        Handle,
    },
    renderer::surface::SurfaceHandle,
    Renderer,
};

macro_rules! externs {
    ($($name:ident: $t:ident),*) => {
        pub struct Externs {
            $(
            pub $name: $t,
            )*

            pub globals: [Vec4; 256],
            placeholder_textures:
                RwLock<HashMap<(ExternIndex, u32), (Texture, d3d11::UnorderedAccessView)>>,
        }

        impl Externs {
            pub fn get_extern_value<T: Sized + Clone + 'static>(
                &self,
                index: ExternIndex,
                offset: usize,
            ) -> Option<&T> {
                match index {
                    $(
                        ExternIndex::$t => self.$name.get_field(offset),
                    )*
                    _ => None,
                }
            }

            pub fn get_extern_field_name(index: ExternIndex, offset: usize) -> Option<&'static str> {
                match index {
                    $(
                        ExternIndex::$t => $t::get_field_name(offset),
                    )*
                    _ => None,
                }
            }
        }

        impl Default for Externs {
            fn default() -> Self {
                let globals = [Vec4::ONE; 256];
                // globals[124] = Vec4::splat(0.1);
                // globals[125] = Vec4::splat(0.5);
                // globals[128] = Vec4::splat(1.5);
                Externs {
                    $(
                        $name: Default::default(),
                    )*
                    placeholder_textures: Default::default(),
                    globals,
                }
            }
        }
    };
}

externs! {
    frame: Frame,
    view: View,
    overhead_visibility_map_main: OverheadVisibilityMapMain
}

#[profiling::all_functions]
impl Externs {
    pub fn get_texture_srv(&self, index: ExternIndex, offset: usize) -> d3d11::ShaderResourceView {
        let v: Option<&TextureView> = self.get_extern_value(index, offset);

        match v.and_then(|v| v.get_srv()) {
            Some(o) => o,
            None => self.get_placeholder_texture(index, offset).0,
        }
    }

    pub fn get_uav(&self, index: ExternIndex, offset: usize) -> d3d11::UnorderedAccessView {
        let v: Option<&Uav> = self.get_extern_value(index, offset);

        match v.and_then(|v| v.get_uav()) {
            Some(o) => o,
            None => self.get_placeholder_texture(index, offset).1,
        }
    }

    fn get_placeholder_texture(
        &self,
        index: ExternIndex,
        offset: usize,
    ) -> (d3d11::ShaderResourceView, d3d11::UnorderedAccessView) {
        // First try with a read lock
        if let Some((texture, uav)) = self
            .placeholder_textures
            .read()
            .get(&(index, offset as u32))
        {
            return (texture.view.clone(), uav.clone());
        }

        // If not found, upgrade to write lock
        let mut placeholder_textures = self.placeholder_textures.write();

        let (texture, uav) = placeholder_textures
            .entry((index, offset as u32))
            .or_insert_with(|| {
                let gpu = &Renderer::instance().gpu;
                let data = match (index, offset) {
                    (ExternIndex::Atmosphere, _) => bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4]),
                    (ExternIndex::Transparent, 0) => bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4]),
                    (ExternIndex::Water, 0x28) => bytemuck::cast_slice(&[[127u8, 127, 0, 0]; 4]), // RG16_UNORM
                    // VolumetricsPass inputs are generally the results of the last pass, alpha is *ALWAYS* cleared to zero
                    (ExternIndex::VolumetricsPass, _) => bytemuck::cast_slice(&[[0u8, 0, 0, 0]; 4]),
                    _ => bytemuck::cast_slice(&[[0u8, 0, 0, 255]; 4]),
                };
                assert_eq!(data.len(), (2 * 2) * 4);

                let texture = Texture::load_2d_raw(
                    gpu,
                    2,
                    2,
                    data,
                    dxgi::Format::R8g8b8a8Unorm,
                    Some(&format!("unknown extern {index:?}+0x{offset:X}")),
                    true,
                )
                .unwrap();

                let TextureHandle::Texture2D(tex) = &texture.handle else {
                    unreachable!()
                };
                let uav = gpu
                    .create_unordered_access_view(tex, None)
                    .expect("Failed to create uav for placeholder texture");

                (texture, uav)
            });

        (texture.view.clone(), uav.clone())
    }
}

trait Extern {
    fn get_name() -> &'static str;

    fn get_field<T: Sized + 'static>(&self, offset: usize) -> Option<&T>;
    fn get_field_name(offset: usize) -> Option<&'static str>;
}

#[derive(Default, Clone)]
pub enum TextureView {
    #[default]
    None,
    Surface(SurfaceHandle),
    Resource(Handle<Texture>),
    Raw(d3d11::ShaderResourceView),
}

impl TextureView {
    pub fn get_srv(&self) -> Option<d3d11::ShaderResourceView> {
        match self {
            TextureView::Surface(surface) => {
                Renderer::instance().surfaces().get(*surface).srv.clone()
            }
            TextureView::Resource(texture) => Some(texture.get()?.view.clone()),
            TextureView::Raw(srv) => Some(srv.clone()),
            TextureView::None => None,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, TextureView::None)
    }
}

impl Debug for TextureView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureView::None => write!(f, "None"),
            TextureView::Surface(surface) => write!(f, "Surface({surface:?})"),
            TextureView::Resource(texture) => write!(f, "Resource({})", texture.tag()),
            TextureView::Raw(_) => write!(f, "Raw"),
        }
    }
}

impl From<SurfaceHandle> for TextureView {
    fn from(surface: SurfaceHandle) -> Self {
        TextureView::Surface(surface)
    }
}

impl From<Handle<Texture>> for TextureView {
    fn from(texture: Handle<Texture>) -> Self {
        TextureView::Resource(texture)
    }
}

impl From<d3d11::ShaderResourceView> for TextureView {
    fn from(srv: d3d11::ShaderResourceView) -> Self {
        TextureView::Raw(srv)
    }
}

#[derive(Default, Clone)]
pub enum Uav {
    #[default]
    None,
    Surface(SurfaceHandle),
}

impl Debug for Uav {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Uav::None => write!(f, "None"),
            Uav::Surface(surface) => write!(f, "Surface({surface:?})"),
        }
    }
}

impl Uav {
    pub fn get_uav(&self) -> Option<d3d11::UnorderedAccessView> {
        match self {
            Uav::Surface(surface) => Renderer::instance().surfaces().get(*surface).uav.clone(),
            Uav::None => None,
        }
    }
}

impl From<SurfaceHandle> for Uav {
    fn from(surface: SurfaceHandle) -> Self {
        Uav::Surface(surface)
    }
}

macro_rules! extern_struct {
    (struct $name:ident ($name_c:literal) { $($field_offset:expr => $field:ident: $field_type:ty $(> default($default_value:expr))? ,)* }) => {
        #[repr(C)]
        #[derive(Clone, Debug)]
        pub struct $name {
            $(pub $field: $field_type,)*
        }

        impl Extern for $name {
            fn get_name() -> &'static str {
                $name_c
            }

            fn get_field<T: Sized + 'static>(&self, offset: usize) -> Option<&T> {
                let ptr = self as *const _ as *const u8;

                match offset {
                    $($field_offset => {
                        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<$field_type>() {
                            unsafe {
                                let ptr = ptr.add(std::mem::offset_of!(Self, $field)) as *const T;

                                Some(&*ptr)
                            }
                        } else {
                            None
                        }
                    })*
                    _ => None
                }
            }

            fn get_field_name(offset: usize) -> Option<&'static str> {
                match offset {
                    $($field_offset => Some(stringify!($field)),)*
                    _ => None
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $($field: $(if true { $default_value } else )* {
                        ExternDefault::extern_default()
                    },)*
                }
            }
        }

    };

}

extern_struct! {
    struct Frame("frame") {
        0x00 => game_time: f32,
        0x04 => render_time: f32,
        0x0C => unk0c: f32,
        0x10 => unk10: f32,
        0x14 => delta_game_time: f32,
        0x18 => exposure_time: f32,
        0x1C => exposure_scale: f32,
        0x20 => unk20: f32,
        0x24 => unk24: f32,
        0x28 => exposure_illum_relative: f32,
        0x2C => unk2c: f32,
        0x40 => unk40: f32,
        0x54 => unk54: f32,
        0x70 => unk70: f32 > default(0.5),
        0x78 => unk78: TextureView,
        0x80 => unk80: TextureView,
        0x88 => unk88: TextureView,
        0x90 => unk90: TextureView,
        0x98 => unk98: TextureView,
        0xA0 => unka0: TextureView,
        0xB0 => unkb0: TextureView,
        0xB8 => unkb8: TextureView,
        0xC0 => iridescence_lookup: TextureView,
        0xD0 => unkd0: Vec4,
        0x150 => unk150: Vec4,
        0x160 => unk160: Vec4,
        0x170 => unk170: Vec4,
        0x180 => unk180: Vec4,
        0x190 => unk190: Vec4,
        // When not zero, causes a weird noise pattern on cutout textures
        0x1A0 => unk1a0: Vec4 > default(Vec4::W),
        0x1B0 => unk1b0: Vec4 > default(Vec4::Z),
        0x1C0 => unk1c0: Vec4 > default(Vec4::ZERO),
    }
}

extern_struct! {
    struct View("view") {
        0x00 => target_width: f32,
        0x04 => target_height: f32,
        0x10 => unk10: Vec4 > default(Vec4::ZERO),
        0x20 => position: Vec4,
        0x30 => view_miscellaneous: Vec4 > default(Vec4::ZERO),
        // TODO(cohae): Used for shadow generation it seems
        0x40 => unk40: Vec4 > default(Vec4::ZERO),
        0x50 => unk50: Vec4,
        // TODO: the void here is added with Marathon, need to figure out what goes here

        0xA0 => camera_to_projective: Mat4,
        0xE0 => camera_to_world: Mat4,
        0x120 => target_pixel_to_camera: Mat4,
        0x160 => unk160: Mat4,
        0x1A0 => world_to_projective: Mat4,
        0x260 => unk260: Mat4,

        // Bungie fucked a good portion of this extern with Marathon, everything prefixed with 0x8_ here needs to be figured out again
        0x8_60 => world_to_camera: Mat4,
        // 0x8_A0 => camera_to_projective: Mat4,
        // 0x8_E0 => camera_to_world: Mat4,
        0x8_120 => projective_to_camera: Mat4,
        // 0x8_160 => world_to_projective: Mat4,
        0x8_1A0 => projective_to_world: Mat4,
        0x8_1E0 => target_pixel_to_world: Mat4,
        // 0x8_220 => target_pixel_to_camera: Mat4,
        0x8_260 => unk220: Mat4,
        0x8_2A0 => tptow_no_proj_w: Mat4,
        0x8_2E0 => unk2a0: Mat4,

        // Newly added to view scope with Marathon
        0x3E0 => unk3e0: Mat4,
        0x420 => unk420: Mat4,
        0x460 => unk460: Vec4,
    }
}

extern_struct! {
    struct OverheadVisibilityMapMain("overhead_visibility_map_main") {
        0x00 => unk00: TextureView,
        0x10 => unk10: Vec4,
        0x20 => unk20: Vec4,
    }
}

impl View {
    const VIEWSPACE_UNORM_TO_SNORM: Mat4 = Mat4::from_cols(
        Vec4::new(2.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, -2.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 0.0),
        Vec4::new(-1.0, 1.0, 0.0, 1.0),
    );

    pub fn update(
        &mut self,
        world_to_camera: Mat4,
        camera_to_projective: Mat4,
        view_resolution: (u32, u32),
    ) {
        self.world_to_camera = world_to_camera;
        self.camera_to_projective = camera_to_projective;
        self.derive_matrices(view_resolution);
    }

    /// Derives matrices based on world_to_camera, camera_to_projective and view resolution
    pub fn derive_matrices(&mut self, (view_width, view_height): (u32, u32)) {
        self.target_width = view_width as f32;
        self.target_height = view_height as f32;

        let target_pixel_to_projective = Self::VIEWSPACE_UNORM_TO_SNORM
            * Mat4::from_scale(Vec3::new(
                1. / view_width as f32,
                1. / view_height as f32,
                1.0,
            ));

        self.camera_to_world = self.world_to_camera.inverse();
        self.world_to_projective = self.camera_to_projective * self.world_to_camera;
        self.projective_to_world = self.world_to_projective.inverse();
        self.projective_to_camera = self.camera_to_projective.inverse();
        self.target_pixel_to_camera = self.projective_to_camera * target_pixel_to_projective;
        self.target_pixel_to_world = self.camera_to_world * self.target_pixel_to_camera;

        self.position = self.camera_to_world.w_axis;
        self.unk40 = Vec4::Z - self.world_to_projective.w_axis;

        self.unk220 = self.projective_to_world * Self::VIEWSPACE_UNORM_TO_SNORM;

        let ptow_no_proj_w = {
            let ptoc = self.projective_to_camera;
            let ctow = self.camera_to_world;
            let ctow_mat3 = Mat3::from_mat4(ctow);
            let ctow = Mat4::from_mat3(ctow_mat3);

            ctow * ptoc
        };

        self.tptow_no_proj_w = ptow_no_proj_w * target_pixel_to_projective;

        // TODO(cohae): Fairly sure this is the same as tptow_no_proj_w but for NDC instead of viewport space
        self.unk2a0 = ptow_no_proj_w * Self::VIEWSPACE_UNORM_TO_SNORM;
    }

    pub fn position(&self) -> Vec3 {
        self.position.xyz()
    }
}

pub trait ExternDefault {
    fn extern_default() -> Self;
}

impl ExternDefault for TextureView {
    fn extern_default() -> Self {
        TextureView::None
    }
}

impl ExternDefault for Uav {
    fn extern_default() -> Self {
        Uav::None
    }
}

impl ExternDefault for Vec4 {
    fn extern_default() -> Self {
        Vec4::ONE
    }
}

impl ExternDefault for Mat4 {
    fn extern_default() -> Self {
        Mat4::IDENTITY
    }
}

impl ExternDefault for f32 {
    fn extern_default() -> Self {
        1.0
    }
}
