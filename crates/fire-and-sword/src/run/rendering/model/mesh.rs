use {
    crate::{
        bind_group_layout,
        run::rendering::wgpu_ext::{
            bind_group::HasBindGroup,
            buffer::{index::IndexBuffer, storage::StorageBuffer},
            global_context::device,
        },
    },
    shader_types::model::ModelVertex,
    tap::prelude::*,
    wgpu::{BindGroup, BindGroupLayout},
};

#[derive(Debug, Clone, Copy)]
pub struct MeshPlugin;

pub struct LoadedMesh {
    #[allow(dead_code)]
    pub(crate) layout: &'static BindGroupLayout,
    #[allow(dead_code)]
    pub(crate) vertex_buffer: StorageBuffer<ModelVertex>,
    pub(crate) index_buffer: IndexBuffer,
    pub(crate) bind_group: BindGroup,
}

bind_group_layout!(
    MeshPlugin,
    wgpu::BindGroupLayoutDescriptor {
        label: struct_label!(),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    }
);

impl MeshPlugin {
    pub fn load_mesh(vertices: &[ModelVertex], indices: &[u32]) -> LoadedMesh {
        Self::bind_group_layout().pipe(|layout| {
            // it is static
            #[allow(deprecated)]
            StorageBuffer::new_init(vertices).pipe(|vertex_buffer| {
                device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        layout,
                        label: struct_label!(),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: vertex_buffer.as_ref().as_entire_binding(),
                        }],
                    })
                    .pipe(|bind_group| LoadedMesh {
                        layout,
                        vertex_buffer,
                        index_buffer: IndexBuffer::new_init(indices),
                        bind_group,
                    })
            })
        })
    }
}
