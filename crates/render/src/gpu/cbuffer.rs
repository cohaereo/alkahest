use std::marker::PhantomData;

use alkahest_data::tfx::ShaderStage;
use anyhow::Context;
use d3d11::{
    dxgi,
    srv::{SrvBufferExFlags, SrvDimension},
    BindFlags, BufferDesc, CpuAccessFlags, ResourceMiscFlags, ShaderResourceViewDesc,
    SubresourceMapGuard, Usage,
};

use super::{command_list::CommandList, Gpu};
use crate::gpu::command_list::ContextExt;

pub struct ConstantBuffer<T: Sized> {
    buffer: d3d11::Buffer,
    /// SRVs for use with raw byte buffers
    srv: Option<d3d11::ShaderResourceView>,
    size: usize,
    _marker: PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(gpu: &Gpu, initial_data: Option<&T>) -> anyhow::Result<Self> {
        let size_aligned = (std::mem::size_of::<T>() + 15) & !15;

        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .usage(Usage::Dynamic)
                .bind_flags(BindFlags::CONSTANT_BUFFER)
                .cpu_access_flags(CpuAccessFlags::WRITE)
                .byte_width(size_aligned as u32)
                .build(),
            initial_data.map(|d| unsafe {
                &*std::ptr::slice_from_raw_parts(
                    d as *const T as *const u8,
                    std::mem::size_of::<T>(),
                )
            }),
        )?;

        Ok(Self {
            buffer,
            srv: None,
            size: size_aligned,
            _marker: Default::default(),
        })
    }

    pub fn create_array(
        gpu: &Gpu,
        count: usize,
        initial_data: Option<&[T]>,
    ) -> anyhow::Result<Self> {
        let size_aligned = ((std::mem::size_of::<T>() * count) + 15) & !15;
        let initial_data_ffi = if let Some(initial_data) = initial_data {
            anyhow::ensure!(
                initial_data.len() == count,
                "Initial data length does not match count"
            );

            Some(unsafe {
                &*std::ptr::slice_from_raw_parts(
                    initial_data.as_ptr() as *const u8,
                    std::mem::size_of::<T>() * count,
                )
            })
        } else {
            None
        };

        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .usage(Usage::Dynamic)
                .bind_flags(BindFlags::CONSTANT_BUFFER)
                .cpu_access_flags(CpuAccessFlags::WRITE)
                .byte_width(size_aligned as u32)
                .build(),
            initial_data_ffi,
        )?;

        Ok(Self {
            buffer,
            size: size_aligned,
            srv: None,
            _marker: Default::default(),
        })
    }

    pub fn create_raw(gpu: &Gpu, size: usize) -> anyhow::Result<Self> {
        let size_aligned = (size + 15) & !15;

        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .usage(Usage::Dynamic)
                .bind_flags(BindFlags::SHADER_RESOURCE)
                .misc_flags(ResourceMiscFlags::BUFFER_ALLOW_RAW_VIEWS)
                .cpu_access_flags(CpuAccessFlags::WRITE)
                .byte_width(size_aligned as u32)
                .build(),
            None,
        )?;

        let srv = gpu.create_shader_resource_view(
            &buffer,
            &ShaderResourceViewDesc::builder()
                .format(dxgi::Format::R32Typeless)
                .view_dimension(SrvDimension::BufferEx {
                    first_element: 0,
                    num_elements: (size_aligned / 4) as u32,
                    flags: SrvBufferExFlags::RAW,
                })
                .build(),
        )?;

        Ok(Self {
            buffer,
            size: size_aligned,
            srv: Some(srv),
            _marker: Default::default(),
        })
    }

    pub fn write(&self, ctx: &d3d11::DeviceContext, data: &T) -> anyhow::Result<()> {
        unsafe {
            let map = self.map(ctx, d3d11::MapType::WriteDiscard)?;
            map.data
                .copy_from_nonoverlapping(data as *const T as _, std::mem::size_of::<T>());
        }
        Ok(())
    }

    /// # Safety
    /// The caller must ensure that the length of the slice matches the size of the buffer.
    pub unsafe fn write_array(&self, ctx: &d3d11::DeviceContext, data: &[T]) -> anyhow::Result<()> {
        let map = self.map(ctx, d3d11::MapType::WriteDiscard)?;
        map.data
            .copy_from_nonoverlapping(data.as_ptr() as _, std::mem::size_of_val(data));
        Ok(())
    }

    pub fn map(
        &self,
        ctx: &d3d11::DeviceContext,
        mode: d3d11::MapType,
    ) -> anyhow::Result<SubresourceMapGuard<d3d11::Buffer>> {
        let ptr = {
            profiling::scope!("map");
            ctx.map(&self.buffer, 0, mode, false)
                .context("Failed to map ConstantBuffer")?
        };

        Ok(ptr)
    }

    pub fn map_slice(
        &self,
        ctx: &d3d11::DeviceContext,
        mode: d3d11::MapType,
        f: impl FnOnce(&mut [T]),
    ) -> anyhow::Result<()> {
        unsafe {
            let map = self.map(ctx, mode)?;
            let slice = std::slice::from_raw_parts_mut(
                map.data as *mut T,
                self.size / std::mem::size_of::<T>(),
            );
            f(slice);
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn buffer(&self) -> &d3d11::Buffer {
        &self.buffer
    }

    pub fn bind(&self, cmd: &mut CommandList, stage: ShaderStage, slot: u32) {
        if self.srv.is_some() {
            self.bind_srv(cmd, stage, slot);
        } else {
            self.bind_cbuffer(cmd, stage, slot);
        }
    }

    fn bind_srv(&self, cmd: &mut CommandList, stage: ShaderStage, slot: u32) {
        cmd.set_shader_resource(stage, slot, &self.srv);
    }

    fn bind_cbuffer(&self, cmd: &mut CommandList, stage: ShaderStage, slot: u32) {
        cmd.set_constant_buffer(stage, slot, &self.buffer);
    }
}
