use std::{io::Write, slice::from_raw_parts};

use windows::{
    core::{s, PCSTR},
    Win32::Graphics::{
        Direct3D::{
            Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS},
            ID3DBlob,
        },
        Direct3D11::{ID3D11Device, ID3D11PixelShader, ID3D11VertexShader},
    },
};

use crate::RenderError;

trait Shader: Sized {
    const ENTRY: PCSTR;
    const TARGET: PCSTR;

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Result<Self, RenderError>;
}

impl Shader for ID3D11VertexShader {
    const ENTRY: PCSTR = s!("vs_main");
    const TARGET: PCSTR = s!("vs_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Result<Self, RenderError> {
        let mut output = None;
        match blob {
            ShaderData::EmbeddedData(arr) => {
                device.CreateVertexShader(arr, None, Some(&mut output))?;
                output.ok_or(RenderError::General(
                    "Unable to create vertex shader for embedded data",
                ))
            }
            ShaderData::CompiledBlob(blob) => {
                device.CreateVertexShader(
                    from_raw_parts(blob.GetBufferPointer() as _, blob.GetBufferSize()),
                    None,
                    Some(&mut output),
                )?;
                output.ok_or(RenderError::General(
                    "Unable to create vertex shader for compiled blob",
                ))
            }
        }
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY: PCSTR = s!("ps_main");
    const TARGET: PCSTR = s!("ps_5_0");

    unsafe fn create_shader(device: &ID3D11Device, blob: &ShaderData) -> Result<Self, RenderError> {
        let mut output = None;
        match blob {
            ShaderData::EmbeddedData(arr) => {
                device.CreatePixelShader(arr, None, Some(&mut output))?;
                output.ok_or(RenderError::General(
                    "Unable to create pixel shader for embedded data",
                ))
            }
            ShaderData::CompiledBlob(blob) => {
                device.CreatePixelShader(
                    from_raw_parts(blob.GetBufferPointer() as _, blob.GetBufferSize()),
                    None,
                    Some(&mut output),
                )?;
                output.ok_or(RenderError::General(
                    "Unable to create pixel shader for compiled blob",
                ))
            }
        }
    }
}

pub enum ShaderData {
    EmbeddedData(&'static [u8]),
    CompiledBlob(ID3DBlob),
}

pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    cache: ShaderData,
}

impl CompiledShaders {
    pub fn new(device: &ID3D11Device) -> Result<Self, RenderError> {
        if cfg!(feature = "force-compile") {
            let (vcache, vertex) = Self::compile_shader::<ID3D11VertexShader>(device)?;
            let (_pcache, pixel) = Self::compile_shader::<ID3D11PixelShader>(device)?;

            if cfg!(feature = "save-blob") {
                unsafe {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .create(true)
                        .truncate(true)
                        .open("vertex.bin")
                        .unwrap()
                        .write_all(std::slice::from_raw_parts(
                            vcache.GetBufferPointer() as *mut u8,
                            vcache.GetBufferSize(),
                        ))
                        .unwrap();

                    std::fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .create(true)
                        .truncate(true)
                        .open("pixel.bin")
                        .unwrap()
                        .write_all(std::slice::from_raw_parts(
                            _pcache.GetBufferPointer() as *mut u8,
                            _pcache.GetBufferSize(),
                        ))
                        .unwrap();
                }
            }

            Ok(Self {
                vertex,
                pixel,
                cache: ShaderData::CompiledBlob(vcache),
            })
        } else {
            unsafe {
                let cache = ShaderData::EmbeddedData(include_bytes!("vertex.bin"));
                let vertex = ID3D11VertexShader::create_shader(device, &cache)?;
                let pixel = ID3D11PixelShader::create_shader(
                    device,
                    &ShaderData::EmbeddedData(include_bytes!("pixel.bin")),
                )?;

                Ok(Self {
                    cache,
                    vertex,
                    pixel,
                })
            }
        }
    }

    pub fn bytecode(&self) -> &[u8] {
        match &self.cache {
            ShaderData::EmbeddedData(arr) => arr,
            ShaderData::CompiledBlob(blob) => unsafe {
                from_raw_parts(blob.GetBufferPointer() as _, blob.GetBufferSize())
            },
        }
    }

    fn compile_shader<S: Shader>(device: &ID3D11Device) -> Result<(ID3DBlob, S), RenderError> {
        const SHADER_TEXT: &str = include_str!("shader.hlsl");

        let mut flags = D3DCOMPILE_ENABLE_STRICTNESS;
        if cfg!(debug_assertions) {
            flags |= D3DCOMPILE_DEBUG;
        }

        let mut code = None;
        let mut error = None;

        unsafe {
            if D3DCompile(
                SHADER_TEXT.as_ptr() as _,
                SHADER_TEXT.len(),
                None,
                None,
                None,
                S::ENTRY,
                S::TARGET,
                flags,
                0,
                &mut code,
                Some(&mut error),
            )
            .is_err()
            {
                if !cfg!(feature = "no-msgs") {
                    panic!(
                        "{}",
                        std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                            error.as_ref().unwrap().GetBufferPointer() as *const u8,
                            error.as_ref().unwrap().GetBufferSize(),
                        ))
                    );
                } else {
                    panic!();
                }
            } else {
                Ok((
                    code.clone().unwrap(),
                    S::create_shader(device, &ShaderData::CompiledBlob(code.unwrap()))?,
                ))
            }
        }
    }
}
