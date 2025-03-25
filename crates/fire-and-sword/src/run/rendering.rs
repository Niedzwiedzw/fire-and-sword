use {
    crate::{
        game::GameState,
        run::rendering::wgpu_ext::global_context::{device, init_device},
    },
    anyhow::{Context, Result},
    camera::CameraPlugin,
    futures::TryFutureExt,
    instance::InstancePlugin,
    itertools::Itertools,
    light_source::LightSourcePlugin,
    model::{material::MaterialPlugin, mesh::MeshPlugin},
    render_pass::WithInstance,
    std::{future::ready, iter::once, ops::Range},
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

pub mod identify;

pub mod camera;
pub mod instance;
pub mod light_source;
pub mod model;
pub mod scene;
pub mod texture;

pub mod render_pass;

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
    pub light_source_plugin: LightSourcePlugin,
    pub depth_texture: texture::Texture,
    pub pass_buffer: self::render_pass::PassBuffer,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a dyn Window, GameState { camera, scene, light_sources }: &GameState) -> Result<Self> {
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
        // let instance_plugin = InstancePlugin::new(instances);
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

        Ok(Self {
            surface,
            config,
            size,
            window,
            render_pipeline,
            camera_plugin,
            light_source_plugin,
            depth_texture,
            pass_buffer: Default::default(),
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

    pub async fn with_command_encoder_async<'task, F>(label: &str, with_command_encoder: F) -> Result<()>
    where
        F: AsyncFnOnce(&mut CommandEncoder) -> Result<()> + 'task,
    {
        device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
            .pipe(|mut encoder| async move {
                with_command_encoder(&mut encoder)
                    .await
                    .with_context(|| format!("running operation [{label}] with encoder"))
                    .map(|_| {
                        queue().submit(once(encoder.finish()));
                    })
            })
            .await
            .with_context(|| format!("running on encoder: {label}"))
    }
    pub async fn render_game_state(&mut self, GameState { camera, scene, light_sources }: &GameState) -> Result<()> {
        self.render_pass(|pass| {
            pass.set_camera(*camera);
            scene
                .as_ref()
                .iter()
                .map(|scene| scene.nodes.iter())
                .flatten()
                .map(|node| WithInstance {
                    instance: Default::default(),
                    inner: node.as_ref(),
                })
                .try_for_each(|node| pass.draw(&node.as_ref()))
        })
        .await
        .context("rendering full game state")
    }
    #[instrument(skip_all)]
    pub async fn render_pass<F>(
        &mut self,
        with_render_pass: F,
        // GameState {
        //     camera,
        //     instances,
        //     light_sources,
        // }: &GameState,
    ) -> Result<()>
    where
        F: FnOnce(&mut self::render_pass::RenderPass<'_, '_>) -> Result<()>,
    {
        trace!("flushing camera");
        // FLUSH CAMERA
        // let camera = camera.get_view_projection();
        // self.camera_plugin
        //     .buffer
        //     .write(0..1u64, move |buf| {
        //         buf[0] = camera;
        //     })
        //     .await
        //     .context("writing camera")?;
        // self.instance_plugin
        //     .buffer
        //     .write(0..(instances.len() as _), {
        //         cloned![instances];
        //         move |current| {
        //             current.copy_from_slice(&instances);
        //         }
        //     })
        //     .await
        //     .context("updating instances")?;

        // self.light_source_plugin
        //     .buffer
        //     .write(0..(light_sources.len() as _), {
        //         cloned![light_sources];
        //         move |current| {
        //             current.copy_from_slice(&light_sources);
        //         }
        //     })
        //     .await
        //     .context("updating instances")?;
        trace!("writing to surface");
        self.surface
            .get_current_texture()
            .context("getting current texture")
            .pipe(ready)
            .and_then(async |output| {
                output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
                    .pipe(|texture_view| async move {
                        Self::with_command_encoder_async("rendering_to_texture", async |encoder| {
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
                                    // pass.set_bind_group(3, &self.instance_plugin.bind_group, &[]);
                                    pass.set_bind_group(4, &self.light_source_plugin.bind_group, &[]);
                                    // pass.draw_scene_instanced(&self.scene, 0..instances.len() as u32);
                                })
                                .pipe_ref_mut(|pass| self::render_pass::RenderPass {
                                    camera_plugin: &mut self.camera_plugin,
                                    camera: None,
                                    buffer: &mut self.pass_buffer,
                                    pass,
                                })
                                .pipe(|mut pass| with_render_pass(&mut pass).map(|_| pass))
                                .pipe(ready)
                                .and_then(|pass| pass.finish())
                                .await
                                .context("finishing up render pass")
                        })
                        .await
                    })
                    .await
                    .map(|_| output.present())
            })
            .await
    }
}
