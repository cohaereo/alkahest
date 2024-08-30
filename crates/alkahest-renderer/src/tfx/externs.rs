use std::{fmt::Debug, ptr::null_mut};

use binrw::binread;
use field_access::FieldAccess;
use glam::{Mat3, Mat4, Quat, Vec4};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use strum::EnumIter;
use windows::{core::Interface, Win32::Graphics::Direct3D11::ID3D11ShaderResourceView};

use super::channels::{ChannelType, GlobalChannel};
use crate::{camera::Viewport, util::short_type_name};

#[derive(Default, Clone)]
pub enum TextureView {
    #[default]
    Null,
    /// Used for internal textures such as gbuffers
    RawSRV(ID3D11ShaderResourceView),
    // Tracked(WeakHandle<Texture>),
}

pub type UnorderedAccessView = TextureView;

impl TextureView {
    pub fn view(&self) -> Option<ID3D11ShaderResourceView> {
        match self {
            TextureView::Null => None,
            TextureView::RawSRV(v) => Some(v.clone()),
            // TextureView::Tracked(t) => t
            //     .upgrade()
            //     .and_then(|t| am.textures.get(&t).map(|t| t.view.clone())),
        }
    }

    pub fn view_unchecked(&self) -> ID3D11ShaderResourceView {
        // cohae: Shoddy as fuck, causes brainaches
        self.view()
            .unwrap_or_else(|| unsafe { ID3D11ShaderResourceView::from_raw(null_mut()) })
    }

    pub fn is_null(&self) -> bool {
        matches!(self, TextureView::Null)
    }
}

impl Debug for TextureView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureView::Null => write!(f, "TextureView::Null"),
            TextureView::RawSRV(_) => write!(f, "TextureView::RawSRV"),
            // TextureView::Tracked(_) => write!(f, "TextureView::Tracked"),
        }
    }
}

impl From<ID3D11ShaderResourceView> for TextureView {
    fn from(v: ID3D11ShaderResourceView) -> Self {
        TextureView::RawSRV(v)
    }
}

// impl From<WeakHandle<Texture>> for TextureView {
//     fn from(t: WeakHandle<Texture>) -> Self {
//         TextureView::Tracked(t)
//     }
// }

pub struct ExternStorage {
    pub frame: Frame,
    pub view: Option<View>,
    pub deferred: Option<Deferred>,
    pub deferred_light: Option<DeferredLight>,
    pub deferred_shadow: Option<DeferredShadow>,
    pub transparent: Option<Transparent>,
    pub rigid_model: Option<RigidModel>,
    pub decal: Option<Decal>,
    pub simple_geometry: Option<SimpleGeometry>,
    pub atmosphere: Option<Atmosphere>,
    pub water: Option<Water>,
    pub hdao: Option<Hdao>,
    pub global_lighting: Option<GlobalLighting>,
    pub cubemaps: Option<Cubemaps>,
    pub speedtree_placements: Option<SpeedtreePlacements>,
    pub decorator_wind: Option<DecoratorWind>,
    pub postprocess: Option<Postprocess>,
    pub shadow_mask: Option<ShadowMask>,

    pub global_channels: [GlobalChannel; 256],
    pub global_channels_used: RwLock<[usize; 256]>,

    pub errors: RwLock<FxHashMap<String, TfxExpressionError>>,
}

impl Default for ExternStorage {
    fn default() -> Self {
        Self {
            frame: Frame::default(),
            view: None,
            deferred: None,
            deferred_light: None,
            deferred_shadow: None,
            transparent: None,
            rigid_model: None,
            decal: None,
            simple_geometry: None,
            atmosphere: None,
            water: None,
            hdao: None,
            global_lighting: None,
            cubemaps: None,
            speedtree_placements: None,
            decorator_wind: Some(DecoratorWind::default()),
            postprocess: None,
            shadow_mask: None,

            global_channels: get_global_channel_defaults(),
            global_channels_used: RwLock::new([0; 256]),

            errors: RwLock::new(FxHashMap::default()),
        }
    }
}

