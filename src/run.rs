use {
    self::window::WindowHandle,
    anyhow::{Context, Result},
    tokio_stream::StreamExt,
    tracing::{instrument, warn},
    wgpu::Color,
    winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
        window::WindowAttributes,
    },
};

pub mod window;

pub mod rendering {
    use {
        anyhow::{Context, Result},
        itertools::Itertools,
        std::iter::once,
        tap::prelude::*,
        wgpu::{Color, CommandEncoder},
        winit::{dpi::PhysicalSize, window::Window},
    };

    pub struct State<'a> {
        pub surface: wgpu::Surface<'a>,
        pub device: wgpu::Device,
        pub queue: wgpu::Queue,
        pub config: wgpu::SurfaceConfiguration,
        pub size: winit::dpi::PhysicalSize<u32>,
        pub window: &'a dyn Window,
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
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .context("requesting device and queue")?;
            Ok(Self {
                surface,
                device,
                queue,
                config,
                size,
                window,
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
        pub fn render(&mut self, color: &Color) -> Result<()> {
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
                                                load: wgpu::LoadOp::Clear(*color),
                                            },
                                        })],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    })
                                    .pipe(drop)
                                    .pipe(Ok)
                            })
                        })
                        .map(|_| output.present())
                })
        }
    }
}

#[instrument]
pub async fn run() -> Result<()> {
    let WindowHandle {
        window,
        mut events,
        handle: _handle,
    } = WindowHandle::new(WindowAttributes::default().with_title(concat!(clap::crate_name!(), " ", clap::crate_version!()))).await?;

    let mut state = rendering::State::new(&*window)
        .await
        .context("creating renderer state")?;

    let mut clear_color = Color::BLACK;

    while let Some(event) = events.next().await {
        match event {
            window::WindowingEvent::Winit(window_event) => match window_event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => std::process::exit(0),
                WindowEvent::PointerMoved { position, .. } => {
                    let size = window.surface_size();
                    clear_color.r = position.x / size.width as f64;
                    clear_color.g = position.y / size.height as f64;
                }
                WindowEvent::RedrawRequested => {
                    state.window.request_redraw();
                    state.update();
                    state
                        .render(&clear_color)
                        .context("rendering failed")
                        .or_else(|reason| match reason {
                            reason if format!("{reason:?}").contains("timeout") => {
                                warn!("timeout: {reason:?}");
                                Ok(())
                            }
                            other => Err(other),
                        })?
                }
                WindowEvent::SurfaceResized(physical_size) => {
                    state.resize(physical_size);
                }

                _ => {}
            },
            window::WindowingEvent::WindowCreated(_) => {}
        }
    }

    Ok(())
}
