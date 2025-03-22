use crate::{
    bind_group_layout,
    run::rendering::{
        texture::Texture,
        wgpu_ext::{bind_group::HasBindGroup, global_context::device},
    },
};

bind_group_layout!(
    MaterialPlugin,
    wgpu::BindGroupLayoutDescriptor {
        label: struct_label!(),
        entries: &[
            // TEXTURE
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    }
);

pub struct MaterialPlugin;

impl MaterialPlugin {
    pub fn load(name: &str, texture: Texture) -> LoadedMaterial {
        LoadedMaterial {
            name: name.into(),
            bind_group: device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: struct_label!(),
                layout: Self::bind_group_layout(),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
            }),
            texture,
        }
    }
}

pub struct LoadedMaterial {
    #[allow(dead_code)]
    pub(crate) name: String,
    #[allow(dead_code)]
    pub(crate) texture: Texture,
    pub(crate) bind_group: wgpu::BindGroup,
}
