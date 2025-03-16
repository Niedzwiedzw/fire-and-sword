use {
    self::window::WindowHandle,
    anyhow::{Context, Result},
    shader_types::{Color, Vec3, Vertex},
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
        position: Vec3::new(0., 1., 0.),
        color: Color::new([1., 1., 1., 1.]),
    },
    Vertex {
        position: Vec3::new(-1., -1., 0.),
        color: Color::new([0., 0., 0., 1.]),
    },
    Vertex {
        position: Vec3::new(-1., -1., 0.),
        color: Color::new([1., 1., 1., 1.]),
    },
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
