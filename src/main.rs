#[macro_use]
extern crate windows;

use std::collections::HashMap;

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::mem::transmute;
use std::path::PathBuf;

use std::str::FromStr;
use std::time::Instant;

use crate::camera::{FpsCamera, InputState};
use crate::dxbc::{get_input_signature, DxbcHeader};
use crate::dxgi::calculate_pitch;

use crate::gui::COMPOSITOR_MODES;
use crate::scopes::{ScopeStaticInstance, ScopeView};
use crate::static_render::{LoadedTexture, StaticModel, TextureHandle};
use crate::statics::{Unk808071a7, Unk8080966d};
use crate::text::{decode_text, StringData, StringPart, StringSetHeader};
use crate::texture::TextureHeader;
use crate::types::DestinyHash;
use crate::vertex_layout::InputElement;
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::PackageVersion::Destiny2PreBeyondLight;
use destiny_pkg::{PackageManager, TagHash};
use glam::{Mat4, Quat, Vec3, Vec3Swizzles, Vec4};
use imgui::{Condition, FontConfig, FontSource, WindowFlags};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use nohash_hasher::IntMap;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use tracing::{debug, debug_span, error, info, info_span, trace, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3DCompileFromFile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION,
};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, VirtualKeyCode};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

mod camera;
mod dds;
mod dxbc;
mod dxgi;
mod entity;
mod gui;
mod material;
mod scopes;
mod static_render;
mod statics;
mod structure;
mod text;
mod texture;
mod types;
mod unknown;
mod vertex_layout;

pub fn get_string(
    pm: &mut PackageManager,
    (container, string): (TagHash, DestinyHash),
) -> Option<String> {
    if string.is_none() {
        return None;
    }

    let textset_header: StringSetHeader = pm.read_tag_struct(container).ok()?;
    // println!("{textset_header:#x?}");
    // println!("{} bytes text data in {i}", data.len());

    let data = pm.read_tag(textset_header.language_english).unwrap();
    let mut cur = Cursor::new(&data);
    let text_data: StringData = cur.read_le().ok()?;

    let (combination, _) = text_data
        .string_combinations
        .iter()
        .zip(textset_header.string_hashes.iter())
        .find(|(_, h)| **h == string)?;
    let mut final_string = String::new();

    for ip in 0..combination.part_count {
        cur.seek(combination.data.into()).unwrap();
        cur.seek(SeekFrom::Current(ip * 0x20)).unwrap();
        let part: StringPart = cur.read_le().unwrap();
        cur.seek(part.data.into()).unwrap();
        let mut data = vec![0u8; part.byte_length as usize];
        cur.read_exact(&mut data).unwrap();
        final_string += &decode_text(&data, part.cipher_shift);
    }

    Some(final_string)
}

// TODO(cohae): Put tracy features such as this behind a feature flag
// #[global_allocator]
// static GLOBAL: ProfiledAllocator<std::alloc::System> =
//     ProfiledAllocator::new(std::alloc::System, 100);

