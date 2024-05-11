use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use alkahest_data::tfx::TfxShaderStage;
use anyhow::Context;
use windows::{core::Interface, Win32::Graphics::Direct3D11::*};

use crate::gpu::{GpuContext, SharedGpuContext};

#[derive(Clone)]
pub struct ConstantBuffer<T: Sized> {
    gctx: Arc<GpuContext>,
    buffer: ID3D11Buffer,
    _marker: PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(gctx: Arc<GpuContext>, initial_data: Option<&T>) -> anyhow::Result<Self> {
        unsafe {
            let mut buffer = None;
            gctx.device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
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
                Some(&mut buffer),
            )?;

            let b = Self {
                gctx,
                buffer: buffer.unwrap(),
                _marker: Default::default(),
            };

            Ok(b)
        }
    }

    pub fn write(&self, data: &T) -> anyhow::Result<()> {
        self.map(D3D11_MAP_WRITE_DISCARD, |map| unsafe {
            map.pData
                .copy_from_nonoverlapping(data as *const T as _, std::mem::size_of::<T>());
        })
    }

    /// This function is unsafe because, unlike `write`, it doesn't check the size of the data
    pub unsafe fn write_array(&self, data: &[T]) -> anyhow::Result<()> {
        self.map(D3D11_MAP_WRITE_DISCARD, |map| {
            map.pData
                .copy_from_nonoverlapping(data.as_ptr() as _, std::mem::size_of_val(data));
        })
    }

    pub fn create_array_init(gctx: SharedGpuContext, initial_data: &[T]) -> anyhow::Result<Self> {
        let mut buffer = None;
        unsafe {
            gctx.device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as _,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as _,
                    ByteWidth: std::mem::size_of_val(initial_data) as _,
                    ..Default::default()
                },
                Some(&D3D11_SUBRESOURCE_DATA {
                    pSysMem: initial_data.as_ptr() as _,
                    SysMemPitch: std::mem::size_of::<T>() as _,
                    ..Default::default()
                }),
                Some(&mut buffer),
            )?;

            let buffer = buffer.unwrap();

            Ok(Self {
                gctx,
                buffer,
                _marker: Default::default(),
            })
        }
    }

    pub fn map(
        &self,
        mode: D3D11_MAP,
        f: impl FnOnce(D3D11_MAPPED_SUBRESOURCE),
    ) -> anyhow::Result<()> {
        unsafe {
            #[allow(clippy::uninit_assumed_init)]
            let mut ptr = std::mem::MaybeUninit::uninit().assume_init();
            self.gctx
                .context()
                .Map(&self.buffer, 0, mode, 0, Some(&mut ptr))
                .context("Failed to map ConstantBuffer")?;

            f(ptr);

            self.gctx.context().Unmap(&self.buffer, 0);

            Ok(())
        }
    }

    pub fn buffer(&self) -> &ID3D11Buffer {
        &self.buffer
    }

    pub fn bind(&self, slot: u32, stage: TfxShaderStage) {
        let ctx = self.gctx.context();
        unsafe {
            match stage {
                TfxShaderStage::Vertex => {
                    ctx.VSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
                TfxShaderStage::Pixel => {
                    ctx.PSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
                TfxShaderStage::Geometry => {
                    ctx.GSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
                TfxShaderStage::Hull => {
                    ctx.HSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
                TfxShaderStage::Domain => {
                    ctx.DSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
                TfxShaderStage::Compute => {
                    ctx.CSSetConstantBuffers(slot, Some(&[Some(self.buffer().clone())]))
                }
            }
        }
    }
}

/// A constant buffer that caches the data on the CPU side
pub struct ConstantBufferCached<T: Sized> {
    data: Vec<T>,
    cbuffer: ConstantBuffer<T>,
    updated: AtomicBool,
}

impl<T: Sized + Clone> ConstantBufferCached<T> {
    pub fn create_empty(gctx: Arc<GpuContext>) -> anyhow::Result<Self> {
        Ok(Self {
            cbuffer: ConstantBuffer::create(gctx, None)?,
            data: vec![],
            updated: AtomicBool::new(false),
        })
    }

    pub fn create_init(gctx: Arc<GpuContext>, initial_data: &T) -> anyhow::Result<Self> {
        Ok(Self {
            cbuffer: ConstantBuffer::create(gctx, Some(initial_data))?,
            data: vec![initial_data.clone()],
            updated: AtomicBool::new(false),
        })
    }

    pub fn create_array_init(gctx: Arc<GpuContext>, initial_data: &[T]) -> anyhow::Result<Self> {
        Ok(Self {
            cbuffer: ConstantBuffer::create_array_init(gctx, initial_data)?,
            data: initial_data.to_vec(),
            updated: AtomicBool::new(false),
        })
    }

    /// Writes the buffer data to the GPU
    pub fn write(&self) -> anyhow::Result<()> {
        if self.updated.load(Ordering::Relaxed) {
            self.cbuffer.map(D3D11_MAP_WRITE_DISCARD, |map| unsafe {
                map.pData.copy_from_nonoverlapping(
                    self.data.as_ptr() as _,
                    std::mem::size_of_val(self.data.as_slice()),
                );
            })?;

            self.updated.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    pub fn buffer(&self) -> &ID3D11Buffer {
        self.write().ok();
        self.cbuffer.buffer()
    }

    #[allow(clippy::mut_from_ref)]
    pub fn data_array(&self) -> &mut [T] {
        self.updated.store(true, Ordering::Relaxed);
        unsafe { std::slice::from_raw_parts_mut(self.data.as_ptr() as *mut T, self.data.len()) }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn data(&self) -> &mut T {
        self.updated.store(true, Ordering::Relaxed);
        unsafe { &mut *(self.data.as_ptr() as *mut T) }
    }

    pub fn bind(&self, slot: u32, stage: TfxShaderStage) {
        // Make sure the buffer is written before we bind it
        // Its fine to call this multiple times per draw, as we keep track of whether
        // the buffer has been acquired before we write
        self.write().ok();
        self.cbuffer.bind(slot, stage)
    }
}
