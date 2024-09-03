use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName, Direct3D11::ID3D11DeviceChild,
};

use crate::gpu::GpuContext;

#[inline]
pub fn calc_dx_subresource(mip_slice: usize, array_slice: usize, mip_levels: usize) -> usize {
    mip_slice + array_slice * mip_levels
}

pub trait D3dResource {
    fn set_debug_name(&self, name: &str);
}

impl D3dResource for ID3D11DeviceChild {
    fn set_debug_name(&self, name: &str) {
        let name_cstr = std::ffi::CString::new(name).unwrap();
        unsafe {
            if let Err(e) = self.SetPrivateData(
                &WKPDID_D3DDebugObjectName,
                name_cstr.to_bytes().len() as _,
                Some(name_cstr.as_ptr() as _),
            ) {
                warn!("Failed to set D3D11 object debug name '{name}': {e:?}");
            }
        }
    }
}

pub trait ErrorExt {
    /// Append the last device error, if any, to the error chain.
    fn with_d3d_error(self, gpu: &GpuContext) -> Self;
}

impl ErrorExt for anyhow::Error {
    fn with_d3d_error(self, gpu: &GpuContext) -> Self {
        if let Some(gerr) = gpu.last_device_error() {
            self.context(gerr)
        } else {
            self
        }
    }
}

pub fn try_out_ptr<T, F, E, O>(mut f: F) -> Result<T, E>
where
    F: FnMut(&mut Option<T>) -> Result<O, E>,
{
    let mut t: Option<T> = None;
    match f(&mut t) {
        Ok(_) => Ok(t.unwrap()),
        Err(e) => Err(e),
    }
}
