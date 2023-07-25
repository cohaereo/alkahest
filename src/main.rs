#[macro_use]
extern crate windows;

use std::collections::HashMap;
use std::fmt::Write;
use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Seek, SeekFrom};
use std::mem::transmute;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use crate::camera::{FpsCamera, InputState};
use anyhow::Context;
use binrw::BinReaderExt;
use destiny_pkg::PackageVersion::Destiny2PreBeyondLight;
use destiny_pkg::{PackageManager, TagHash};
use egui::Widget;
use glam::{EulerRot, Mat4, Quat, Vec2, Vec3, Vec3Swizzles, Vec4};
use itertools::Itertools;
use nohash_hasher::IntMap;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use tracing::{error, info, warn};
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

use crate::dxgi::{calculate_pitch, DxgiFormat};
use crate::entity::{
    decode_vertices, ELodCategory, EPrimitiveType, IndexBufferHeader, VertexBufferHeader,
};
use crate::static_render::{LoadedTexture, StaticModel};
use crate::statics::{Unk80807194, Unk8080719a, Unk808071a7, Unk8080966d};
use crate::text::{decode_text, StringData, StringPart, StringSetHeader};
use crate::texture::TextureHeader;
use crate::types::{DestinyHash, Vector3};

mod camera;
mod dds;
mod dxgi;
mod entity;
mod material;
mod static_render;
mod statics;
mod structure;
mod text;
mod texture;
mod types;
mod unknown;

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

