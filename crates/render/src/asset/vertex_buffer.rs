use alkahest_data::tfx::buffers::VertexBufferHeader;
use anyhow::Context;
use d3d11::{dxgi, BindFlags, BufferDesc, DeviceChild, DeviceContext, ShaderResourceViewDesc};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use crate::{gpu::command_list::CommandList, Gpu};

#[derive(Clone)]
pub struct VertexBuffer {
    pub buffer: d3d11::Buffer,
    pub size: u32,
    pub length: u32,
    pub stride: u32,
    /// Optional SRV for the buffer. Created for buffers with stride 1 and 4
    pub srv: Option<d3d11::ShaderResourceView>,
}

impl VertexBuffer {
    pub fn load(device: &d3d11::Device, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;

        let header: VertexBufferHeader = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read header data")?;
        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read buffer data")?;

        let vb = VertexBuffer::load_data(device, &data, header.stride as _)?;
        vb.buffer.set_debug_name(format!("VertexBuffer {hash}"));
        Ok(vb)
    }

    pub fn load_data(device: &d3d11::Device, data: &[u8], stride: u32) -> anyhow::Result<Self> {
        Self::load_data_ex(device, data, stride, false)
    }

    #[profiling::function]
    pub fn load_data_ex(
        device: &d3d11::Device,
        data: &[u8],
        stride: u32,
        cpu_writable: bool,
    ) -> anyhow::Result<Self> {
        let bind_flags = if matches!(stride, 1 | 4) {
            BindFlags::VERTEX_BUFFER | BindFlags::SHADER_RESOURCE
        } else {
            BindFlags::VERTEX_BUFFER
        };
        let buffer = device.create_buffer(
            &BufferDesc::builder()
                .byte_width(data.len() as u32)
                .usage(if cpu_writable {
                    d3d11::Usage::Dynamic
                } else {
                    d3d11::Usage::Default
                })
                .bind_flags(bind_flags)
                .cpu_access_flags(if cpu_writable {
                    d3d11::CpuAccessFlags::WRITE
                } else {
                    d3d11::CpuAccessFlags::empty()
                })
                .build(),
            Some(data),
        )?;

        let srv = if matches!(stride, 1 | 4) {
            Some(
                device.create_shader_resource_view(
                    &buffer,
                    Some(
                        &ShaderResourceViewDesc::builder()
                            .format(if stride == 1 {
                                dxgi::Format::R8Unorm
                            } else {
                                dxgi::Format::R8g8b8a8Unorm
                            })
                            .view_dimension(d3d11::srv::SrvDimension::Buffer {
                                first_element_or_element_offset: 0,
                                num_elements_or_element_width: (data.len() / stride as usize)
                                    as u32,
                            })
                            .build(),
                    ),
                )?,
            )
        } else {
            None
        };

        Ok(VertexBuffer {
            buffer,
            size: data.len() as u32,
            length: data.len() as u32 / stride,
            stride,
            srv,
        })
    }

    pub fn bind_single(&self, cmd: &mut CommandList, slot: u32) {
        cmd.input_assembler_set_vertex_buffers(
            slot,
            &[Some(&self.buffer)],
            Some(&[self.stride]),
            Some(&[0]),
        )
        .expect("Failed to bind vertex buffer");
    }

    /// # Safety
    ///
    /// The caller must ensure that the data fits within the buffer.
    pub unsafe fn write(&self, cmd: &DeviceContext, data: &[u8]) -> anyhow::Result<()> {
        let m = cmd.map_unchecked(&self.buffer, 0, d3d11::MapType::WriteDiscard, false)?;
        m.data.copy_from(data.as_ptr() as _, data.len());
        cmd.unmap(&self.buffer, 0);

        Ok(())
    }
}

pub(crate) fn load_vertex_buffer(
    device: &d3d11::Device,
    hash: TagHash,
) -> anyhow::Result<VertexBuffer> {
    VertexBuffer::load(device, hash)
}
