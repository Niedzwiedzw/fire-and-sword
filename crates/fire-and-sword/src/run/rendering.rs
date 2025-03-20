use {
    super::VERTICES,
    crate::run::INDICES,
    anyhow::{Context, Result},
    camera::Camera,
    futures::channel::oneshot,
    itertools::Itertools,
    shader_types::{
        bytemuck::{self, AnyBitPattern, NoUninit},
        glam::Quat,
        Instance,
        Vec3,
    },
    std::{
        iter::once,
        ops::{Mul, Range},
    },
    tap::prelude::*,
    tracing::{error, info, instrument, trace},
    wgpu::{util::DeviceExt, Color, CommandEncoder, MapMode, WasmNotSend},
    winit::{dpi::PhysicalSize, window::Window},
};

pub mod camera;
pub mod model;
pub mod texture;

#[extension_traits::extension(pub trait RangeMapExt)]
impl<T> Range<T> {
    fn map_range<U>(self, mut map: impl FnMut(T) -> U) -> Range<U> {
        Range {
            start: map(self.start),
            end: map(self.end),
        }
    }
}

#[extension_traits::extension(pub(crate) trait AsyncBufferWriteExt)]
impl wgpu::Buffer {
    async fn write_async<'a, T, F>(&'a self, device: &wgpu::Device, bounds: std::ops::Range<u64>, write: F) -> Result<()>
    where
        T: NoUninit + AnyBitPattern + 'a,
        F: FnOnce(&mut [T]) + WasmNotSend + 'static,
    {
        let bounds = bounds.map_range(|address| address * (core::mem::size_of::<T>() as u64));
        let (tx, rx) = oneshot::channel();
        self.slice(bounds.clone()).pipe(|slice| {
            self.clone().pipe(|slice_access| {
                slice.map_async(MapMode::Write, move |w| {
                    w.context("bad write")
                        .and_then(|_| {
                            let mut slice = slice_access.slice(bounds).get_mapped_range_mut();
                            let data = bytemuck::try_cast_slice_mut::<_, _>(&mut slice)
                                .map_err(|bytes| anyhow::anyhow!("{bytes:?}"))
                                .context("casting failed")?;
                            write(data);
                            drop(slice);
                            slice_access.unmap();
                            Ok(())
                        })
                        .and_then(|_| {
                            tx.send(())
                                .map_err(|_| anyhow::anyhow!("send failed"))
                                .context("sending")
                        })
                        .pipe(|r| {
                            if let Err(reason) = r {
                                error!("write failed:\n{reason:?}");
                            }
                        })
                })
            });
        });
        device.poll(wgpu::Maintain::Wait);
        trace!("waiting for async operation to finish");
        rx.await.context("task cancelled")
    }
}

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
    pub index_buffer: wgpu::Buffer,
    pub camera_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
    pub depth_texture: texture::Texture,
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
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("requesting device and queue")?;

        surface.configure(&device, &config);
        let depth_texture = texture::Texture::depth_texture(&device, (config.width, config.height), "depth texture");
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, include_bytes!("../../../../assets/happy-tree.png"), "happy-tree.png")
            .context("loading happy little tree")?;
        info!("loaded happy tree");

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("diffuse_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
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
        const NUM_INSTANCES: u32 = 10;
        const INSTANCE_DISPLACEMENT: Vec3 = Vec3::new(NUM_INSTANCES as f32 * 0.5, 0.0, NUM_INSTANCES as f32 * 0.5);

        let instances = (0..NUM_INSTANCES)
            .flat_map(|z| (0..NUM_INSTANCES).map(move |x| (x, z)))
            .map(|(x, z)| Vec3::new(x as _, 0., z as _))
            .map(|position| position - INSTANCE_DISPLACEMENT)
            .enumerate()
            .map(|(idx, position)| {
                Quat::from_rotation_z((idx as f32).mul(15.).to_radians()).pipe(|rotation| Instance {
                    position: position.extend(1.),
                    rotation,
                })
            })
            .inspect(|instance| info!("{:?}", instance.position))
            .collect_vec();
        let instance_buffer = {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex buffer"),
                contents: instances.pipe_deref(bytemuck::cast_slice),
                usage: wgpu::BufferUsages::STORAGE,
            })
        };

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let camera_buffer = Camera::new([10., 0., 0.].into())
            .pipe(|camera| {
                camera.get_view_projection(
                    window
                        .surface_size()
                        .pipe(|PhysicalSize { width, height }| (width as _, height as _)),
                )
            })
            .pipe(|camera| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera buffer"),
                    contents: bytemuck::cast_slice(&[camera]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::MAP_WRITE,
                })
            });
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
                // CAMERA
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // INSTANCES
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
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
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
                // CAMERA
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: camera_buffer.as_entire_binding(),
                },
                // INSTANCE
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: instance_buffer.as_entire_binding(),
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
                // WARN: huge perf hit
                cull_mode: None,
                // cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
            index_buffer,
            bind_group,
            camera_buffer,
            instances,
            instance_buffer,
            depth_texture,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture = texture::Texture::depth_texture(&self.device, self.config.pipe_ref(|c| (c.width, c.height)), "depth texture");
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
    #[instrument(skip_all)]
    pub async fn render(&mut self, camera: &Camera) -> Result<()> {
        trace!("flushing camera");
        // FLUSH CAMERA
        let camera = camera.get_view_projection(
            self.window
                .surface_size()
                .pipe(|PhysicalSize { width, height }| (width as _, height as _)),
        );
        self.camera_buffer
            .write_async(&self.device, 0..1u64, move |buf| {
                buf[0] = camera;
            })
            .await
            .context("writing camera")?;
        trace!("writing to surface");
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
                                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                        view: &self.depth_texture.view,
                                        depth_ops: Some(wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(1.),
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    }),
                                    timestamp_writes: None,
                                    occlusion_query_set: None,
                                })
                                .tap_mut(|pass| {
                                    pass.set_pipeline(&self.render_pipeline);
                                    pass.set_bind_group(0, &self.bind_group, &[]);
                                    pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                                    pass.draw_indexed(0..INDICES.len() as _, 0, 0..(self.instances.len() as _));
                                })
                                .pipe(drop)
                                .pipe(Ok)
                        })
                    })
                    .map(|_| output.present())
            })
    }
}