impl ExternStorage {
    pub fn get_value_or_default<T: Sized + Default + 'static>(
        &self,
        ext: TfxExtern,
        offset: usize,
    ) -> T {
        self.get_value(ext, offset).unwrap_or_default()
    }

    pub fn get_value<T: Sized + 'static>(
        &self,
        ext: TfxExtern,
        offset: usize,
    ) -> anyhow::Result<T> {
        match self.get_value_inner::<T>(ext, offset) {
            ExternValue::Value(v) => Ok(v),
            ExternValue::Unimplemented(v) => {
                self.errors
                    .write()
                    .entry(format!(
                        "Extern field {ext:?}@0x{offset:X} is unimplemented (type {})",
                        short_type_name::<T>()
                    ))
                    .or_insert_with(|| TfxExpressionError {
                        error_type: TfxExpressionErrorType::Unimplemented {
                            field_offset: offset,
                        },
                        repeat_count: 0,
                        // occurences: FxHashSet::default(),
                    })
                    .repeat_count += 1;

                Ok(v)
            }

            ExternValue::InvalidType(t) => {
                self.errors
                    .write()
                    .entry(format!(
                        "Extern field {ext:?}@0x{offset:X} has invalid type (expected {})",
                        short_type_name::<T>()
                    ))
                    .or_insert_with(|| TfxExpressionError {
                        error_type: TfxExpressionErrorType::InvalidType(t),
                        repeat_count: 0,
                        // occurences: FxHashSet::default(),
                    })
                    .repeat_count += 1;

                Err(anyhow::anyhow!("Invalid type: {ext:?}@0x{offset:X}"))
            }
            ExternValue::FieldNotFound => {
                self.errors
                    .write()
                    .entry(format!(
                        "Extern field @ 0x{offset:X} for {ext:?} not found (type {})",
                        short_type_name::<T>()
                    ))
                    .or_insert_with(|| TfxExpressionError {
                        error_type: TfxExpressionErrorType::Unimplemented {
                            field_offset: offset,
                        },
                        repeat_count: 0,
                        // occurences: FxHashSet::default(),
                    })
                    .repeat_count += 1;

                Err(anyhow::anyhow!("Field not found: {ext:?}@0x{offset:X}"))
            }
            ExternValue::ExternNotFound => {
                self.errors
                    .write()
                    .entry(format!("Extern {ext:?} not found"))
                    .or_insert_with(|| TfxExpressionError {
                        error_type: TfxExpressionErrorType::ExternNotSet("Extern not found"),
                        repeat_count: 0,
                        // occurences: FxHashSet::default(),
                    })
                    .repeat_count += 1;

                Err(anyhow::anyhow!("Extern not found: {ext:?}"))
            }
            ExternValue::ExternNotSet => {
                self.errors
                    .write()
                    .entry(format!("Extern {ext:?} not set"))
                    .or_insert_with(|| TfxExpressionError {
                        error_type: TfxExpressionErrorType::ExternNotSet("Extern not set"),
                        repeat_count: 0,
                        // occurences: FxHashSet::default(),
                    })
                    .repeat_count += 1;

                Err(anyhow::anyhow!("Extern not set: {ext:?}"))
            }
        }
    }

    fn get_value_inner<T: Sized + 'static>(&self, ext: TfxExtern, offset: usize) -> ExternValue<T> {
        macro_rules! extern_lookup {
            ($(
                $ext:ident => $field:expr,
            )*) => {
                match ext {
                    TfxExtern::Frame => self.frame.get_field(offset),
                    $(
                        TfxExtern::$ext => $field.as_ref().map(|v| v.get_field(offset)).unwrap_or_else(|| ExternValue::ExternNotSet),
                    )*
                    _ => {
                        ExternValue::ExternNotFound
                    },
                }
            };
        }

        extern_lookup! {
            View => self.view,
            Deferred => self.deferred,
            DeferredLight => self.deferred_light,
            DeferredShadow => self.deferred_shadow,
            Transparent => self.transparent,
            RigidModel => self.rigid_model,
            Decal => self.decal,
            SimpleGeometry => self.simple_geometry,
            Atmosphere => self.atmosphere,
            Water => self.water,
            Hdao => self.hdao,
            GlobalLighting => self.global_lighting,
            Cubemaps => self.cubemaps,
            SpeedtreePlacements => self.speedtree_placements,
            DecoratorWind => self.decorator_wind,
            Postprocess => self.postprocess,
            ShadowMask => self.shadow_mask,
        }
    }

    pub fn get_field_path(ext: TfxExtern, offset: usize) -> Option<String> {
        macro_rules! extern_lookup {
            ($(
                $field:ident
            ),*) => {
                match ext {
                    $(
                        TfxExtern::$field => <$field as Extern>::get_field_name(offset).map(|f| format!("{}->{f}", <$field as Extern>::get_name())),
                    )*
                    _ => {
                        None
                    },
                }
            };
        }

        extern_lookup! {
            Frame,
            View,
            Deferred,
            DeferredLight,
            DeferredShadow,
            Transparent,
            RigidModel,
            Decal,
            SimpleGeometry,
            Atmosphere,
            Water,
            Hdao,
            GlobalLighting,
            Cubemaps,
            SpeedtreePlacements,
            DecoratorWind,
            Postprocess,
            ShadowMask
        }
    }

    pub fn get_extern_editable(&mut self, ext: TfxExtern) -> Option<&mut dyn FieldAccess> {
        macro_rules! extern_lookup {
            ($(
                $ext:ident => $field:expr,
            )*) => {
                match ext {
                    TfxExtern::Frame => Some(&mut self.frame as &mut dyn FieldAccess),
                    $(
                        TfxExtern::$ext => $field.as_mut().map(|f| f as &mut dyn FieldAccess),
                    )*
                    _ => None,
                }
            };
        }

        extern_lookup! {
            View => self.view,
            Deferred => self.deferred,
            DeferredLight => self.deferred_light,
            DeferredShadow => self.deferred_shadow,
            Transparent => self.transparent,
            RigidModel => self.rigid_model,
            Decal => self.decal,
            SimpleGeometry => self.simple_geometry,
            Atmosphere => self.atmosphere,
            Water => self.water,
            Hdao => self.hdao,
            GlobalLighting => self.global_lighting,
            Cubemaps => self.cubemaps,
            SpeedtreePlacements => self.speedtree_placements,
            DecoratorWind => self.decorator_wind,
            Postprocess => self.postprocess,
            ShadowMask => self.shadow_mask,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ExternValue<T> {
    Value(T),
    Unimplemented(T),
    InvalidType(&'static str),
    FieldNotFound,

    ExternNotFound,
    ExternNotSet,
}

#[allow(clippy::missing_safety_doc)]
pub trait Extern {
    fn get_name() -> &'static str;

    fn get_field<T: Sized + 'static>(&self, offset: usize) -> ExternValue<T>;

    fn get_field_name(offset: usize) -> Option<&'static str>;
}

/*
## Syntax

struct Name("internal_name") {
    0xOFFSET => field_name: field_type,
    // Optionally, you can mark a field as unimplemented
    0xOFFSET => field_name: field_type > unimplemented(true),
    // Marking the field as unimplemented(false) will still return the value
    0xOFFSET => field_name: field_type > unimplemented(true),
}
*/

macro_rules! extern_struct {
    (struct $name:ident ($name_c:literal) { $($field_offset:expr => $field:ident: $field_type:ty  $(> unimplemented($unimp:expr))? $(> default($default_value:expr))? ,)* }) => {
        #[repr(C)]
        #[derive(Debug, Clone, FieldAccess)]
        pub struct $name {
            $(pub $field: $field_type,)*
        }

        impl Extern for $name {
            fn get_name() -> &'static str {
                $name_c
            }

            fn get_field<T: Sized + 'static>(&self, offset: usize) -> ExternValue<T> {
                let ptr = self as *const _ as *const u8;

                match offset {
                    $($field_offset => {
                        if std::any::TypeId::of::<T>() == remap_quat_to_vec4(std::any::TypeId::of::<$field_type>()) {
                            unsafe {
                                let ptr = ptr.add(std::mem::offset_of!(Self, $field)) as *const T;

                                if false $(|| $unimp)* {
                                    return ExternValue::Unimplemented(ptr.read());
                                } else {
                                    ExternValue::Value(ptr.read())
                                }
                            }
                        } else {
                            ExternValue::InvalidType(concat!(stringify!($field), ": ", stringify!($field_type)))
                        }
                    })*
                    _ => ExternValue::FieldNotFound
                }
            }

            fn get_field_name(offset: usize) -> Option<&'static str> {
                match offset {
                    $($field_offset => Some(stringify!($field)),)*
                    _ => None
                }
            }
        }

        impl ExternDefault for $name {
            fn extern_default() -> Self {
                Self {
                    $($field: $(if true { $default_value } else )* {
                        <$field_type as ExternDefault>::extern_default()
                    },)*
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::extern_default()
            }
        }
    };

}

fn remap_quat_to_vec4(type_id: std::any::TypeId) -> std::any::TypeId {
    if type_id == std::any::TypeId::of::<Quat>() {
        std::any::TypeId::of::<Vec4>()
    } else {
        type_id
    }
}

extern_struct! {
    struct Frame("frame") {
        0x00 => game_time: f32,
        0x04 => render_time: f32,
        0x0c => unk0c: f32 > unimplemented(true),
        0x10 => unk10: f32 > unimplemented(true),
        0x14 => delta_game_time: f32 > unimplemented(true),
        0x18 => exposure_time: f32 > unimplemented(true),
        0x1c => exposure_scale: f32 > unimplemented(false),
        0x20 => unk20: f32 > unimplemented(true),
        0x24 => unk24: f32 > unimplemented(true),
        0x28 => exposure_illum_relative: f32 > unimplemented(true),
        0x2c => unk2c: f32 > unimplemented(true),
        0x40 => unk40: f32 > unimplemented(true),
        0x70 => unk70: f32 > unimplemented(true),
        0x78 => unk78: TextureView > unimplemented(true),
        0x80 => unk80: TextureView > unimplemented(true),
        0x88 => unk88: TextureView > unimplemented(true),
        0x90 => unk90: TextureView > unimplemented(true),
        0x98 => unk98: TextureView > unimplemented(true),
        0xa0 => unka0: TextureView > unimplemented(true),
        0xa8 => specular_lobe_lookup: TextureView,
        0xb0 => specular_lobe_3d_lookup: TextureView,
        0xb8 => specular_tint_lookup: TextureView,
        0xc0 => iridescence_lookup: TextureView,
        0xd0 => unkd0: Vec4 > unimplemented(true),
        0x150 => unk150: Vec4 > unimplemented(true),
        0x160 => unk160: Vec4 > unimplemented(true),
        0x170 => unk170: Vec4 > unimplemented(true),
        0x180 => unk180: Vec4 > unimplemented(true),
        0x190 => unk190: f32 > unimplemented(true),
        0x194 => unk194: f32 > unimplemented(true),
        // When not zero, causes a weird noise pattern on cutout textures
        0x1a0 => unk1a0: Vec4 > unimplemented(false) > default(Vec4::ZERO),
        0x1b0 => unk1b0: Vec4 > unimplemented(false),
        0x1c0 => unk1c0: Vec4 > unimplemented(false) > default(Vec4::new(1.0, 1.0, 0.0, 1.0)),
        0x1e0 => unk1e0: TextureView > unimplemented(true),
        0x1e8 => unk1e8: TextureView > unimplemented(true),
        0x1f0 => unk1f0: TextureView > unimplemented(true),
    }
}

extern_struct! {
    struct View("view") {
        0x00 => resolution_width: f32,
        0x04 => resolution_height: f32,
        0x10 => view_miscellaneous: Vec4 > unimplemented(false),
        0x20 => position: Vec4,
        // TODO(cohae): Used for shadow generation it seems
        0x30 => unk30: Vec4 > unimplemented(false),
        0x40 => world_to_camera: Mat4,
        0x80 => camera_to_projective: Mat4,
        0xc0 => camera_to_world: Mat4,
        0x100 => projective_to_camera: Mat4,
        0x140 => world_to_projective: Mat4,
        0x180 => projective_to_world: Mat4,
        0x1c0 => target_pixel_to_world: Mat4,
        0x200 => target_pixel_to_camera: Mat4,
        0x240 => unk240: Mat4 > unimplemented(true),
        0x280 => tptow_no_proj_w: Mat4,
        0x2c0 => unk2c0: Mat4 > unimplemented(true),
    }
}

impl View {
    /// Derives matrices based on world_to_camera, camera_to_projective and viewport
    pub fn derive_matrices(&mut self, viewport: &Viewport) {
        self.resolution_width = viewport.size.x as f32;
        self.resolution_height = viewport.size.y as f32;

        self.camera_to_world = self.world_to_camera.inverse();
        self.world_to_projective = self.camera_to_projective * self.world_to_camera;
        self.projective_to_world = self.world_to_projective.inverse();
        self.projective_to_camera = self.camera_to_projective.inverse();
        self.target_pixel_to_camera =
            self.projective_to_camera * viewport.target_pixel_to_projective();
        self.target_pixel_to_world = self.camera_to_world * self.target_pixel_to_camera;

        self.position = self.camera_to_world.w_axis;
        self.unk30 = Vec4::Z - self.world_to_projective.w_axis;

        let ptow_no_proj_w = {
            let ptoc = self.projective_to_camera;
            let ctow = self.camera_to_world;
            let ctow_mat3 = Mat3::from_mat4(ctow);
            let ctow = Mat4::from_mat3(ctow_mat3);

            ctow * ptoc
        };

        self.tptow_no_proj_w = ptow_no_proj_w * viewport.target_pixel_to_projective();
    }
}

extern_struct! {
    struct Deferred("deferred") {
        0x00 => depth_constants: Vec4 > default(Vec4::new(0.0, 1. / 0.01, 0.0, 0.0)),
        0x10 => unk10: Vec4 > unimplemented(true),
        0x20 => unk20: Vec4 > unimplemented(true),
        0x30 => unk30: f32 > unimplemented(true),
        0x38 => deferred_depth: TextureView,
        0x48 => deferred_rt0: TextureView,
        0x50 => deferred_rt1: TextureView,
        0x58 => deferred_rt2: TextureView,
        0x60 => light_diffuse: TextureView,
        0x68 => light_specular: TextureView,
        0x70 => light_ibl_specular: TextureView,
        0x78 => unk78: TextureView > unimplemented(true),
        0x80 => unk80: TextureView > unimplemented(true),
        0x88 => unk88: TextureView > unimplemented(true),
        0x90 => unk90: TextureView > unimplemented(true),
        0x98 => sky_hemisphere_mips: TextureView,
    }
}

extern_struct! {
    struct DeferredLight("deferred_light") {
        0x40 => unk40: Mat4 > unimplemented(false),
        0x80 => unk80: Mat4 > unimplemented(true),
        0xc0 => unkc0: Vec4 > unimplemented(true) > default(Vec4::W),
        0xd0 => unkd0: Vec4 > unimplemented(true) > default(Vec4::W),
        0xe0 => unke0: Vec4 > unimplemented(true) > default(Vec4::W),
        0xf0 => unkf0: Vec4 > unimplemented(true) > default(Vec4::W),
        0x100 => unk100: Vec4,
        0x110 => unk110: f32 > unimplemented(true),
        0x114 => unk114: f32 > unimplemented(true) > default(7500.0),
        0x118 => unk118: f32 > unimplemented(true),
        0x11c => unk11c: f32 > unimplemented(true),
        0x120 => unk120: f32 > unimplemented(true),
    }
}

extern_struct! {
    struct DeferredShadow("deferred_shadow") {
        0x00 => unk00: TextureView,
        0x08 => unk08: TextureView > unimplemented(true),
        0x10 => unk10: TextureView > unimplemented(true),
        0x18 => resolution_width: f32,
        0x1c => resolution_height: f32,
        0x20 => unk20: f32 > unimplemented(true),
        0x28 => unk28: TextureView > unimplemented(true),
        0x30 => unk30: Vec4 > unimplemented(true) > default(Vec4::new(1.5, 1.0, 1.0, 1.0)),
        0x40 => unk40: Vec4 > unimplemented(true),
        0x50 => unk50: Vec4 > unimplemented(true),
        0x80 => unk80: Vec4 > unimplemented(true),
        0x90 => unk90: Vec4 > unimplemented(true),
        0xa0 => unka0: Vec4 > unimplemented(true),
        0xb0 => unkb0: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 0.0, 1.0, 1.0)),
        0xc0 => unkc0: Mat4,
        0x100 => unk100: Mat4 > unimplemented(true),
        0x180 => unk180: f32 > unimplemented(true),
    }
}

