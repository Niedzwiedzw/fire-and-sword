use {
    super::AsyncBufferWriteExt,
    crate::run::rendering::wgpu_ext::global_context::device,
    anyhow::{Context, Result},
    shader_types::bytemuck::{self, AnyBitPattern, NoUninit},
    std::{any::type_name, marker::PhantomData},
    tap::prelude::*,
    wgpu::{util::DeviceExt, WasmNotSend},
};

impl<T> StorageBuffer<T> {
    pub async fn write<'a, F>(&'a self, bounds: std::ops::Range<u64>, write: F) -> Result<()>
    where
        F: FnOnce(&mut [T]) + WasmNotSend + 'static,
        T: NoUninit + AnyBitPattern + 'a,
    {
        self.0
            .write_async(device(), bounds, write)
            .await
            .with_context(|| format!("writing to buffer of type [{}]", type_name::<T>()))
    }
}

pub struct StorageBuffer<T>(wgpu::Buffer, PhantomData<T>);

impl<T> AsRef<wgpu::Buffer> for StorageBuffer<T> {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.0
    }
}

impl<T> StorageBuffer<T>
where
    T: NoUninit,
{
    pub fn new_init(init: &[T]) -> Self {
        device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: label!(format!("StorageBuffer<{}>", std::any::type_name::<T>())),
                contents: bytemuck::cast_slice(init),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::MAP_WRITE,
            })
            .pipe(|d| Self(d, Default::default()))
    }
}
