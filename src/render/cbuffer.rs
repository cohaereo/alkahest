use crate::render::drawcall::ShaderStages;
use crate::render::DeviceContextSwapchain;
use anyhow::Context;
use std::marker::PhantomData;
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D11::*;

pub struct ConstantBuffer<T: Sized> {
    dcs: Arc<DeviceContextSwapchain>,
    buffer: ID3D11Buffer,
    _marker: PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(
        dcs: Arc<DeviceContextSwapchain>,
        initial_data: Option<&T>,
    ) -> anyhow::Result<Self> {
        unsafe {
            let buffer = dcs.device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    ByteWidth: std::mem::size_of::<T>() as _,
                    ..Default::default()
                },
                initial_data.map(|d| &D3D11_SUBRESOURCE_DATA {
                    pSysMem: d as *const T as _,
                    SysMemPitch: std::mem::size_of::<T>() as _,
                    ..Default::default()
                } as *const D3D11_SUBRESOURCE_DATA),
            )?;

            let b = Self {
                dcs,
                buffer,
                _marker: Default::default(),
            };

            // FIXME: initial data does not work
            if let Some(d) = initial_data {
                b.write(d)?
            }

            Ok(b)
        }
    }

    // pub fn create_array(dcs: Arc<DeviceContextSwapchain>, length: usize) -> anyhow::Result<Self> {
    //     unsafe {
    //         let buffer = dcs.device.CreateBuffer(
    //             &D3D11_BUFFER_DESC {
    //                 Usage: D3D11_USAGE_DYNAMIC,
    //                 BindFlags: D3D11_BIND_CONSTANT_BUFFER,
    //                 CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
    //                 ByteWidth: (length * std::mem::size_of::<T>()) as _,
    //                 ..Default::default()
    //             },
    //             None,
    //         )?;

    //         Ok(Self {
    //             dcs,
    //             buffer,
    //             _marker: Default::default(),
    //         })
    //     }
    // }

    pub fn create_array_init(
        dcs: Arc<DeviceContextSwapchain>,
        initial_data: &[T],
    ) -> anyhow::Result<Self> {
        unsafe {
            let buffer = dcs.device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
                    ByteWidth: std::mem::size_of_val(initial_data) as _,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: initial_data.as_ptr() as _,
                    SysMemPitch: std::mem::size_of::<T>() as _,
                    ..Default::default()
                }),
            )?;

            Ok(Self {
                dcs,
                buffer,
                _marker: Default::default(),
            })
        }
    }

    pub fn write(&self, data: &T) -> anyhow::Result<()> {
        unsafe {
            let memory = self
                .dcs
                .context()
                .Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
                .context("Failed to map ConstantBuffer for writing")?;

            memory
                .pData
                .copy_from_nonoverlapping(data as *const T as _, std::mem::size_of::<T>());

            self.dcs.context().Unmap(&self.buffer, 0);
        }

        Ok(())
    }

    // pub fn write_array(&self, data: &[T]) -> anyhow::Result<()> {
    //     unsafe {
    //         let memory = self
    //             .dcs
    //             .context
    //             .Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
    //             .context("Failed to map ConstantBuffer for writing (array)")?;

    //         memory
    //             .pData
    //             .copy_from_nonoverlapping(data.as_ptr() as _, std::mem::size_of_val(data));

    //         self.dcs.context().Unmap(&self.buffer, 0);
    //     }

    //     Ok(())
    // }

    pub fn buffer(&self) -> &ID3D11Buffer {
        &self.buffer
    }

    pub fn bind(&self, slot: u32, stages: ShaderStages) {
        unsafe {
            if stages.contains(ShaderStages::VERTEX) {
                self.dcs
                    .context()
                    .VSSetConstantBuffers(slot, Some(&[Some(self.buffer.clone())]))
            }

            if stages.contains(ShaderStages::PIXEL) {
                self.dcs
                    .context()
                    .PSSetConstantBuffers(slot, Some(&[Some(self.buffer.clone())]))
            }

            if stages.contains(ShaderStages::COMPUTE) {
                self.dcs
                    .context()
                    .CSSetConstantBuffers(slot, Some(&[Some(self.buffer.clone())]))
            }
        }
    }
}