extern_struct! {
    struct Transparent("transparent") {
        0x00 => unk00: TextureView > unimplemented(false), // t11 | atmos_ss_far_lookup(_low_res)
        0x08 => unk08: TextureView > unimplemented(false), // t12 | atmos_ss_far_lookup_downsampled
        0x10 => unk10: TextureView > unimplemented(false), // t13 | atmos_ss_near_lookup(_low_res)
        0x18 => unk18: TextureView > unimplemented(false), // t14 | atmos_ss_near_lookup_downsampled
        0x20 => unk20: TextureView > unimplemented(false), // t15 | surf_atmosphere_depth_angle_density_lookup
        0x28 => unk28: TextureView > unimplemented(false), // t16 | field456_0x200 (Texture3D)
        0x30 => unk30: TextureView > unimplemented(false), // t17 | field457_0x204 (Texture3D)
        0x38 => unk38: TextureView > unimplemented(false), // t18 | field458_0x208 (Texture3D)
        0x40 => unk40: TextureView > unimplemented(false), // t19 | light_grid_shadow_final Texture3D
        0x48 => unk48: TextureView > unimplemented(false), // t20 | surf_volumetrics_surface0/surf_volumetrics_result Texture2D
        0x50 => unk50: TextureView > unimplemented(false), // t21 | surf_volumetrics_result_intensity_3d
        0x58 => unk58: TextureView > unimplemented(false), // t22 | field491_0x238
        0x60 => unk60: TextureView > unimplemented(false), // t23 | surf_shading_result_read
        0x70 => unk70: Vec4 > unimplemented(true),
        0x80 => unk80: Vec4 > unimplemented(true),
        0x90 => unk90: Vec4 > unimplemented(true),
        0xa0 => unka0: Vec4 > unimplemented(true),
        0xb0 => unkb0: Vec4 > unimplemented(true),
    }
}

