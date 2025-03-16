use {
    self::window::WindowHandle,
    anyhow::{Context, Result},
    shader_types::{padding::pad, Color, Vec2, Vec3, Vec4, Vertex},
    tokio_stream::StreamExt,
    tracing::{instrument, warn},
    winit::{
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
        window::WindowAttributes,
    },
};

pub mod window;

pub mod rendering;

const VERTICES: &[Vertex] = &[
    Vertex {
        position: Vec4::new(-0.0868241, 0.49240386, 0.0, 1.),
        tex_coords: Vec2::new(0.4131759, 0.99240386),
        padding: pad(()),
    }, // A
    Vertex {
        position: Vec4::new(-0.49513406, 0.06958647, 0.0, 1.),
        tex_coords: Vec2::new(0.0048659444, 0.56958647),
        padding: pad(()),
    }, // B
    Vertex {
        position: Vec4::new(-0.21918549, -0.44939706, 0.0, 1.),
        tex_coords: Vec2::new(0.28081453, 0.05060294),
        padding: pad(()),
    }, // C
    Vertex {
        position: Vec4::new(0.35966998, -0.3473291, 0.0, 1.),
        tex_coords: Vec2::new(0.85967, 0.1526709),
        padding: pad(()),
    }, // D
    Vertex {
        position: Vec4::new(0.44147372, 0.2347359, 0.0, 1.),
        tex_coords: Vec2::new(0.9414737, 0.7347359),
        padding: pad(()),
    }, // E
];

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
                WindowEvent::RedrawRequested => {
                    state.window.request_redraw();
                    state.update();
                    state
                        .render()
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
