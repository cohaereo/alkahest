use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName, Direct3D11::ID3D11DeviceChild,
};

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
