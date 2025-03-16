use {
    super::VERTICES,
    anyhow::{Context, Result},
    itertools::Itertools,
    shader_types::bytemuck,
    std::iter::once,
    tap::prelude::*,
    wgpu::{util::DeviceExt, Color, CommandEncoder},
    winit::{dpi::PhysicalSize, window::Window},
};

pub struct State<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a dyn Window,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a dyn Window) -> Result<Self> {
        let size = window.surface_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window)
            .context("creating surface")?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("requesting adapter")?;

        // creating surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find_or_first(|f| f.is_srgb())
            .copied()
            .context("no surface format available")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps
                .present_modes
                .first()
                .copied()
                .context("no present mode")?,
            alpha_mode: surface_caps
                .alpha_modes
                .first()
                .copied()
                .context("no alpha mode")?,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("main device"),
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("requesting device and queue")?;

        surface.configure(&device, &config);

        let diffuse_texture = include_bytes!("../../../../assets/happy-tree.png")
            .pipe_as_ref(image::load_from_memory)
            .context("bad image")
            .map(|i| i.to_rgba8())
            .context("loading happy tree")
            .map(|diffuse_rgba| {
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
                                    label: Some("happy-tree.png"),
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
            .context("loading happy little tree")?;
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("diffuse_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // building the pipeline
        let shader = unsafe { device.create_shader_module_spirv(&wgpu::include_spirv_raw!("../../../../shaders.spv")) };

        // vertex pulling because i dont want to write the layout for a vertex
        let vertex_buffer = {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Bind Group Layout"),
            entries: &[
                // VERTEX PULLING
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // TEXTURE
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pipeline layout"),
            layout: &bind_group_layout,
            entries: &[
                // VERTEX PULLING
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_buffer.as_entire_binding(),
                },
                // TEXTURE
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("main_vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("main_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],

                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            bind_group,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn with_command_encoder(&self, label: &str, with_command_encoder: impl FnOnce(&mut CommandEncoder) -> Result<()>) -> Result<()> {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
            .pipe(|mut encoder| {
                with_command_encoder(&mut encoder)
                    .with_context(|| format!("running operation [{label}] with encoder"))
                    .map(|_| {
                        self.queue.submit(once(encoder.finish()));
                    })
            })
            .with_context(|| format!("running on encoder: {label}"))
    }
    pub fn update(&mut self) {}
    pub fn render(&mut self) -> Result<()> {
        self.surface
            .get_current_texture()
            .context("getting current texture")
            .and_then(|output| {
                output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
                    .pipe(|texture_view| {
                        self.with_command_encoder("rendering_to_texture", |encoder| {
                            encoder
                                .begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: Some("render pass"),
                                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                        view: &texture_view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            store: wgpu::StoreOp::Store,
                                            load: wgpu::LoadOp::Clear(Color {
                                                r: 0.1,
                                                g: 0.2,
                                                b: 0.3,
                                                a: 1.0,
                                            }),
                                        },
                                    })],
                                    depth_stencil_attachment: None,
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                })
                                .tap_mut(|pass| {
                                    pass.set_pipeline(&self.render_pipeline);
                                    pass.set_bind_group(0, &self.bind_group, &[]);
                                    pass.draw(0..VERTICES.len() as _, 0..1);
                                })
                                .pipe(drop)
                                .pipe(Ok)
                        })
                    })
                    .map(|_| output.present())
            })
    }
}