extern_struct! {
    struct Atmosphere("atmosphere") {
        0x00 => unk00: TextureView > unimplemented(true),
        0x08 => unk08: TextureView > unimplemented(true),
        0x10 => unk10: TextureView > unimplemented(true),
        0x18 => unk18: TextureView > unimplemented(true),

        // cohae: Everything below got shifted by 0x20 bytes in TFS
        0x40 => unk40: TextureView > unimplemented(true), // lightfall = 0x20
        0x58 => unk58: TextureView > unimplemented(true), // lightfall = 0x38
        // Normalized time of day value, where 0 is midnight, 0.5 is midday, and 1 is midnight again
        0x70 => time_of_day_normalized: f32 > default(0.5),
        0x74 => unk74: f32 > unimplemented(true),
        0x78 => unk78: f32 > unimplemented(true),
        0x80 => unk80: TextureView > unimplemented(true),
        0x88 => unk88: TextureView > unimplemented(true),
        0x90 => unk90: Vec4 > unimplemented(true),
        0xa0 => light_shaft_optical_depth: TextureView > unimplemented(true),
        0xc0 => unkc0: TextureView > unimplemented(true),
        0xd0 => unkd0: Vec4 > unimplemented(true),

        // Result of sky_lookup_generate_far
        0xe0 => atmos_ss_far_lookup: TextureView, // lightfall = 0xc0
        0xe8 => atmos_ss_far_lookup_downsampled: TextureView > unimplemented(true),
        // Result of sky_lookup_generate_near
        0xf0 => atmos_ss_near_lookup: TextureView, // lightfall = 0xd0
        0xf8 => atmos_ss_near_lookup_downsampled: TextureView > unimplemented(true),

        0x100 => unk100: TextureView > unimplemented(true),
        0x110 => unk110: Vec4 > unimplemented(true) > default(Vec4::Z * -1.5),
        0x140 => fog_color: Vec4 > unimplemented(true), // lightfall = 0x120
        0x150 => unk150: f32 > unimplemented(true),
        0x154 => unk154: f32 > unimplemented(true),
        0x160 => fog_intensity: f32 > unimplemented(true),
        0x164 => unk164: f32 > unimplemented(true),
        0x168 => unk168: f32 > unimplemented(true),
        0x16c => unk16c: f32 > unimplemented(true),
        0x170 => unk170: f32 > unimplemented(true) > default(0.0001),
        0x180 => unk180: Vec4 > unimplemented(true),
        0x190 => unk190: f32 > unimplemented(true),
        0x194 => unk194: f32 > unimplemented(true),
        0x198 => unk198: f32 > unimplemented(true) > default(0.0001),
        // Atmosphere rotation
        0x1b4 => unk1b4_rotation: f32 > unimplemented(true) > default(0.0),
        // Intensity
        0x1b8 => unk1b8_intensity: f32 > unimplemented(true),
        // Some kind of cutoff
        0x1bc => unk1bc: f32 > unimplemented(true) > default(0.5),
        0x1c0 => unk1c0: f32 > unimplemented(true),
        0x1c4 => unk1c4: f32 > unimplemented(true),
        0x1d0 => unk1d0: Vec4 > unimplemented(true) > default(Vec4::ZERO),
        0x1e0 => unk1e0: f32 > unimplemented(true),
        0x1e4 => unk1e4: f32 > unimplemented(true),
        // Another cutoff value
        0x1e8 => unk1e8: f32 > unimplemented(true) > default(0.0),
        0x1ec => unk1ec: f32 > unimplemented(true),
        0x1f8 => unk1f8: f32 > unimplemented(true),
        0x1fc => unk1fc: f32 > unimplemented(true),
        0x208 => unk208: f32 > unimplemented(true),
        0x210 => unk210: Vec4 > unimplemented(true),
    }
}

