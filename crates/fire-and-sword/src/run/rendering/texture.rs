use {
    anyhow::{Context, Result},
    tap::prelude::*,
};

pub struct Texture {
    #[allow(unused)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8], label: &str) -> Result<Self> {
        image::load_from_memory(bytes)
            .context("Bad image")
            .map(|image| Self::from_image(device, queue, &image, Some(label)))
    }

    pub fn from_image(device: &wgpu::Device, queue: &wgpu::Queue, img: &image::DynamicImage, label: Option<&str>) -> Self {
        img.pipe(|i| i.to_rgba8())
            .pipe(|diffuse_rgba| {
                diffuse_rgba
                    .dimensions()
                    .pipe(|(width, height)| wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    })
                    .pipe(
                        |size @ wgpu::Extent3d {
                             width,
                             height,
                             depth_or_array_layers: _,
                         }| {
                            device
                                .create_texture(&wgpu::TextureDescriptor {
                                    size,
                                    mip_level_count: 1,
                                    sample_count: 1,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                                    label,
                                    view_formats: &[],
                                })
                                .pipe(|diffuse_texture| {
                                    queue
                                        .write_texture(
                                            wgpu::TexelCopyTextureInfo {
                                                texture: &diffuse_texture,
                                                mip_level: 0,
                                                origin: wgpu::Origin3d::ZERO,
                                                aspect: wgpu::TextureAspect::All,
                                            },
                                            &diffuse_rgba,
                                            wgpu::TexelCopyBufferLayout {
                                                offset: 0,
                                                bytes_per_row: Some(4 * width),
                                                rows_per_image: Some(height),
                                            },
                                            size,
                                        )
                                        .pipe(|_| diffuse_texture)
                                })
                        },
                    )
            })
            .pipe(|texture| Self {
                view: texture.create_view(&wgpu::TextureViewDescriptor { label, ..Default::default() }),
                texture,
                sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                    label,
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                }),
            })
    }
}
