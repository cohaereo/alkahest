use windows::Win32::Graphics::Direct3D11::{
    ID3D11GeometryShader, ID3D11PixelShader, ID3D11VertexShader,
};

use crate::gpu::GpuContext;

pub struct ShaderProgram {
    pub vs: ID3D11VertexShader,
    pub gs: Option<ID3D11GeometryShader>,
    pub ps: ID3D11PixelShader,
}

impl ShaderProgram {
    pub fn load(
        gctx: &GpuContext,
        vs: &[u8],
        gs: Option<&[u8]>,
        ps: &[u8],
    ) -> anyhow::Result<Self> {
        let vs = unsafe {
            let mut vs_prog = None;
            gctx.device
                .CreateVertexShader(vs, None, Some(&mut vs_prog))?;
            vs_prog.unwrap()
        };

        let gs = gs.map(|gs| unsafe {
            let mut gs_prog = None;
            gctx.device
                .CreateGeometryShader(gs, None, Some(&mut gs_prog))
                .unwrap();
            gs_prog.unwrap()
        });

        let ps = unsafe {
            let mut ps_prog = None;
            gctx.device
                .CreatePixelShader(ps, None, Some(&mut ps_prog))?;
            ps_prog.unwrap()
        };

        Ok(Self { vs, gs, ps })
    }

    pub fn bind(&self, gctx: &GpuContext) {
        unsafe {
            gctx.context().VSSetShader(&self.vs, None);
            if let Some(gs) = &self.gs {
                gctx.context().GSSetShader(gs, None);
            }
            gctx.context().PSSetShader(&self.ps, None);
        }
    }
}