extern_struct! {
    struct Water("water") {
        0x00 => unk00: TextureView > unimplemented(true),
        0x08 => unk08: TextureView > unimplemented(true),
        0x18 => unk18: TextureView > unimplemented(true),
        0x28 => unk28: TextureView > unimplemented(true),
        0x30 => unk30: TextureView > unimplemented(true),
        0x40 => unk40: Vec4 > unimplemented(true),
        0x50 => unk50: Vec4 > unimplemented(true),
        0x70 => unk70: f32 > unimplemented(true),
    }
}

extern_struct! {
    struct SimpleGeometry("simple_geometry") {
        0x00 => transform: Mat4,
    }
}

extern_struct! {
    struct Cubemaps("cubemaps") {
        0x00 => temp_ao: TextureView > unimplemented(true),
    }
}

extern_struct! {
    struct Decal("decal") {
        0x00 => unk00: TextureView > unimplemented(true),
        0x08 => unk08: TextureView, // rt1 copy
        0x10 => unk10: Vec4 > unimplemented(true),
        0x20 => unk20: Vec4 > unimplemented(true),
    }
}

extern_struct! {
    struct RigidModel("rigid_model") {
        0x00 => mesh_to_world: Mat4,
        0x40 => position_scale: Vec4,
        0x50 => position_offset: Vec4,
        0x60 => texcoord0_scale_offset: Vec4,
        0x70 => dynamic_sh_ao_values: Vec4,
    }
}

