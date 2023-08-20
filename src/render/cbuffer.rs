use crate::render::DeviceContextSwapchain;
use anyhow::Context;
use std::marker::PhantomData;
use std::rc::Rc;
use windows::Win32::Graphics::Direct3D11::*;

pub struct ConstantBuffer<T: Sized> {
    dcs: Rc<DeviceContextSwapchain>,
    buffer: ID3D11Buffer,
    _marker: PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(
        dcs: Rc<DeviceContextSwapchain>,
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
                    ..Default::default()
                } as *const D3D11_SUBRESOURCE_DATA),
            )?;

            Ok(Self {
                dcs,
                buffer,
                _marker: Default::default(),
            })
        }
    }

    // pub fn create_array(dcs: Rc<DeviceContextSwapchain>, length: usize) -> anyhow::Result<Self> {
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
        dcs: Rc<DeviceContextSwapchain>,
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
                .context
                .Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
                .context("Failed to map ConstantBuffer for writing")?;

            memory
                .pData
                .copy_from_nonoverlapping(data as *const T as _, std::mem::size_of::<T>());

            self.dcs.context.Unmap(&self.buffer, 0);
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

    //         self.dcs.context.Unmap(&self.buffer, 0);
    //     }

    //     Ok(())
    // }

    pub fn buffer(&self) -> &ID3D11Buffer {
        &self.buffer
    }
}
