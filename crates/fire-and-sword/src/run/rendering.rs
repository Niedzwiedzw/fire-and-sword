use {
    crate::{
        cloned,
        game::GameState,
        run::rendering::wgpu_ext::global_context::{device, init_device},
    },
    anyhow::{Context, Result},
    camera::CameraPlugin,
    instance::InstancePlugin,
    itertools::Itertools,
    light_source::LightSourcePlugin,
    model::{load_gltf::Model, material::MaterialPlugin, mesh::MeshPlugin, RenderPassDrawModelExt},
    scene::Scene,
    std::{iter::once, ops::Range},
    tap::prelude::*,
    tracing::{instrument, trace},
    wgpu::{Color, CommandEncoder},
    wgpu_ext::{
        bind_group::HasBindGroup,
        global_context::{init_queue, queue},
    },
    winit::{dpi::PhysicalSize, window::Window},
};

pub mod wgpu_ext;

pub mod camera;
pub mod instance;
pub mod light_source;
pub mod model;
pub mod scene;
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

pub struct State<'a> {
    pub surface: wgpu::Surface<'a>,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a dyn Window,
    pub render_pipeline: wgpu::RenderPipeline,
    pub camera_plugin: CameraPlugin,
    pub instance_plugin: InstancePlugin,
    pub light_source_plugin: LightSourcePlugin,
    pub depth_texture: texture::Texture,
    pub scene: Scene,
}

impl<'a> State<'a> {
    pub async fn new(
        window: &'a dyn Window,
        GameState {
            camera,
            instances,
            light_sources,
        }: &GameState,
    ) -> Result<Self> {
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

        let (device_handle, queue_handle) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("main device"),
                    required_features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: wgpu::Limits::default().tap_mut(|limits| limits.max_bind_groups = 5),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("requesting device and queue")?;

        surface.configure(&device_handle, &config);
        init_device(device_handle);
        init_queue(queue_handle);
        let depth_texture = texture::Texture::depth_texture((config.width, config.height), "depth texture");
        // building the pipeline
        let shader = unsafe { device().create_shader_module_spirv(&wgpu::include_spirv_raw!("../../../../shaders.spv")) };

        let camera_plugin = CameraPlugin::new(camera);
        let instance_plugin = InstancePlugin::new(instances);
        let light_source_plugin = LightSourcePlugin::new(light_sources);

        let render_pipeline_layout = device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                // 0
                CameraPlugin::bind_group_layout(),
                // 1
                MeshPlugin::bind_group_layout(),
                // 2
                MaterialPlugin::bind_group_layout(),
                // 3
                InstancePlugin::bind_group_layout(),
                // 4
                LightSourcePlugin::bind_group_layout(),
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                // cull_mode: None,
                cull_mode: Some(wgpu::Face::Back),
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

        let scene = gltf::import_slice(include_bytes!("../../../../assets/test-map-1.glb"))
            .context("loading gltf map")
            .and_then(|gltf| Scene::load_all(&gltf).context("loading all models from gltf"))
            .map(|map| map.head)
            .context("loading blender scene")?;

        Ok(Self {
            surface,
            config,
            size,
            window,
            render_pipeline,
            camera_plugin,
            instance_plugin,
            light_source_plugin,
            depth_texture,
            scene,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture = texture::Texture::depth_texture(self.config.pipe_ref(|c| (c.width, c.height)), "depth texture");
            self.surface.configure(device(), &self.config);
        }
    }

    pub fn with_command_encoder(label: &str, with_command_encoder: impl FnOnce(&mut CommandEncoder) -> Result<()>) -> Result<()> {
        device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
            .pipe(|mut encoder| {
                with_command_encoder(&mut encoder)
                    .with_context(|| format!("running operation [{label}] with encoder"))
                    .map(|_| {
                        queue().submit(once(encoder.finish()));
                    })
            })
            .with_context(|| format!("running on encoder: {label}"))
    }

    #[instrument(skip_all)]
    pub async fn render(
        &mut self,
        GameState {
            camera,
            instances,
            light_sources,
        }: &GameState,
    ) -> Result<()> {
        trace!("flushing camera");
        // FLUSH CAMERA
        let camera = camera.get_view_projection();
        self.camera_plugin
            .buffer
            .write(0..1u64, move |buf| {
                buf[0] = camera;
            })
            .await
            .context("writing camera")?;
        self.instance_plugin
            .buffer
            .write(0..(instances.len() as _), {
                cloned![instances];
                move |current| {
                    current.copy_from_slice(&instances);
                }
            })
            .await
            .context("updating instances")?;

        self.light_source_plugin
            .buffer
            .write(0..(light_sources.len() as _), {
                cloned![light_sources];
                move |current| {
                    current.copy_from_slice(&light_sources);
                }
            })
            .await
            .context("updating instances")?;
        trace!("writing to surface");
        self.surface
            .get_current_texture()
            .context("getting current texture")
            .and_then(|output| {
                output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
                    .pipe(|texture_view| {
                        Self::with_command_encoder("rendering_to_texture", |encoder| {
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
                                                g: 0.1,
                                                b: 0.1,
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
                                    pass.set_bind_group(0, &self.camera_plugin.bind_group, &[]);
                                    pass.set_bind_group(3, &self.instance_plugin.bind_group, &[]);
                                    pass.set_bind_group(4, &self.light_source_plugin.bind_group, &[]);
                                    pass.draw_scene_instanced(&self.scene, 0..instances.len() as u32);
                                })
                                .pipe(drop)
                                .pipe(Ok)
                        })
                    })
                    .map(|_| output.present())
            })
    }
}