extern_struct! {
    struct Hdao("hdao") {
        0x00 => unk00: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 1. / 0.01, 0.0, 0.0)),
        0x10 => unk10: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 1. / 0.01, 0.0, 0.0)),
        0x20 => unk20: Vec4 > unimplemented(true),
        0x30 => unk30: Vec4 > unimplemented(true),
        0x40 => unk40: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 1. / 0.01, 0.0, 0.0)),
        0x50 => unk50: Vec4 > unimplemented(true),
        0x60 => unk60: TextureView,
        0x68 => unk68: TextureView,
        0x70 => unk70: Vec4 > unimplemented(true),
        0x80 => unk80: Vec4 > unimplemented(true),
        0x90 => unk90: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 1. / 0.01, 0.0, 0.0)),
        // 0xa0 => unka0: UnorderedAccessView,
    }
}

extern_struct! {
    struct GlobalLighting("global_lighting") {
        0x08 => unk08: TextureView > unimplemented(true),
        0x10 => unk10: Vec4 > unimplemented(true),
        // Specular light direction, as a 3 dimensional vector
        0x30 => unk30: Vec4 > unimplemented(true) > default(Vec4::new(1.0, -1.0, 1.0, 0.0)),
        // Diffuse light direction, as a 3 dimensional vector
        0x50 => unk50: Vec4 > unimplemented(true) > default(Vec4::new(1.0, -1.0, 1.0, 0.0)),
        0x70 => unk70: Vec4 > unimplemented(true),
        0x80 => unk80: Vec4 > unimplemented(true),
        0x90 => unk90: f32 > unimplemented(true),
        0x94 => unk94: f32 > unimplemented(true) > default(-0.5),
        0x98 => unk98: f32 > unimplemented(true),
        0x9c => unk9c: f32 > unimplemented(true),
        0xa0 => unka0: f32 > unimplemented(true),
        0xb0 => unkb0: Vec4 > unimplemented(true),
        0xc0 => unkc0: Vec4 > unimplemented(true),
        0xd0 => unkd0: Vec4 > unimplemented(true),
    }
}

