use {
    super::wgpu_ext::{bind_group::HasBindGroup, buffer::storage::StorageBuffer, global_context::device},
    crate::bind_group_layout,
    shader_types::Instance,
    tap::prelude::*,
};

pub struct InstancePlugin {
    pub buffer: StorageBuffer<Instance>,
    pub bind_group: wgpu::BindGroup,
}

impl InstancePlugin {
    pub fn new(init: &[Instance]) -> Self {
        StorageBuffer::new_init(init).pipe(|buffer| Self {
            bind_group: device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: struct_label!(),
                layout: Self::bind_group_layout(),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_ref().as_entire_binding(),
                }],
            }),
            buffer,
        })
    }
}

bind_group_layout!(
    InstancePlugin,
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
        },],
    }
);
