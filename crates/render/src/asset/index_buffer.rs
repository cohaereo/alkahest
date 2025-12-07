use alkahest_data::tfx::buffers::IndexBufferHeader;
use anyhow::Context;
use d3d11::{dxgi, BindFlags, BufferDesc, DeviceChild, Usage};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

use crate::{gpu::command_list::CommandList, Gpu};

#[derive(Clone)]
pub struct IndexBuffer {
    pub buffer: d3d11::Buffer,
    /// Amount of elements in the buffer
    pub length: usize,
    pub format: dxgi::Format,
}

impl IndexBuffer {
    pub fn load(device: &d3d11::Device, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;

        let header: IndexBufferHeader = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read header data")?;
        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read buffer data")?;

        let buffer = device.create_buffer(
            &BufferDesc::builder()
                .byte_width(header.data_size as u32)
                .usage(Usage::Immutable)
                .bind_flags(BindFlags::INDEX_BUFFER)
                .build(),
            Some(&data),
        )?;

        buffer.set_debug_name(format!("IndexBuffer: {hash}"));

        Ok(IndexBuffer {
            buffer,
            length: header.data_size as usize / if header.is_32bit { 4 } else { 2 },
            format: if header.is_32bit {
                dxgi::Format::R32Uint
            } else {
                dxgi::Format::R16Uint
            },
        })
    }

    pub fn load_u16(gpu: &Gpu, data: &[u16]) -> anyhow::Result<Self> {
        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .byte_width(std::mem::size_of_val(data) as u32)
                .usage(Usage::Immutable)
                .bind_flags(BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(data)),
        )?;

        Ok(Self {
            buffer,
            length: data.len(),
            format: dxgi::Format::R16Uint,
        })
    }

    pub fn load_u32(gpu: &Gpu, data: &[u32]) -> anyhow::Result<Self> {
        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .byte_width(std::mem::size_of_val(data) as u32)
                .usage(Usage::Immutable)
                .bind_flags(BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(data)),
        )?;

        Ok(Self {
            buffer,
            length: data.len(),
            format: dxgi::Format::R32Uint,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        cmd.input_assembler_set_index_buffer(&self.buffer, self.format, 0);
    }
}

pub(crate) fn load_index_buffer(gctx: &Gpu, hash: TagHash) -> anyhow::Result<IndexBuffer> {
    IndexBuffer::load(&gctx.device, hash)
}