extern_struct! {
    struct SpeedtreePlacements("speedtree_placements") {
        0x00 => unk00: Vec4 > unimplemented(true) > default(Vec4::ZERO),
        0x10 => unk10: Vec4 > unimplemented(true) > default(Vec4::W),
        0x20 => unk20: Vec4 > unimplemented(true),
        0x30 => unk30: Vec4 > unimplemented(true),
        0x40 => unk40: Vec4 > unimplemented(true),
        0x50 => unk50: Vec4 > unimplemented(true),
        0x60 => unk60: Vec4 > unimplemented(true),
        // cohae: zero = color, one = white???
        0x70 => unk70: Vec4 > unimplemented(true) > default(Vec4::ZERO),
    }
}

extern_struct! {
    struct DecoratorWind("decorator_wind") {
        0x00 => unk00: Vec4 > unimplemented(true) > default(Vec4::new(0.0, 0.0, 0.0, 0.01)),
    }
}

extern_struct! {
    struct Postprocess("postprocess") {
        0x00 => unk00: TextureView,
        0x08 => unk08: TextureView,
        0x10 => unk10: TextureView,
        0x18 => unk18: TextureView,
        0x20 => unk20: TextureView,
        0x28 => unk28: TextureView,
        0x30 => unk30: UnorderedAccessView,
        0x38 => unk38: UnorderedAccessView,
        0x40 => unk40: UnorderedAccessView,
        0x48 => unk48: UnorderedAccessView,
        0x50 => unk50: Vec4,
        0x60 => unk60: Vec4,
        0x80 => unk80: Vec4,
        0xc0 => unkc0: Vec4,
        0xd0 => unkd0: Vec4,
        0xe0 => unke0: Vec4,
        0xf0 => unkf0: Vec4,
        0x100 => unk100: Vec4,
        0x110 => unk110: Vec4,
        0x130 => unk130: Vec4,
    }
}

extern_struct! {
    struct ShadowMask("shadowmask") {
        0x00 => unk00: TextureView,
        0x08 => unk08: TextureView,
        0x10 => unk10: TextureView,
        0x20 => unk20: Vec4,
        0x30 => unk30: f32,
        0x34 => unk34: f32,
    }
}

#[test]
fn test_externs() {
    let deferred = Deferred {
        depth_constants: Vec4::new(1.0, 2.0, 3.0, 4.0),
        ..Default::default()
    };

    assert_eq!(
        deferred.get_field::<Vec4>(0x20),
        ExternValue::Unimplemented(Deferred::default().unk20)
    );

    let view = View {
        resolution_width: 1920.0,
        resolution_height: 1080.0,
        ..Default::default()
    };

    assert_eq!(view.get_field::<f32>(0x00), ExternValue::Value(1920.0));
    assert_eq!(view.get_field::<f32>(0x04), ExternValue::Value(1080.0));
}

