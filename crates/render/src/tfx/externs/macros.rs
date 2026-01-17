macro_rules! extern_container {
    ($($name:ident: $t:ident),*) => {
        /// Container holding all externs and global channels used in the renderer.
        pub struct Externs {
            $(
            pub $name: Box<$t>,
            )*

            pub default_globals: [Vec4; 256],

            pub globals: [Vec4; 256],
            pub global_ids: Vec<u32>,
            pub unk_sequencer_values: [Vec4; 256],
        }

        impl Externs {
            // pub fn get_extern_value<T: Sized + Clone + 'static>(
            //     &self,
            //     index: ExternIndex,
            //     offset: usize,
            // ) -> Option<&T> {
            //     match index {
            //         $(
            //             ExternIndex::$t => self.$name.get_field(offset),
            //         )*
            //         _ => None,
            //     }
            // }

            pub fn get_extern_field_name(index: ExternIndex, offset: usize) -> Option<&'static str> {
                match index {
                    $(
                        ExternIndex::$t => $t::get_field_name(offset),
                    )*
                    _ => None,
                }
            }
        }

        impl ExternAccessor for Externs {
            fn get_value_ptr(&self, index: ExternIndex, offset: usize) -> Option<(*const (), TypeId)> {
                match index {
                    $(
                        ExternIndex::$t => {
                            self.$name.get_field_ptr(offset)
                        }
                    )*
                    _ => None,
                }
            }
        }

        impl Externs {
            pub fn new(globs: &RenderGlobals) -> Self {
                let mut r = {
                    let mut globals = [Vec4::ONE; 256];
                    globals[..globs.channels.default_values.len()].copy_from_slice(&globs.channels.default_values);

                    Self {
                        $(
                            $name: Default::default(),
                        )*
                        default_globals: globals,
                        globals,
                        global_ids: globs.channels.channel_ids.clone(),
                        unk_sequencer_values: [Vec4::ZERO; 256],
                    }
                };

                r.set_global_channel_by_id(0x2C538179, Vec4::splat(0.1));

                r.set_global_channel_by_id(777225282, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225283, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225280, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225281, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225286, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225287, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225284, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225285, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225290, Vec4::splat(1.0));
                r.set_global_channel_by_id(777225291, Vec4::splat(1.0));

                r.set_global_channel_by_id(0x2E538441, Vec4::ZERO); // global_channels[68]
                r.set_global_channel_by_id(0x2C53817D, Vec4::ONE); // global_channels[46]
                r.set_global_channel_by_id(0x2C53817E, Vec4::ONE); // global_channels[47]

                r.set_global_channel_by_id(0xCF70AC7C, Vec4::new(1.5, 0.0, 0.0, 0.0)); // global_channels[82]
                r.set_global_channel_by_id(0xCF70AC7C, Vec4::ZERO); // global_channels[138]

                r.default_globals = r.globals.clone();

                r
            }
        }
    };
}

macro_rules! local_extern_container {
    ($($name:ident: $t:ident),*) => {
        /// Containers holding localized externs that allows for overriding externs for individual command lists
        #[derive(Default)]
        pub struct LocalExterns {
            $(
            pub $name: Option<Box<$t>>,
            )*
        }

        impl ExternAccessor for LocalExterns {
            fn get_value_ptr(&self, index: ExternIndex, offset: usize) -> Option<(*const (), TypeId)> {
                let base_externs = &crate::renderer::Renderer::instance().externs;
                match index {
                    $(
                        ExternIndex::$t => {
                            self.$name.as_ref().unwrap_or(&base_externs.$name).get_field_ptr(offset)
                        }
                    )*
                    _ => base_externs.get_value_ptr(index, offset),
                }
            }
        }
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
            fn get_field_ptr(&self, offset: usize) -> Option<(*const (), TypeId)> {
                let ptr = self as *const _ as *const u8;

                match offset {
                    $($field_offset => {
                        unsafe {
                            let ptr = ptr.add(std::mem::offset_of!(Self, $field));

                            Some((ptr as *const (), std::any::TypeId::of::<$field_type>()))
                        }
                    })*
                    _ => {
                        None
                    }
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

pub(super) use extern_container;
pub(super) use extern_struct;
pub(super) use local_extern_container;
