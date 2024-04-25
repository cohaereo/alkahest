use windows::Win32::Graphics::{Direct3D::WKPDID_D3DDebugObjectName, Direct3D11::ID3D11Resource};

#[inline]
pub fn calc_dx_subresource(mip_slice: usize, array_slice: usize, mip_levels: usize) -> usize {
    mip_slice + array_slice * mip_levels
}

pub trait D3dResource {
    fn set_debug_name(&self, name: &str);
}

impl<I: windows::core::Interface> D3dResource for I {
    fn set_debug_name(&self, name: &str) {
        let name_cstr = std::ffi::CString::new(name).unwrap();
        if let Ok(resource) = self.cast::<ID3D11Resource>() {
            unsafe {
                resource
                    .SetPrivateData(
                        &WKPDID_D3DDebugObjectName,
                        name_cstr.to_bytes().len() as _,
                        Some(name_cstr.as_ptr() as _),
                    )
                    .ok();
            }
        }
    }
}
