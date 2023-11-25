use crate::render::drawcall::ShaderStages;
use crate::render::DeviceContextSwapchain;
use anyhow::Context;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D11::*;

#[derive(Clone)]
pub struct ConstantBuffer<T: Sized> {
    dcs: Arc<DeviceContextSwapchain>,
    buffer: ID3D11Buffer,
    /// Size of the buffer, in bytes
    size: usize,
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
                initial_data.map(|d| {
                    let sr = Box::new(D3D11_SUBRESOURCE_DATA {
                        pSysMem: d as *const T as _,
                        SysMemPitch: std::mem::size_of::<T>() as _,
                        ..Default::default()
                    });

                    Box::into_raw(sr) as _
                }),
            )?;

            let b = Self {
                dcs,
                buffer,
                size: std::mem::size_of::<T>(),
                _marker: Default::default(),
            };

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
                size: std::mem::size_of_val(initial_data),
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
    //             .context()
    //             .Map(&self.buffer, 0, D3D11_MAP_WRITE_DISCARD, 0)
    //             .context("Failed to map ConstantBuffer for writing (array)")?;

    //         memory
    //             .pData
    //             .copy_from_nonoverlapping(data.as_ptr() as _, std::mem::size_of_val(data));

    //         self.dcs.context().Unmap(&self.buffer, 0);
    //     }

    //     Ok(())
    // }

    pub fn map(&self, mode: D3D11_MAP) -> anyhow::Result<BufferMapGuard<T>> {
        let ptr = unsafe {
            self.dcs
                .context()
                .Map(&self.buffer, 0, mode, 0)
                .context("Failed to map ConstantBuffer")?
        };

        Ok(BufferMapGuard {
            ptr: ptr.pData as _,
            buffer: self.buffer.clone(),
            dcs: self.dcs.clone(),
        })
    }

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

            if stages.contains(ShaderStages::GEOMETRY) {
                self.dcs
                    .context()
                    .GSSetConstantBuffers(slot, Some(&[Some(self.buffer.clone())]))
            }
        }
    }

    // /// The size of the buffer, in bytes
    // pub fn size(&self) -> usize {
    //     self.size
    // }

    // /// The size of the buffer in elements (size / sizeof(T))
    // pub fn elements(&self) -> usize {
    //     self.size() / std::mem::size_of::<T>()
    // }
}

pub struct BufferMapGuard<T: Sized> {
    pub ptr: *mut T,
    buffer: ID3D11Buffer,
    dcs: Arc<DeviceContextSwapchain>,
}

impl<T> Drop for BufferMapGuard<T> {
    fn drop(&mut self) {
        unsafe { self.dcs.context().Unmap(&self.buffer, 0) }
    }
}

pub struct ConstantBufferCached<T: Sized> {
    data: Vec<T>,
    cbuffer: ConstantBuffer<T>,
    updated: AtomicBool,
}

impl<T: Sized + Clone> ConstantBufferCached<T> {
    pub fn create_array_init(
        dcs: Arc<DeviceContextSwapchain>,
        initial_data: &[T],
    ) -> anyhow::Result<Self> {
        Ok(Self {
            cbuffer: ConstantBuffer::create_array_init(dcs, initial_data)?,
            data: initial_data.to_vec(),
            updated: AtomicBool::new(false),
        })
    }

    /// Writes the buffer data to the GPU
    pub fn write(&self) -> anyhow::Result<()> {
        if self.updated.load(Ordering::Relaxed) {
            let map = self.cbuffer.map(D3D11_MAP_WRITE_DISCARD)?;
            unsafe { map.ptr.copy_from(self.data.as_ptr(), self.data.len()) };
            self.updated.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    // Deny clippy from the realms of dark magic (the good kind)
    #[allow(clippy::mut_from_ref)]
    pub fn data(&self) -> &mut [T] {
        self.updated.store(true, Ordering::Relaxed);
        unsafe { std::slice::from_raw_parts_mut(self.data.as_ptr() as *mut T, self.data.len()) }
    }

    pub fn bind(&self, slot: u32, stages: ShaderStages) {
        // Make sure the buffer is written before we bind it
        // Its fine to call this multiple times per draw, as we keep track of whether the buffer has been acquired before we write
        self.write().ok();
        self.cbuffer.bind(slot, stages)
    }
}
