use crate::RenderError;

pub struct CompiledShaders {
    pub vertex: d3d11::VertexShader,
    pub pixel: d3d11::PixelShader,
    pub pixel_no_alpha: d3d11::PixelShader,
    vs_bytecode: Vec<u8>,
}

impl CompiledShaders {
    pub fn new(device: &d3d11::Device) -> Result<Self, RenderError> {
        let vs_bytecode = include_bytes!("vertex.bin").to_vec();
        let vertex = device.create_vertex_shader(&vs_bytecode)?;
        let pixel = device.create_pixel_shader(include_bytes!("pixel.bin"))?;
        let pixel_no_alpha = device.create_pixel_shader(include_bytes!("pixel_no_alpha.bin"))?;

        Ok(Self {
            vertex,
            pixel,
            pixel_no_alpha,
            vs_bytecode,
        })
    }

    pub fn vs_bytecode(&self) -> &[u8] {
        &self.vs_bytecode
    }
}
