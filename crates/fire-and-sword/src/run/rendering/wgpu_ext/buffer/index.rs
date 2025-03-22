use {
    super::AsyncBufferWriteExt,
    crate::run::rendering::wgpu_ext::global_context::device,
    anyhow::{Context, Result},
    shader_types::bytemuck::{self},
    tap::prelude::*,
    wgpu::{util::DeviceExt, WasmNotSend},
};

impl IndexBuffer {
    pub async fn write<F>(&self, bounds: std::ops::Range<u64>, write: F) -> Result<()>
    where
        F: FnOnce(&mut [u32]) + WasmNotSend + 'static,
    {
        self.buffer
            .write_async(device(), bounds, write)
            .await
            .context("writing to index buffer")
    }
}

pub struct IndexBuffer {
    buffer: wgpu::Buffer,
    len: u32,
}

impl AsRef<wgpu::Buffer> for IndexBuffer {
    fn as_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

impl IndexBuffer {
    /// in learn wgpu: "num_elements"
    #[doc(alias = "num_elements")]
    pub fn len(&self) -> u32 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn new_init(init: &[u32]) -> Self {
        device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: struct_label!(),
                contents: bytemuck::cast_slice(init),
                usage: wgpu::BufferUsages::INDEX,
            })
            .pipe(|buffer| Self { len: init.len() as _, buffer })
    }
}
