use std::marker::PhantomData;

use anyhow::Context;
use bon::Builder;
use d3d11::{BindFlags, BufferDesc, UavDimension, dxgi};

pub struct GpuBuffer<T: TypedBufferFormat> {
    pub desc: GpuBufferDesc,
    pub gpu_buffer: d3d11::Buffer,
    pub uav: d3d11::UnorderedAccessView,
    pub staging_buffer: d3d11::Buffer,
    _type: PhantomData<T>, // Why yes, we're totally using T. Don't worry about it
}

impl<T: TypedBufferFormat> GpuBuffer<T> {
    pub fn new(device: &d3d11::Device, desc: GpuBufferDesc) -> anyhow::Result<Self> {
        let byte_width = (std::mem::size_of::<T>() as u32) * desc.capacity;

        let buffer_desc = d3d11::BufferDesc::builder()
            .byte_width(byte_width)
            .bind_flags(BindFlags::UNORDERED_ACCESS)
            .build();

        let gpu_buffer = device
            .create_buffer(&buffer_desc, None)
            .context(format!("Failed to create GpuBuffer {}", desc.label))?;

        let uav_desc = d3d11::UnorderedAccessViewDesc::builder()
            .format(T::FORMAT)
            .view_dimension(UavDimension::Buffer {
                first_element: 0,
                num_elements: desc.capacity,
                flags: 0,
            })
            .build();

        let uav = device
            .create_unordered_access_view(&gpu_buffer, &uav_desc)
            .context(format!("Failed to create uav for {}", desc.label))?;

        let staging_buffer_desc = BufferDesc::builder()
            .byte_width(byte_width)
            .usage(d3d11::Usage::Staging)
            .cpu_access_flags(d3d11::CpuAccessFlags::READ)
            .bind_flags(BindFlags::empty())
            .build();
        let staging_buffer = device
            .create_buffer(&staging_buffer_desc, None)
            .context(format!(
                "Failed to create staging buffer for {}",
                desc.label
            ))?;

        Ok(Self {
            desc: desc.clone(),
            gpu_buffer,
            uav,
            staging_buffer,
            _type: PhantomData,
        })
    }

    pub fn copy_to_staging(&self, context: &d3d11::DeviceContext) {
        context.copy_resource(&self.gpu_buffer, &self.staging_buffer);
    }

    pub fn read_element(&self, context: &d3d11::DeviceContext, index: u32) -> Option<T> {
        if index < self.desc.capacity {
            let map = context
                .map(&self.staging_buffer, 0, d3d11::MapType::Read, false)
                .context(format!(
                    "Failed to read from staging buffer for {}",
                    self.desc.label
                ))
                .ok()?;
            Some(unsafe { *map.data.cast::<T>().add(index as usize) })
        } else {
            None
        }
    }
}

#[derive(Builder, Clone)]
pub struct GpuBufferDesc {
    #[builder(start_fn, into)]
    pub label: String,

    #[builder(default = 1)]
    pub capacity: u32,
}

impl GpuBufferDesc {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.label = name.into();
        self
    }
}

pub trait TypedBufferFormat: Copy {
    const FORMAT: dxgi::Format;
}

impl TypedBufferFormat for f32 {
    const FORMAT: dxgi::Format = dxgi::Format::R32Float;
}

impl TypedBufferFormat for [f32; 2] {
    const FORMAT: dxgi::Format = dxgi::Format::R32g32Float;
}

impl TypedBufferFormat for [f32; 3] {
    const FORMAT: dxgi::Format = dxgi::Format::R32g32b32Float;
}

impl TypedBufferFormat for [f32; 4] {
    const FORMAT: dxgi::Format = dxgi::Format::R32g32b32a32Float;
}

impl TypedBufferFormat for u32 {
    const FORMAT: dxgi::Format = dxgi::Format::R32Uint;
}

impl TypedBufferFormat for [u32; 2] {
    const FORMAT: dxgi::Format = dxgi::Format::R32g32Uint;
}
// Can add more here if we actually need any
// Maybe extend to structured buffs if we need em?