pub fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let pkg_path = std::env::args().nth(1).unwrap_or_default();
    let package = Destiny2PreBeyondLight
        .open(&pkg_path)
        .expect("Failed to open package");
    let mut package_manager = PackageManager::new(
        PathBuf::from_str(&pkg_path).unwrap().parent().unwrap(),
        Destiny2PreBeyondLight,
        true,
    )
    .unwrap();

    // return;
    // Winit event loop
    let event_loop = EventLoop::new();
    let mut window = winit::window::WindowBuilder::new()
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
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
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
            D3D11_CREATE_DEVICE_SINGLETHREADED | D3D11_CREATE_DEVICE_DEBUG,
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

    let mut render_target_view = unsafe {
        let buffer = swap_chain.GetBuffer::<ID3D11Resource>(0)?;

        device.CreateRenderTargetView(&buffer, None)?
    };

    let mut placement_groups = vec![];

    let mut to_load: HashMap<TagHash, ()> = Default::default();
    for (i, (pi, _)) in package.get_all_by_reference(0x8080966d).iter().enumerate() {
        let placements: Unk8080966d = package_manager
            .read_tag_struct(TagHash::new(package.pkg_id(), *pi as _))
            .unwrap();
        info!(
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
    let mut vshader_map: IntMap<u32, ID3D11VertexShader> = Default::default();
    let mut pshader_map: IntMap<u32, ID3D11PixelShader> = Default::default();
    let mut texture_map: IntMap<u32, LoadedTexture> = Default::default();
    for almostloadable in &to_load {
        // let almostloadable = &TagHash::new(package.pkg_id(), 637);

        let mheader: Unk808071a7 = package_manager.read_tag_struct(*almostloadable).unwrap();
        for m in &mheader.materials {
            let mat: material::Unk808071e8 = package_manager.read_tag_struct(*m).unwrap();
            material_map.insert(m.0, mat);
        }

        match StaticModel::load(mheader, &device, &device_context, &mut package_manager) {
            Ok(model) => {
                static_map.insert(almostloadable.0, model);
                // info!(model = ?almostloadable, "Successfully loaded model");
            }
            Err(e) => {
                error!(model = ?almostloadable, "Failed to load model: {e}");
            }
        }
    }

    println!("Compiling shaders");
    let mut vshader = None;
    // let mut pshader = None;
    let mut errors = None;

    let flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };
    unsafe {
        (
            D3DCompileFromFile(
                w!("vshader.shader"),
                None,
                None,
                s!("VShader"),
                s!("vs_5_0"),
                flags,
                0,
                &mut vshader,
                Some(&mut errors),
            )
            .context("Failed to compile vertex shader")?,
            // D3DCompileFromFile(
            //     w!("shaders.shader"),
            //     None,
            //     None,
            //     s!("PShader"),
            //     s!("ps_5_0"),
            //     flags,
            //     0,
            //     &mut pshader,
            //     Some(&mut errors),
            // )
            // .context("Failed to compile pixel shader")?,
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

    let vshader = vshader.unwrap();
    // let pshader = pshader.unwrap();

    println!("Creating vertex layout");
    let vertex_layout = unsafe {
        let vs_blob = std::slice::from_raw_parts(
            vshader.GetBufferPointer() as *const u8,
            vshader.GetBufferSize(),
        );
        // let ps_blob = std::slice::from_raw_parts(
        //     pshader.GetBufferPointer() as *const u8,
        //     pshader.GetBufferSize(),
        // );
        device.CreateInputLayout(
            &[
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("TEXCOORD"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 12,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ],
            vs_blob,
        )?
    };

    println!("Creating shaders");
    for m in material_map.values() {
        if let Ok(v) = package_manager.get_entry_by_tag(m.vertex_shader) {
            vshader_map.entry(m.vertex_shader.0).or_insert_with(|| {
                info!("Loading VShader {:?}", m.vertex_shader);
                let vs_data = package_manager.read_tag(TagHash(v.reference)).unwrap();
                unsafe {
                    device
                        .CreateVertexShader(&vs_data, None)
                        .context("Failed to load vertex shader")
                        .unwrap()
                }
            });
        }

        if let Ok(v) = package_manager.get_entry_by_tag(m.pixel_shader) {
            pshader_map.entry(m.pixel_shader.0).or_insert_with(|| {
                info!("Loading PShader {:?}", m.pixel_shader);
                let ps_data = package_manager.read_tag(TagHash(v.reference)).unwrap();
                unsafe {
                    device
                        .CreatePixelShader(&ps_data, None)
                        .context("Failed to load pixel shader")
                        .unwrap()
                }
            });
        }
    }

    let vshader = unsafe {
        let vs_blob = std::slice::from_raw_parts(
            vshader.GetBufferPointer() as *const u8,
            vshader.GetBufferSize(),
        );
        let v = device.CreateVertexShader(vs_blob, None)?;
        // let ps_blob = std::slice::from_raw_parts(
        //     pshader.GetBufferPointer() as *const u8,
        //     pshader.GetBufferSize(),
        // );
        // let p = device.CreatePixelShader(ps_blob, None)?;

        v
    };

    println!("Loading textures");

    let mut egui_ctx = egui::Context::default();
    struct PackageTexture {
        handle: ID3D11Texture2D,
        view: ID3D11ShaderResourceView,
        format: DxgiFormat,
        width: u16,
        height: u16,
        depth: u16,
        array_size: u16,
        e_type: u8,
        e_subtype: u8,
        index: usize,
    }

    for m in material_map.values()
    // for (i, e) in package
    //     .entries()
    //     .iter()
    //     .enumerate()
    //     .filter(|(_, v)| v.file_type == 32 && v.file_size == 40)
    {
        // println!("Material");
        for t in &m.ps_textures {
            // println!("\t{} {:?}", t.index, t.texture);
            let tex_hash = t.texture;
            if !tex_hash.is_valid() || texture_map.contains_key(&tex_hash.0) {
                continue;
            }

            let texture_header_ref = TagHash(
                package_manager
                    .get_entry_by_tag(tex_hash)
                    .unwrap()
                    .reference,
            );

            // // Skip everything but cubemaps
            // if e.file_subtype != 2 {
            //     continue;
            // }

            // let v = package.read_entry(i).unwrap();
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
            // let mut out_file = File::create(format!("dump/{i}.dds")).unwrap();
            // dump_to_dds(&mut out_file, &texture, &texture_data);

            // info!("Uploading texture {} {texture:?}", tex_hash);
            let tex = unsafe {
                let mut initial_data = [D3D11_SUBRESOURCE_DATA::default(); 6];
                let (pitch, slice_pitch) =
                    calculate_pitch(texture.format, texture.width as _, texture.height as _);

                for (i, d) in initial_data.iter_mut().enumerate() {
                    d.pSysMem = texture_data.as_ptr().add(i * slice_pitch) as _;
                    d.SysMemPitch = pitch as _;
                }

                device
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
                    .context("Failed to create texture")?
            };
            let view = unsafe {
                device.CreateShaderResourceView(
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
                )?
            };

            unsafe {
                device_context.PSSetShaderResources(0, Some(&[Some(view.clone())]));
            }
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

    let le_cbuffer = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: 2 * (4 * 4 * 4),
                ..Default::default()
            },
            None,
        )?
    };

    let le_model_cbuffer = unsafe {
        device.CreateBuffer(
            &D3D11_BUFFER_DESC {
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                ByteWidth: (4 * 4 * 4) * 2,
                ..Default::default()
            },
            None,
        )?
    };

    let le_model_cb11 = unsafe {
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

    let rasterizer_state = unsafe {
        device
            .CreateRasterizerState(&D3D11_RASTERIZER_DESC {
                FillMode: D3D11_FILL_SOLID,
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
                    Format: DXGI_FORMAT_D32_FLOAT_S8X24_UINT,
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
                DepthFunc: D3D11_COMPARISON_LESS,
                StencilEnable: true.into(),
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
                    Format: DXGI_FORMAT_D32_FLOAT_S8X24_UINT,
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

    let start_time = Instant::now();
    let mut tex_i: usize = 0;
    let mut placement_i: usize = 1;
    let mut rotation = Vec2::ZERO;
    let mut last_frame = Instant::now();
    let mut last_cursor_pos: Option<PhysicalPosition<f64>> = None;
    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::Resized(new_dims) => unsafe {
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

                        device_context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                            TopLeftX: 0.0,
                            TopLeftY: 0.0,
                            Width: new_dims.width as f32,
                            Height: new_dims.height as f32,
                            MinDepth: 0.0,
                            MaxDepth: 1.0,
                        }]));
                        device_context.OMSetRenderTargets(Some(&[Some(new_rtv.clone())]), None);

                        render_target_view = new_rtv;
                    },
                    WindowEvent::ScaleFactorChanged { .. } => {
                        // renderer.resize();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == &MouseButton::Left {
                            input_state.mouse1 = *state == ElementState::Pressed
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(ref mut p) = last_cursor_pos {
                            let delta = (position.x - p.x, position.y - p.y);

                            if input_state.mouse1 {
                                camera.update_mouse((delta.0 as f32, delta.1 as f32).into());
                                // rotation -= Vec2::new(delta.1 as f32, delta.0 as f32) * 0.005;
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
                                Some(VirtualKeyCode::Up) => {
                                    tex_i = tex_i.wrapping_add(1);
                                    info!("Switched to texture index {}", tex_i)
                                }
                                Some(VirtualKeyCode::Down) => {
                                    tex_i = tex_i.wrapping_sub(1);
                                    info!("Switched to texture index {}", tex_i)
                                }
                                Some(VirtualKeyCode::Right) => {
                                    placement_i = placement_i.wrapping_add(1)
                                }

                                Some(VirtualKeyCode::Left) => {
                                    placement_i = placement_i.wrapping_sub(1)
                                }

                                Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
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
            Event::RedrawRequested(..) => {
                camera.update(&input_state, last_frame.elapsed().as_secs_f32());
                last_frame = Instant::now();

                unsafe {
                    device_context.ClearRenderTargetView(
                        &render_target_view,
                        [1.0, 0.0, 0.0, 1.0].as_ptr() as _,
                    );
                    device_context.ClearDepthStencilView(
                        &depth_stencil_view,
                        D3D11_CLEAR_DEPTH.0 as _,
                        1.0,
                        0,
                    );

                    let window_dims = window.inner_size();

                    device_context.OMGetDepthStencilState(
                        Some(&mut Some(depth_stencil_state.clone())),
                        Some(&mut 1),
                    );
                    device_context.OMSetRenderTargets(
                        Some(&[Some(render_target_view.clone())]),
                        &depth_stencil_view,
                    );

                    device_context.RSSetState(&rasterizer_state);

                    device_context.VSSetShader(&vshader, None);
                    // device_context.PSSetShader(&pshader, None);
                    device_context.IASetInputLayout(&vertex_layout);

                    let projection = Mat4::perspective_lh(
                        90f32.to_radians(),
                        window_dims.width as f32 / window_dims.height as f32,
                        0.01,
                        3000.0,
                    );
                    let view = camera.calculate_matrix();

                    let view_proj = view * projection;
                    // Mat4::from_quat(camera_rot) * Mat4::from_translation(position) * y_to_z_up;

                    let bmap = device_context
                        .Map(&le_cbuffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
                        .unwrap();

                    bmap.pData.copy_from_nonoverlapping(
                        &(projection, view) as *const (Mat4, Mat4) as _,
                        2 * std::mem::size_of::<Mat4>(),
                    );

                    device_context.Unmap(&le_cbuffer, 0);
                    device_context.VSSetConstantBuffers(0, Some(&[Some(le_cbuffer.clone())]));
                    // device_context.VSSetConstantBuffers(12, Some(&[Some(le_cbuffer.clone())]));

                    // if !textures.is_empty() {
                    //     let le_texture = &textures[tex_i % textures.len()];
                    //     device_context
                    //         .PSSetShaderResources(0, Some(&[Some(le_texture.view.clone())]));
                    // }
                    device_context.PSSetSamplers(
                        0,
                        Some(&[Some(le_sampler.clone()), Some(le_sampler.clone())]),
                    );

                    // device_context.Draw(4, 0);
                    // model.draw(&device_context);
                    // static_map.values().next().unwrap().draw(&device_context);
                    let placements = &placement_groups[placement_i % placement_groups.len()];
                    for instance in &placements.instances {
                        if let Some(model_hash) =
                            placements.statics.iter().nth(instance.static_index as _)
                        {
                            if let Some(model) = static_map.get(&model_hash.0) {
                                for transform in &placements.transforms[instance.instance_offset
                                    as usize
                                    ..(instance.instance_offset + instance.instance_count) as usize]
                                {
                                    let model_matrix =
                                        Mat4::from_translation(
                                            [
                                                transform.translation.x,
                                                transform.translation.y,
                                                transform.translation.z,
                                            ]
                                            .into(),
                                        ) * Mat4::from_quat(Quat::from_xyzw(
                                            transform.rotation.x,
                                            transform.rotation.y,
                                            transform.rotation.z,
                                            transform.rotation.w,
                                        )) * Mat4::from_scale(Vec3::splat(transform.scale.x));

                                    const RANDOM_COLORS: [u32; 64] = [
                                        0xFFE4C4, 0x008080, 0x00FA9A, 0xF5DEB3, 0x8FBC8F, 0x7FFF00,
                                        0x6A5ACD, 0xFF1493, 0x5F9EA0, 0x7B68EE, 0x778899, 0xFFF0F5,
                                        0xDA70D6, 0xF0F8FF, 0xCD853F, 0xFFC0CB, 0xFFE4E1, 0x000080,
                                        0xB0E0E6, 0xD3D3D3, 0x2F4F4F, 0x483D8B, 0xA0522D, 0xBC8F8F,
                                        0x228B22, 0xEE82EE, 0xFF00FF, 0xFF0000, 0xFFB6C1, 0x20B2AA,
                                        0xFFFAF0, 0xD8BFD8, 0x0000FF, 0xA52A2A, 0xFFFFE0, 0xF5FFFA,
                                        0x9370DB, 0x2E8B57, 0x9932CC, 0xFFFFF0, 0x87CEEB, 0x48D1CC,
                                        0xEEE8AA, 0x00008B, 0xF0E68C, 0x3CB371, 0x87CEFA, 0xE0FFFF,
                                        0xADD8E6, 0xFF8C00, 0x00FF00, 0xFF69B4, 0xDC143C, 0xBDB76B,
                                        0x006400, 0xF08080, 0xFFFAFA, 0xFF4500, 0xDCDCDC, 0x708090,
                                        0xFDF5E6, 0x808000, 0xC0C0C0, 0x7B68EE,
                                    ];

                                    let color = {
                                        let c = RANDOM_COLORS
                                            [model_hash.0 as usize % RANDOM_COLORS.len()];

                                        let r = (c >> 16) as u8;
                                        let g = (c >> 8) as u8;
                                        let b = c as u8;

                                        Vec3::new(r as f32 / 255., g as f32 / 255., b as f32 / 255.)
                                    };

                                    let bmap = device_context
                                        .Map(&le_model_cbuffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
                                        .context("Failed to map model cbuffer")
                                        .unwrap();

                                    bmap.pData.copy_from_nonoverlapping(
                                        &(model_matrix, color) as *const (Mat4, Vec3) as _,
                                        std::mem::size_of::<(Mat4, Vec3)>(),
                                    );

                                    device_context.Unmap(&le_model_cbuffer, 0);
                                    device_context.VSSetConstantBuffers(
                                        1,
                                        Some(&[Some(le_model_cbuffer.clone())]),
                                    );

                                    let cb11_data = vec![Vec4::ONE; 64];
                                    let bmap = device_context
                                        .Map(&le_model_cb11, 0, D3D11_MAP_WRITE_DISCARD, 0)
                                        .context("Failed to map model cbuffer11")
                                        .unwrap();

                                    bmap.pData.copy_from_nonoverlapping(
                                        cb11_data.as_ptr() as _,
                                        std::mem::size_of::<Vec4>() * 64,
                                    );

                                    device_context.Unmap(&le_model_cb11, 0);
                                    device_context.VSSetConstantBuffers(
                                        11,
                                        Some(&[Some(le_model_cb11.clone())]),
                                    );

                                    model.draw(
                                        &device_context,
                                        &material_map,
                                        &vshader_map,
                                        &pshader_map,
                                        &texture_map,
                                        tex_i,
                                    );
                                }
                            }
                        }
                    }

                    swap_chain.Present(1, 0).unwrap()
                };
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => (),
        }
    });

    Ok(())
}
