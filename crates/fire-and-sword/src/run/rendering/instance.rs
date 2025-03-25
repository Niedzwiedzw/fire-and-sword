use crate::bind_group_layout;

pub struct InstancePlugin;

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
