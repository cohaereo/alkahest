use windows::{
    core::PCSTR,
    Win32::Graphics::{
        Direct3D::Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION},
        Direct3D11::{ID3D11PixelShader, ID3D11VertexShader},
    },
};

use super::DeviceContextSwapchain;

pub fn compile_hlsl(source: &str, entrypoint: &str, target: &str) -> Result<Vec<u8>, String> {
    let mut shader = None;
    let mut errors = None;

    let flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };

    let entrypoint_cstr = format!("{entrypoint}\0");
    let entrypoint = PCSTR::from_raw(entrypoint_cstr.as_ptr());
    let target_cstr = format!("{target}\0");
    let target = PCSTR::from_raw(target_cstr.as_ptr());
    let result = unsafe {
        D3DCompile(
            source.as_ptr() as _,
            source.len(),
            None,
            None,
            None,
            entrypoint,
            target,
            flags,
            0,
            &mut shader,
            Some(&mut errors),
        )
    };

    let mut error_string = String::new();
    if let Some(errors) = errors {
        let estr = unsafe {
            let eptr = errors.GetBufferPointer();
            std::slice::from_raw_parts(eptr.cast(), errors.GetBufferSize())
        };
        let errors = String::from_utf8_lossy(estr);
        error_string = errors.to_string();
    }

    if result.is_err() {
        return Err(error_string);
    }

    if !error_string.is_empty() {
        warn!("{error_string}");
    }

    let blob = shader.unwrap();

    let vs_blob = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
    };

    Ok(vs_blob.to_vec())
}

pub fn load_vshader(
    dcs: &DeviceContextSwapchain,
    data: &[u8],
) -> anyhow::Result<ID3D11VertexShader> {
    Ok(unsafe { dcs.device.CreateVertexShader(data, None)? })
}

pub fn load_pshader(
    dcs: &DeviceContextSwapchain,
    data: &[u8],
) -> anyhow::Result<ID3D11PixelShader> {
    Ok(unsafe { dcs.device.CreatePixelShader(data, None)? })
}