pub fn main() -> anyhow::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .unwrap();

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_tracy::TracyLayer::new())
            .with(tracing_subscriber::fmt::layer())
            .with(EnvFilter::from_default_env()),
    )
    .expect("Failed to set up the tracing subscriber");

    let (package, mut package_manager) =
        info_span!("Initializing package manager").in_scope(|| {
            let pkg_path = std::env::args().nth(1).unwrap_or_default();
            (
                Destiny2PreBeyondLight
                    .open(&pkg_path)
                    .expect("Failed to open package"),
                PackageManager::new(
                    PathBuf::from_str(&pkg_path).unwrap().parent().unwrap(),
                    Destiny2PreBeyondLight,
                    true,
                )
                .unwrap(),
            )
        });

    // return;
    // Winit event loop
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Alkahest")
        .with_inner_size(PhysicalSize::new(1600u32, 900u32))
        .build(&event_loop)?;

    let mut device: Option<ID3D11Device> = None;
    let mut swap_chain: Option<IDXGISwapChain> = None;
    let mut device_context: Option<ID3D11DeviceContext> = None;
    let swap_chain_description: DXGI_SWAP_CHAIN_DESC = {
        let buffer_descriptor = {
            let refresh_rate = DXGI_RATIONAL {
                Numerator: 0,
                Denominator: 0,
            };

            DXGI_MODE_DESC {
                Width: 0,
                Height: 0,
                RefreshRate: refresh_rate,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
            }
        };

        let sample_descriptor = DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        };

        DXGI_SWAP_CHAIN_DESC {
            BufferDesc: buffer_descriptor,
            SampleDesc: sample_descriptor,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT | DXGI_USAGE_SHADER_INPUT,
            BufferCount: 1,
            OutputWindow: match window.raw_window_handle() {
                RawWindowHandle::Win32(h) => unsafe { transmute(h.hwnd) },
                u => panic!("Can't open window for {u:?}"),
            },
            Windowed: BOOL(1),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        }
    };

    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HINSTANCE::default(),
            D3D11_CREATE_DEVICE_SINGLETHREADED, // | D3D11_CREATE_DEVICE_DEBUG,
            Some(&[D3D_FEATURE_LEVEL_11_1]),
            D3D11_SDK_VERSION,
            Some(&swap_chain_description),
            Some(&mut swap_chain),
            Some(&mut device),
            Some(&mut D3D_FEATURE_LEVEL_11_1),
            Some(&mut device_context),
        )?;
    }

    let device = device.unwrap();
    let device_context = device_context.unwrap();
    let swap_chain = swap_chain.unwrap();

    let mut swapchain_target = unsafe {
        let buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;
        Some(device.CreateRenderTargetView(&buffer, None)?)
    };

    let (rtv0, rtv0_view) = unsafe {
        let _buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;
        let tex = device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: window.inner_size().width as _,
                    Height: window.inner_size().height as _,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                None,
            )
            .context("Failed to create RT1 texture")?;

        (
            device.CreateRenderTargetView(&tex, None)?,
            device.CreateShaderResourceView(
                &tex,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
            )?,
        )
    };

    let (rtv1, rtv1_view) = unsafe {
        let tex = device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: window.inner_size().width as _,
                    Height: window.inner_size().height as _,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    // Format: DXGI_FORMAT_R10G10B10A2_TYPELESS,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                None,
            )
            .context("Failed to create RT1 texture")?;

        (
            device.CreateRenderTargetView(&tex, None)?,
            device.CreateShaderResourceView(
                &tex,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    // Format: DXGI_FORMAT_R10G10B10A2_TYPELESS,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
            )?,
        )
    };

    let (rtv2, rtv2_view) = unsafe {
        let tex = device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: window.inner_size().width as _,
                    Height: window.inner_size().height as _,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    // Format: DXGI_FORMAT_R8G8B8A8_TYPELESS,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                None,
            )
            .context("Failed to create RT2 texture")?;

        (
            device.CreateRenderTargetView(&tex, None)?,
            device.CreateShaderResourceView(
                &tex,
                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                        Texture2D: D3D11_TEX2D_SRV {
                            MostDetailedMip: 0,
                            MipLevels: 1,
                        },
                    },
                }),
            )?,
        )
    };

    let mut placement_groups = vec![];

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    for (i, (pi, _)) in package.get_all_by_reference(0x8080966d).iter().enumerate() {
        let placements: Unk8080966d = package_manager
            .read_tag_struct(TagHash::new(package.pkg_id(), *pi as _))
            .unwrap();
        debug!(
            "{i} Placement group {pi}, {} statics, {} instances ({:?})",
            TagHash::new(package.pkg_id(), *pi as _),
            placements.statics.len(),
            placements.transforms.len()
        );

        for v in &placements.statics {
            to_load.insert(*v, ());
        }

        placement_groups.push(placements);
    }

    if placement_groups.is_empty() {
        panic!("No map placements found in package");
    }

    let to_load: Vec<TagHash> = to_load.keys().cloned().collect();

    let mut static_map: IntMap<u32, StaticModel> = Default::default();
    let mut material_map: IntMap<u32, material::Unk808071e8> = Default::default();
    let mut vshader_map: IntMap<u32, (ID3D11VertexShader, ID3D11InputLayout)> = Default::default();
    let mut pshader_map: IntMap<u32, ID3D11PixelShader> = Default::default();
    let mut cbuffer_map_vs: IntMap<u32, ID3D11Buffer> = Default::default();
    let mut cbuffer_map_ps: IntMap<u32, ID3D11Buffer> = Default::default();
    let mut texture_map: IntMap<u32, LoadedTexture> = Default::default();

    info_span!("Loading statics").in_scope(|| {
        for almostloadable in &to_load {
            let mheader: Unk808071a7 = package_manager.read_tag_struct(*almostloadable).unwrap();
            for m in &mheader.materials {
                let mat: material::Unk808071e8 = package_manager.read_tag_struct(*m).unwrap();
                material_map.insert(m.0, mat);
            }

            match StaticModel::load(mheader, &device, &mut package_manager) {
                Ok(model) => {
                    static_map.insert(almostloadable.0, model);
                }
                Err(e) => {
                    error!(model = ?almostloadable, "Failed to load model: {e}");
                }
            }
        }
    });

    info!("Loaded {} statics", static_map.len());

    let mut vshader_fullscreen = None;
    let mut pshader_fullscreen = None;
    let mut errors = None;

    let flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };
    unsafe {
        (
            D3DCompileFromFile(
                w!("fullscreen.shader"),
                None,
                None,
                s!("VShader"),
                s!("vs_5_0"),
                flags,
                0,
                &mut vshader_fullscreen,
                Some(&mut errors),
            )
            .context("Failed to compile vertex shader")?,
            D3DCompileFromFile(
                w!("fullscreen.shader"),
                None,
                None,
                s!("PShader"),
                s!("ps_5_0"),
                flags,
                0,
                &mut pshader_fullscreen,
                Some(&mut errors),
            )
            .context("Failed to compile pixel shader")?,
        )
    };

    if let Some(errors) = errors {
        let estr = unsafe {
            let eptr = errors.GetBufferPointer();
            std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
        };
        let errors = String::from_utf8_lossy(estr);
        warn!("{}", errors);
    }

    // let vshader = vshader.unwrap();
    let vshader_fullscreen = vshader_fullscreen.unwrap();
    let pshader_fullscreen = pshader_fullscreen.unwrap();
    // let pshader = pshader.unwrap();

    info_span!("Loading shaders").in_scope(|| {
        for (t, m) in material_map.iter() {
            if let Ok(v) = package_manager.get_entry_by_tag(m.vertex_shader) {
                let _span =
                    tracing::debug_span!("load vshader", shader = ?m.vertex_shader).entered();

                vshader_map.entry(m.vertex_shader.0).or_insert_with(|| {
                    let vs_data = package_manager.read_tag(TagHash(v.reference)).unwrap();
                    let mut vs_cur = Cursor::new(&vs_data);
                    let dxbc_header: DxbcHeader = vs_cur.read_le().unwrap();
                    let input_sig = get_input_signature(&mut vs_cur, &dxbc_header).unwrap();

                    let layout = vertex_layout::build_input_layout(
                        &input_sig
                            .elements
                            .iter()
                            .map(|e| InputElement::from_dxbc(e, false))
                            .collect::<Vec<InputElement>>(),
                    );

                    unsafe {
                        let v = device
                            .CreateVertexShader(&vs_data, None)
                            .context("Failed to load vertex shader")
                            .unwrap();

                        let name = format!("VS {:?} (mat 0x{:x})\0", m.vertex_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        let input_layout = device
                            .CreateInputLayout(&layout, &vs_data)
                            .expect("Failed to create input layout");

                        (v, input_layout)
                    }
                });
            }

            if let Ok(v) = package_manager.get_entry_by_tag(m.pixel_shader) {
                let _span =
                    tracing::debug_span!("load pshader", shader = ?m.pixel_shader).entered();

                pshader_map.entry(m.pixel_shader.0).or_insert_with(|| {
                    let ps_data = package_manager.read_tag(TagHash(v.reference)).unwrap();
                    unsafe {
                        let v = device
                            .CreatePixelShader(&ps_data, None)
                            .context("Failed to load pixel shader")
                            .unwrap();

                        let name = format!("PS {:?} (mat 0x{:x})\0", m.pixel_shader, t);
                        v.SetPrivateData(
                            &WKPDID_D3DDebugObjectName,
                            name.len() as u32 - 1,
                            Some(name.as_ptr() as _),
                        )
                        .expect("Failed to set VS name");

                        v
                    }
                });
            }

            if m.unk98.len() > 1
                && m.unk98
                    .iter()
                    .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
            {
                trace!("Loading float4 cbuffer with {} elements", m.unk318.len());
                let buf = unsafe {
                    device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_IMMUTABLE,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: Default::default(),
                                ByteWidth: (m.unk98.len() * std::mem::size_of::<Vec4>()) as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: m.unk98.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load float4 cbuffer")
                        .unwrap()
                };

                cbuffer_map_vs.insert(*t, buf);
            }

            if m.unk34c.is_valid() {
                let buffer_header_ref = TagHash(
                    package_manager
                        .get_entry_by_tag(m.unk34c)
                        .unwrap()
                        .reference,
                );

                let buffer = package_manager.read_tag(buffer_header_ref).unwrap();
                trace!(
                    "Read {} bytes cbuffer from {buffer_header_ref:?}",
                    buffer.len()
                );
                let buf = unsafe {
                    device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_IMMUTABLE,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: Default::default(),
                                ByteWidth: buffer.len() as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: buffer.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load variable cbuffer")
                        .unwrap()
                };

                cbuffer_map_ps.insert(*t, buf);
            } else if !m.unk318.is_empty()
                && m.unk318
                    .iter()
                    .any(|v| v.x != 0.0 || v.y != 0.0 || v.z != 0.0 || v.w != 0.0)
            {
                trace!("Loading float4 cbuffer with {} elements", m.unk318.len());
                let buf = unsafe {
                    device
                        .CreateBuffer(
                            &D3D11_BUFFER_DESC {
                                Usage: D3D11_USAGE_IMMUTABLE,
                                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                                CPUAccessFlags: Default::default(),
                                ByteWidth: (m.unk318.len() * std::mem::size_of::<Vec4>()) as _,
                                ..Default::default()
                            },
                            Some(&D3D11_SUBRESOURCE_DATA {
                                pSysMem: m.unk318.as_ptr() as _,
                                ..Default::default()
                            }),
                        )
                        .context("Failed to load float4 cbuffer")
                        .unwrap()
                };

                cbuffer_map_ps.insert(*t, buf);
            }
        }
    });

    info!(
        "Loaded {} vertex shaders, {} pixel shaders",
        vshader_map.len(),
        pshader_map.len()
    );

    let (vshader_fullscreen, pshader_fullscreen) = unsafe {
        // let vs_blob = std::slice::from_raw_parts(
        //     vshader.GetBufferPointer() as *const u8,
        //     vshader.GetBufferSize(),
        // );
        // let v = device.CreateVertexShader(vs_blob, None)?;
        let vs_blob = std::slice::from_raw_parts(
            vshader_fullscreen.GetBufferPointer() as *const u8,
            vshader_fullscreen.GetBufferSize(),
        );
        let v2 = device.CreateVertexShader(vs_blob, None)?;
        let ps_blob = std::slice::from_raw_parts(
            pshader_fullscreen.GetBufferPointer() as *const u8,
            pshader_fullscreen.GetBufferSize(),
        );
        let v3 = device.CreatePixelShader(ps_blob, None)?;
        // let ps_blob = std::slice::from_raw_parts(
        //     pshader.GetBufferPointer() as *const u8,
        //     pshader.GetBufferSize(),
        // );
        // let p = device.CreatePixelShader(ps_blob, None)?;

        (v2, v3)
    };

    info_span!("Loading textures").in_scope(|| {
        for m in material_map.values() {
            for t in m.ps_textures.iter().chain(m.vs_textures.iter()) {
                let tex_hash = t.texture;
                if !tex_hash.is_valid() || texture_map.contains_key(&tex_hash.0) {
                    continue;
                }
                let _span = debug_span!("load texture", texture = ?tex_hash).entered();

                let texture_header_ref = TagHash(
                    package_manager
                        .get_entry_by_tag(tex_hash)
                        .unwrap()
                        .reference,
                );

                let texture: TextureHeader = package_manager.read_tag_struct(tex_hash).unwrap();
                let texture_data = if let Some(t) = texture.large_buffer {
                    package_manager
                        .read_tag(t)
                        .expect("Failed to read texture data")
                } else {
                    package_manager
                        .read_entry(
                            texture_header_ref.pkg_id(),
                            texture_header_ref.entry_index() as _,
                        )
                        .expect("Failed to read texture data")
                        .to_vec()
                };

                // info!("Uploading texture {} {texture:?}", tex_hash);
                let (tex, view) = unsafe {
                    let mut initial_data =
                        vec![D3D11_SUBRESOURCE_DATA::default(); texture.array_size as _];
                    let (pitch, slice_pitch) =
                        calculate_pitch(texture.format, texture.width as _, texture.height as _);

                    for (i, d) in initial_data.iter_mut().enumerate() {
                        d.pSysMem = texture_data.as_ptr().add(i * slice_pitch) as _;
                        d.SysMemPitch = pitch as _;
                        d.SysMemSlicePitch = slice_pitch as _;
                    }

                    if texture.depth > 1 {
                        let tex = device
                            .CreateTexture3D(
                                &D3D11_TEXTURE3D_DESC {
                                    Width: texture.width as _,
                                    Height: texture.height as _,
                                    Depth: texture.depth as _,
                                    MipLevels: 1,
                                    Format: texture.format.into(),
                                    Usage: D3D11_USAGE_DEFAULT,
                                    BindFlags: D3D11_BIND_SHADER_RESOURCE,
                                    CPUAccessFlags: Default::default(),
                                    MiscFlags: Default::default(),
                                },
                                Some(initial_data.as_ptr()),
                            )
                            .context("Failed to create 3D texture")
                            .unwrap();

                        let view = device
                            .CreateShaderResourceView(
                                &tex,
                                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                                    Format: texture.format.into(),
                                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE3D,
                                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                        Texture3D: D3D11_TEX3D_SRV {
                                            MostDetailedMip: 0,
                                            MipLevels: 1,
                                        },
                                    },
                                }),
                            )
                            .unwrap();

                        (TextureHandle::Texture3D(tex), view)
                    } else {
                        let tex = device
                            .CreateTexture2D(
                                &D3D11_TEXTURE2D_DESC {
                                    Width: texture.width as _,
                                    Height: texture.height as _,
                                    MipLevels: 1,
                                    ArraySize: texture.array_size as _,
                                    Format: texture.format.into(),
                                    SampleDesc: DXGI_SAMPLE_DESC {
                                        Count: 1,
                                        Quality: 0,
                                    },
                                    Usage: D3D11_USAGE_DEFAULT,
                                    BindFlags: D3D11_BIND_SHADER_RESOURCE,
                                    CPUAccessFlags: Default::default(),
                                    MiscFlags: Default::default(),
                                },
                                Some(initial_data.as_ptr()),
                            )
                            .context("Failed to create texture")
                            .unwrap();

                        let view = device
                            .CreateShaderResourceView(
                                &tex,
                                Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                                    Format: texture.format.into(),
                                    ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                                    Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                                        Texture2D: D3D11_TEX2D_SRV {
                                            MostDetailedMip: 0,
                                            MipLevels: 1,
                                        },
                                    },
                                }),
                            )
                            .unwrap();

                        (TextureHandle::Texture2D(tex), view)
                    }
                };

                texture_map.insert(
                    tex_hash.0,
                    LoadedTexture {
                        handle: tex,
                        view,
                        format: texture.format,
                    },
                );
            }
        }
    });

    info!("Loaded {} textures", texture_map.len());

    let le_sampler = unsafe {
        device.CreateSamplerState(&D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
            MipLODBias: 0.,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            BorderColor: Default::default(),
            MinLOD: 0.,
            MaxLOD: f32::MAX,
        })?
    };

    let _le_cbuffer = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: 2 * (4 * 4 * 4) + 4 * 4,
                ..Default::default()
            },
            None,
        )?
    };

    let _le_model_cbuffer = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (4 * 4 * 4) * 3,
                ..Default::default()
            },
            None,
        )?
    };

    let le_model_cb0 = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (64 * 4) * 4,
                ..Default::default()
            },
            None,
        )?
    };

    let le_vertex_cb11 = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (16 * std::mem::size_of::<ScopeStaticInstance>()) as _,
                ..Default::default()
            },
            None,
        )?
    };

    let le_vertex_cb12 = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: std::mem::size_of::<ScopeView>() as _,
                ..Default::default()
            },
            None,
        )?
    };

    let le_pixel_cb12 = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (4 * 4) * 8,
                ..Default::default()
            },
            None,
        )?
    };

    let cb_composite_options = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: 4 * 4,
                ..Default::default()
            },
            None,
        )?
    };

    let rasterizer_state = unsafe {
        device
            .CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
                // CullMode: D3D11_CULL_NONE,
                CullMode: D3D11_CULL_BACK,
                FrontCounterClockwise: true.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: true.into(),
                ScissorEnable: Default::default(),
                MultisampleEnable: Default::default(),
                AntialiasedLineEnable: Default::default(),
            })
            .context("Failed to create Rasterizer State")?
    };

    let depth_stencil_texture = unsafe {
        device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: window.inner_size().width,
                    Height: window.inner_size().height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_D32_FLOAT,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_DEPTH_STENCIL,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                None,
            )
            .context("Failed to create depth texture")?
    };

    let depth_stencil_state = unsafe {
        device
            .CreateDepthStencilState(&D3D11_DEPTH_STENCIL_DESC {
                DepthEnable: true.into(),
                DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
                DepthFunc: D3D11_COMPARISON_GREATER_EQUAL,
                StencilEnable: false.into(),
                StencilReadMask: 0xff,
                StencilWriteMask: 0xff,
                FrontFace: D3D11_DEPTH_STENCILOP_DESC {
                    StencilFailOp: D3D11_STENCIL_OP_KEEP,
                    StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
                    StencilPassOp: D3D11_STENCIL_OP_KEEP,
                    StencilFunc: D3D11_COMPARISON_ALWAYS,
                },
                BackFace: D3D11_DEPTH_STENCILOP_DESC {
                    StencilFailOp: D3D11_STENCIL_OP_KEEP,
                    StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
                    StencilPassOp: D3D11_STENCIL_OP_KEEP,
                    StencilFunc: D3D11_COMPARISON_ALWAYS,
                },
            })
            .context("Failed to create depth stencil state")?
    };

    let depth_stencil_view = unsafe {
        device
            .CreateDepthStencilView(
                &depth_stencil_texture,
                Some(&D3D11_DEPTH_STENCIL_VIEW_DESC {
                    Format: DXGI_FORMAT_D32_FLOAT,
                    ViewDimension: D3D11_DSV_DIMENSION_TEXTURE2D,
                    Flags: 0,
                    Anonymous: D3D11_DEPTH_STENCIL_VIEW_DESC_0 {
                        Texture2D: { D3D11_TEX2D_DSV { MipSlice: 0 } },
                    },
                }),
            )
            .context("Failed to create depth stencil view")?
    };

    // let y_to_z_up: Mat4 = Mat4::from_rotation_x(-90f32.to_radians());

    let mut input_state = InputState {
        w: false,
        a: false,
        s: false,
        d: false,
        mouse1: false,
        shift: false,
        ctrl: false,
        space: false,
    };

    let mut camera = FpsCamera::default();

    unsafe {
        let cb11_data = vec![Vec4::splat(0.6); 128];
        let bmap = device_context
            .Map(&le_model_cb0, 0, D3D11_MAP_WRITE_DISCARD, 0)
            .context("Failed to map model cbuffer11")
            .unwrap();

        bmap.pData
            .copy_from_nonoverlapping(cb11_data.as_ptr() as _, std::mem::size_of::<Vec4>() * 64);

        device_context.Unmap(&le_model_cb0, 0);
    }

    let matcap = unsafe {
        const MATCAP_DATA: &[u8] = include_bytes!("matte.data");
        device
            .CreateTexture2D(
                &D3D11_TEXTURE2D_DESC {
                    Width: 128 as _,
                    Height: 128 as _,
                    MipLevels: 1,
                    ArraySize: 1 as _,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_DEFAULT,
                    BindFlags: D3D11_BIND_SHADER_RESOURCE,
                    CPUAccessFlags: Default::default(),
                    MiscFlags: Default::default(),
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: MATCAP_DATA.as_ptr() as _,
                    SysMemPitch: 128 * 4,
                    ..Default::default()
                } as _),
            )
            .context("Failed to create texture")?
    };
    let matcap_view = unsafe {
        device.CreateShaderResourceView(
            &matcap,
            Some(&D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            }),
        )?
    };

    let blend_state = unsafe {
        device.CreateBlendState(&D3D11_BLEND_DESC {
            RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: false.into(),
                SrcBlend: D3D11_BLEND_ONE,
                DestBlend: D3D11_BLEND_ZERO,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_ZERO,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            }; 8],
            ..Default::default()
        })?
    };

    let mut imgui = imgui::Context::create();
    imgui.style_mut().window_rounding = 4.0;

    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);
    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(FontConfig {
            size_pixels: font_size,
            ..FontConfig::default()
        }),
    }]);
    let mut renderer = unsafe { imgui_dx11_renderer::Renderer::new(&mut imgui, &device)? };

    let _start_time = Instant::now();
    let mut composite_mode: usize = 0;
    let mut placement_i: usize = 1;
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;
    event_loop.run(move |event, _, control_flow| {
        platform.handle_event(imgui.io_mut(), &window, &event);
        match &event {
            Event::WindowEvent { event, .. } => {
                if true {
                    match event {
                        WindowEvent::Resized(new_dims) => unsafe {
                            swapchain_target = None;
                            swap_chain
                                .ResizeBuffers(
                                    1,
                                    new_dims.width,
                                    new_dims.height,
                                    DXGI_FORMAT_B8G8R8A8_UNORM,
                                    0,
                                )
                                .expect("Failed to resize swapchain");

                            let bb: ID3D11Texture2D = swap_chain.GetBuffer(0).unwrap();

                            let new_rtv = device.CreateRenderTargetView(&bb, None).unwrap();

                            device_context.OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                            swapchain_target = Some(new_rtv);
                        },
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // renderer.resize();
                        }
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            if button == &MouseButton::Left && !imgui.io().want_capture_mouse {
                                input_state.mouse1 = *state == ElementState::Pressed
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if let Some(ref mut p) = last_cursor_pos {
                                let delta = (position.x - p.x, position.y - p.y);

                                if input_state.mouse1 && !imgui.io().want_capture_mouse {
                                    camera.update_mouse((delta.0 as f32, delta.1 as f32).into());
                                }

                                last_cursor_pos = Some(*position);
                            } else {
                                last_cursor_pos = Some(*position);
                            }
                        }
                        WindowEvent::ModifiersChanged(modifiers) => {
                            input_state.shift = modifiers.shift();
                            input_state.ctrl = modifiers.ctrl();
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            if input.state == ElementState::Pressed {
                                match input.virtual_keycode {
                                    Some(VirtualKeyCode::Right) => {
                                        placement_i = placement_i.wrapping_add(1)
                                    }

                                    Some(VirtualKeyCode::Left) => {
                                        placement_i = placement_i.wrapping_sub(1)
                                    }

                                    Some(VirtualKeyCode::Escape) => {
                                        *control_flow = ControlFlow::Exit
                                    }
                                    _ => {}
                                }

                                if let Some(VirtualKeyCode::Right | VirtualKeyCode::Left) =
                                    input.virtual_keycode
                                {
                                    info!(
                                        "Switched to placement group {}",
                                        placement_i % placement_groups.len()
                                    );

                                    // let mut p_min = Vec3::MAX;
                                    // let mut p_max = Vec3::MIN;
                                    // for t in &placement_groups[placement_i % placement_groups.len()]
                                    //     .transforms
                                    // {
                                    //     let v = Vec3::new(
                                    //         t.translation.x,
                                    //         t.translation.y,
                                    //         t.translation.z,
                                    //     );
                                    //
                                    //     p_min = p_min.min(v);
                                    //     p_max = p_max.max(v);
                                    // }
                                    //
                                    // let center = (p_min + p_max) / 2.0;
                                    // camera.position = center;
                                }
                            }

                            match input.virtual_keycode {
                                Some(VirtualKeyCode::W) => {
                                    input_state.w = input.state == ElementState::Pressed
                                }
                                Some(VirtualKeyCode::A) => {
                                    input_state.a = input.state == ElementState::Pressed
                                }
                                Some(VirtualKeyCode::S) => {
                                    input_state.s = input.state == ElementState::Pressed
                                }
                                Some(VirtualKeyCode::D) => {
                                    input_state.d = input.state == ElementState::Pressed
                                }
                                Some(VirtualKeyCode::Space) => {
                                    input_state.space = input.state == ElementState::Pressed
                                }
                                _ => {}
                            }
                        }

                        _ => (),
                    }
                }
            }
            Event::RedrawRequested(..) => {
                imgui.io_mut().update_delta_time(last_frame.elapsed());
                camera.update(&input_state, last_frame.elapsed().as_secs_f32());

                let ui = imgui.new_frame();
                ui.window("FPS")
                    .flags(WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
                    .build(|| ui.text(format!("{:.1}", 1.0 / last_frame.elapsed().as_secs_f32())));

                ui.window("Options")
                    .flags(WindowFlags::NO_TITLE_BAR | WindowFlags::NO_RESIZE)
                    .size([128.0, 36.0], Condition::Always)
                    .build(|| {
                        ui.combo(" ", &mut composite_mode, &COMPOSITOR_MODES, |v| {
                            format!("{v}").into()
                        });
                    });

                last_frame = Instant::now();

                unsafe {
                    device_context.ClearRenderTargetView(&rtv0, [0.0, 0.0, 0.0, 1.0].as_ptr() as _);
                    device_context.ClearRenderTargetView(&rtv1, [0.0, 0.0, 0.0, 0.0].as_ptr() as _);
                    device_context.ClearRenderTargetView(&rtv2, [0.0, 0.0, 0.0, 0.0].as_ptr() as _);
                    device_context.ClearDepthStencilView(
                        &depth_stencil_view,
                        D3D11_CLEAR_DEPTH.0 as _,
                        0.0,
                        0,
                    );

                    let window_dims = window.inner_size();

                    device_context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                        TopLeftX: 0.0,
                        TopLeftY: 0.0,
                        Width: window_dims.width as f32,
                        Height: window_dims.height as f32,
                        MinDepth: 0.0,
                        MaxDepth: 1.0,
                    }]));

                    device_context.RSSetState(&rasterizer_state);
                    device_context.OMSetBlendState(
                        &blend_state,
                        Some(&[1f32, 1., 1., 1.] as _),
                        0xffffffff,
                    );
                    device_context.OMSetRenderTargets(
                        Some(&[Some(rtv0.clone()), Some(rtv1.clone()), Some(rtv2.clone())]),
                        &depth_stencil_view,
                    );
                    device_context.OMSetDepthStencilState(
                        &depth_stencil_state.clone(),
                        0,
                    );

                    let projection = Mat4::perspective_infinite_reverse_rh(
                        90f32.to_radians(),
                        window_dims.width as f32 / window_dims.height as f32,
                        0.001,
                    );
                    let view = camera.calculate_matrix();

                    let bmap = device_context
                        .Map(&le_vertex_cb12, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    let proj_view = projection * view;
                    let mut view2 = Mat4::IDENTITY;
                    view2.w_axis = camera.position.extend(1.0);

                    let scope_view = ScopeView {
                        world_to_projective: proj_view, //Mat4::from_mat3(normalized),
                        camera_to_world: view2,
                        // HACK: Account for missing depth value in output
                        view_miscellaneous: Vec4::new(0.0, 0.0, 0.001, 1.0),
                        ..Default::default()
                    };
                    bmap.pData.copy_from_nonoverlapping(
                        &scope_view as *const ScopeView as _,
                        std::mem::size_of::<ScopeView>(),
                    );

                    device_context.Unmap(&le_vertex_cb12, 0);

                    let bmap = device_context
                        .Map(&le_pixel_cb12, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    let mut cb12_data = vec![Vec4::ZERO; 8];
                    cb12_data[7] = camera.position.yxz().extend(1.0);

                    bmap.pData.copy_from_nonoverlapping(
                        cb12_data.as_ptr() as _,
                        8 * std::mem::size_of::<Vec4>(),
                    );

                    device_context.Unmap(&le_pixel_cb12, 0);

                    // device_context.VSSetConstantBuffers(0, Some(&[Some(le_cbuffer.clone())]));
                    device_context.VSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.clone())]));

                    // TODO(cohae): Find a more solid way to assign samplers
                    device_context.PSSetSamplers(
                        0,
                        Some(&[
                            Some(le_sampler.clone()),
                            Some(le_sampler.clone()),
                            Some(le_sampler.clone()),
                            Some(le_sampler.clone()),
                            Some(le_sampler.clone()),
                        ]),
                    );

                    device_context.PSSetConstantBuffers(12, Some(&[Some(le_vertex_cb12.clone())]));
                    // device_context.PSSetConstantBuffers(12, Some(&[Some(le_pixel_cb12.clone())]));

                    let placements = &placement_groups[placement_i % placement_groups.len()];
                    // for placements in &placement_groups {
                    for instance in &placements.instances {
                        if let Some(model_hash) =
                            placements.statics.iter().nth(instance.static_index as _)
                        {
                            let _span =
                                debug_span!("Draw static instance", count = instance.instance_count, model = ?model_hash)
                                    .entered();

                            if let Some(model) = static_map.get(&model_hash.0) {
                                for transform in &placements.transforms[instance.instance_offset
                                    as usize
                                    ..(instance.instance_offset + instance.instance_count) as usize]
                                {
                                    let mm = Mat4::from_scale_rotation_translation(
                                        Vec3::splat(transform.scale.x),
                                        Quat::from_xyzw(
                                            transform.rotation.x,
                                            transform.rotation.y,
                                            transform.rotation.z,
                                            transform.rotation.w,
                                        )
                                        .inverse(),
                                        Vec3::ZERO,
                                    );

                                    let model_matrix = Mat4::from_cols(
                                        mm.x_axis.truncate().extend(transform.translation.x),
                                        mm.y_axis.truncate().extend(transform.translation.y),
                                        mm.z_axis.truncate().extend(transform.translation.z),
                                        mm.w_axis,
                                    );

                                    let combined_matrix = model.mesh_transform() * model_matrix;

                                    let bmap = device_context
                                        .Map(&le_vertex_cb11, 0, D3D11_MAP_WRITE_DISCARD, 0)
                                        .unwrap();

                                    let scope_instance = Mat4 {
                                        x_axis: combined_matrix.x_axis,
                                        y_axis: combined_matrix.y_axis,
                                        z_axis: combined_matrix.z_axis,
                                        w_axis: model
                                            .texcoord_transform()
                                            .extend(f32::from_bits(u32::MAX)),
                                    };

                                    let bdata = vec![scope_instance; 16];
                                    bmap.pData.copy_from_nonoverlapping(
                                        // &scope_instance as *const ScopeStaticInstance as _,
                                        bdata.as_ptr() as _,
                                        16 * std::mem::size_of::<Mat4>(),
                                    );

                                    device_context.Unmap(&le_vertex_cb11, 0);
                                    device_context.VSSetConstantBuffers(
                                        11,
                                        Some(&[Some(le_vertex_cb11.clone())]),
                                    );

                                    model.draw(
                                        &device_context,
                                        &material_map,
                                        &vshader_map,
                                        &pshader_map,
                                        &cbuffer_map_vs,
                                        &cbuffer_map_ps,
                                        &texture_map,
                                        le_model_cb0.clone(),
                                    );
                                }
                        }
                    }
                }

                    device_context.OMSetRenderTargets(
                        Some(&[Some(swapchain_target.as_ref().unwrap().clone())]),
                        None,
                    );
                    device_context.PSSetShaderResources(
                        0,
                        Some(&[
                            Some(rtv0_view.clone()),
                            Some(rtv1_view.clone()),
                            Some(rtv2_view.clone()),
                            Some(matcap_view.clone()),
                        ]),
                    );

                    let bmap = device_context
                        .Map(&cb_composite_options, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();
                    bmap.pData.copy_from_nonoverlapping(
                        &(COMPOSITOR_MODES[composite_mode] as u32) as *const u32 as _,
                        4,
                    );

                    device_context.Unmap(&cb_composite_options, 0);
                    device_context
                        .PSSetConstantBuffers(0, Some(&[Some(cb_composite_options.clone())]));

                    device_context.VSSetShader(&vshader_fullscreen, None);
                    device_context.PSSetShader(&pshader_fullscreen, None);
                    device_context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
                    device_context.Draw(3, 0);

                    platform.prepare_render(&ui, &window);
                    renderer
                        .render(imgui.render())
                        .expect("imgui rendering failed");
                    device_context.OMSetDepthStencilState(None, 0);

                    swap_chain.Present(1, 0).unwrap();

                    tracy_client::Client::running().map(|c| c.frame_mark());
                };
            }
            Event::MainEventsCleared => {
                let io = imgui.io_mut();
                platform
                    .prepare_frame(io, &window)
                    .expect("Failed to start frame");
                window.request_redraw();
            }
            _ => (),
        }
    });
}