#[derive(Clone, Copy, Debug, PartialEq, EnumIter, Hash, Eq)]
#[binread]
#[br(repr(u8))]
pub enum TfxExtern {
    None = 0,
    Frame = 1,
    View = 2,
    Deferred = 3,
    DeferredLight = 4,
    DeferredUberLight = 5,
    DeferredShadow = 6,
    Atmosphere = 7,
    RigidModel = 8,
    EditorMesh = 9,
    EditorMeshMaterial = 10,
    EditorDecal = 11,
    EditorTerrain = 12,
    EditorTerrainPatch = 13,
    EditorTerrainDebug = 14,
    SimpleGeometry = 15,
    UiFont = 16,
    CuiView = 17,
    CuiObject = 18,
    CuiBitmap = 19,
    CuiVideo = 20,
    CuiStandard = 21,
    CuiHud = 22,
    CuiScreenspaceBoxes = 23,
    TextureVisualizer = 24,
    Generic = 25,
    Particle = 26,
    ParticleDebug = 27,
    GearDyeVisualizationMode = 28,
    ScreenArea = 29,
    Mlaa = 30,
    Msaa = 31,
    Hdao = 32,
    DownsampleTextureGeneric = 33,
    DownsampleDepth = 34,
    Ssao = 35,
    VolumetricObscurance = 36,
    Postprocess = 37,
    TextureSet = 38,
    Transparent = 39,
    Vignette = 40,
    GlobalLighting = 41,
    ShadowMask = 42,
    ObjectEffect = 43,
    Decal = 44,
    DecalSetTransform = 45,
    DynamicDecal = 46,
    DecoratorWind = 47,
    TextureCameraLighting = 48,
    VolumeFog = 49,
    Fxaa = 50,
    Smaa = 51,
    Letterbox = 52,
    DepthOfField = 53,
    PostprocessInitialDownsample = 54,
    CopyDepth = 55,
    DisplacementMotionBlur = 56,
    DebugShader = 57,
    MinmaxDepth = 58,
    SdsmBiasAndScale = 59,
    SdsmBiasAndScaleTextures = 60,
    ComputeShadowMapData = 61,
    ComputeLocalLightShadowMapData = 62,
    BilateralUpsample = 63,
    HealthOverlay = 64,
    LightProbeDominantLight = 65,
    LightProbeLightInstance = 66,
    Water = 67,
    LensFlare = 68,
    ScreenShader = 69,
    Scaler = 70,
    GammaControl = 71,
    SpeedtreePlacements = 72,
    Reticle = 73,
    Distortion = 74,
    WaterDebug = 75,
    ScreenAreaInput = 76,
    WaterDepthPrepass = 77,
    OverheadVisibilityMap = 78,
    ParticleCompute = 79,
    CubemapFiltering = 80,
    ParticleFastpath = 81,
    VolumetricsPass = 82,
    TemporalReprojection = 83,
    FxaaCompute = 84,
    VbCopyCompute = 85,
    UberDepth = 86,
    GearDye = 87,
    Cubemaps = 88,
    ShadowBlendWithPrevious = 89,
    DebugShadingOutput = 90,
    Ssao3d = 91,
    WaterDisplacement = 92,
    PatternBlending = 93,
    UiHdrTransform = 94,
    PlayerCenteredCascadedGrid = 95,
    SoftDeform = 96,
}

pub struct TfxExpressionError {
    pub error_type: TfxExpressionErrorType,
    pub repeat_count: usize,
    // pub occurences: FxHashSet<TagHash>,
}

pub enum TfxExpressionErrorType {
    Unimplemented { field_offset: usize },
    InvalidType(&'static str),
    ExternNotSet(&'static str),
}

pub trait ExternDefault {
    fn extern_default() -> Self;
}

impl ExternDefault for TextureView {
    fn extern_default() -> Self {
        TextureView::Null
    }
}

impl ExternDefault for Vec4 {
    fn extern_default() -> Self {
        Vec4::ONE
    }
}

impl ExternDefault for Quat {
    fn extern_default() -> Self {
        Quat::IDENTITY
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

#[rustfmt::skip]
fn get_global_channel_defaults() -> [GlobalChannel; 256] {
    let mut channels: [GlobalChannel; 256] = core::array::from_fn(|_| GlobalChannel::default());

    channels[10].value = Vec4::ZERO;
    channels[97].value = Vec4::ZERO;

    channels[75] = GlobalChannel::new("unk75 (verity dark/light)", ChannelType::FloatSlider(0f32..=1f32), Vec4::ZERO);
    channels[76] = GlobalChannel::new("unk76 (verity dark/light, cancels out unk75)", ChannelType::FloatSlider(0f32..=1f32), Vec4::ZERO);

    // Sun related
    channels[82].value = Vec4::ZERO;
    channels[83].value = Vec4::ZERO;
    channels[98].value = Vec4::ZERO;
    channels[100].value = Vec4::ZERO;

    channels[27] = GlobalChannel::new("global specular intensity", ChannelType::Float, Vec4::ONE);
    channels[28] = GlobalChannel::new("global specular tint", ChannelType::Color, Vec4::ONE);

    channels[31] = GlobalChannel::new("global diffuse direct tint", ChannelType::Color, Vec4::ONE);
    channels[32] = GlobalChannel::new("global diffuse direct intensity", ChannelType::Float, Vec4::ONE);
    channels[33] = GlobalChannel::new("global diffuse penumbra tint", ChannelType::Color, Vec4::ONE);
    channels[34] = GlobalChannel::new("global diffuse penumbra intensity", ChannelType::Float, Vec4::ONE);

    channels[37] = GlobalChannel::new("fog start", ChannelType::Float, Vec4::X * 50.0);
    channels[41] = GlobalChannel::new("fog falloff", ChannelType::Float, Vec4::X * 50.0);

    // Misc lights
    channels[84] = GlobalChannel::new("ao intensity", ChannelType::Float, Vec4::ONE);

    channels[93].value = Vec4::new(1.0, 0.0, 0.0, 0.0);
    channels[113].value = Vec4::ZERO;
    channels[127].value = Vec4::ZERO;

    // TODO(cohae): this channel is a bit confusing. There is no 1 magic universal value, some lights work, some environments need different values
    channels[131].value = Vec4::new(0.5, 0.5, 0.3, 0.0); // Seems related to line lights

    channels
}
