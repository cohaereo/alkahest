use windows::Win32::Graphics::Direct3D11::{ID3D11DepthStencilView, ID3D11RenderTargetView};

use crate::gpu::GpuContext;

pub struct D3DState {
    pub render_targets: [Option<ID3D11RenderTargetView>; 8],
    pub depth_stencil_view: Option<ID3D11DepthStencilView>,
}

impl GpuContext {
    pub fn backup_state(&self) -> D3DState {
        let mut render_targets = [const { None }; 8];
        let mut depth_stencil_view = None;
        unsafe {
            self.context()
                .OMGetRenderTargets(Some(&mut render_targets), Some(&mut depth_stencil_view));
        }

        D3DState {
            render_targets,
            depth_stencil_view,
        }
    }

    pub fn restore_state(&self, state: &D3DState) {
        unsafe {
            if let Some(dsv) = &state.depth_stencil_view {
                self.context
                    .OMSetRenderTargets(Some(&state.render_targets), dsv);
            } else {
                self.context()
                    .OMSetRenderTargets(Some(&state.render_targets), None);
            }
        }
    }
}
