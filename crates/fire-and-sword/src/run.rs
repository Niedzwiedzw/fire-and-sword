use {
    self::window::WindowHandle,
    anyhow::{Context, Result},
    futures::{FutureExt, Stream, StreamExt},
    rendering::camera::Camera,
    shader_types::{padding::pad, Vec2, Vec3, Vec4, Vertex},
    std::{collections::BTreeMap, future::ready},
    tap::prelude::*,
    tokio::time::Instant,
    tracing::{instrument, warn},
    winit::{
        dpi::PhysicalSize,
        event::{ElementState, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
        window::WindowAttributes,
    },
};

mod config {
    pub const FRAMES_PER_SECOND: usize = 30;
    pub const TICK_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_micros(1_000_000 / (FRAMES_PER_SECOND as u64));
}

pub mod window;

pub mod rendering;

const VERTICES: &[Vertex] = &[
    Vertex {
        position: Vec4::new(-0.0868241, 0.49240386, 0.0, 1.),
        tex_coords: Vec2::new(0.4131759, 0.00759614),
        padding: pad(()),
    }, // A
    Vertex {
        position: Vec4::new(-0.49513406, 0.06958647, 0.0, 1.),
        tex_coords: Vec2::new(0.0048659444, 0.43041354),
        padding: pad(()),
    }, // B
    Vertex {
        position: Vec4::new(-0.21918549, -0.44939706, 0.0, 1.),
        tex_coords: Vec2::new(0.28081453, 0.949397),
        padding: pad(()),
    }, // C
    Vertex {
        position: Vec4::new(0.35966998, -0.3473291, 0.0, 1.),
        tex_coords: Vec2::new(0.85967, 0.84732914),
        padding: pad(()),
    }, // D
    Vertex {
        position: Vec4::new(0.44147372, 0.2347359, 0.0, 1.),
        tex_coords: Vec2::new(0.9414737, 0.2652641),
        padding: pad(()),
    }, // E
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

pub fn game_clock_v2() -> impl Stream<Item = ()> {
    let start = Instant::now();
    let timeouts = (0..).map(move |offset| (offset, start + config::TICK_INTERVAL * offset));
    timeouts
        .pipe(futures::stream::iter)
        .then(|(index, timeout)| tokio::time::sleep_until(timeout).map(move |_| index))
        .map(|_| ())
}

pub enum AppEvent {
    Key(KeyCode, ElementState),
    Tick,
    Redraw,
    Resize(PhysicalSize<u32>),
    Exit,
}

#[derive(Default)]
struct KeyboardState(BTreeMap<KeyCode, ElementState>);

#[instrument]
pub async fn run() -> Result<()> {
    let WindowHandle {
        window,
        events,
        handle: _handle,
    } = WindowHandle::new(WindowAttributes::default().with_title(concat!(clap::crate_name!(), " ", clap::crate_version!()))).await?;

    let mut state = rendering::State::new(&*window)
        .await
        .context("creating renderer state")?;

    let mut camera = Camera::default_for_size(
        window
            .surface_size()
            .pipe(|PhysicalSize { width, height }| (width as _, height as _)),
    );

    let mut keyboard_state = KeyboardState::default();

    let mut events = events
        .filter_map(|event| match event {
            window::WindowingEvent::Winit(window_event) => match window_event {
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(key),
                            state,
                            ..
                        },
                    ..
                } => AppEvent::Key(key, state).pipe(Some).pipe(ready),
                WindowEvent::CloseRequested => AppEvent::Exit.pipe(Some).pipe(ready),
                WindowEvent::RedrawRequested => AppEvent::Redraw.pipe(Some).pipe(ready),
                WindowEvent::SurfaceResized(physical_size) => physical_size.pipe(AppEvent::Resize).pipe(Some).pipe(ready),

                _ => None.pipe(ready),
            },
            _ => None.pipe(ready),
        })
        .pipe(|events| {
            [events.boxed(), game_clock_v2().map(|_| AppEvent::Tick).boxed()]
                .pipe(futures::stream::iter)
                .flatten_unordered(8)
        });
    state.render(&camera).await?;
    while let Some(event) = events.next().await {
        match event {
            AppEvent::Key(key, state) => {
                keyboard_state.0.insert(key, state);
            }
            AppEvent::Redraw => {
                state.window.request_redraw();
                state
                    .render(&camera)
                    .await
                    .context("rendering failed")
                    .or_else(|reason| match reason {
                        reason if format!("{reason:?}").contains("timeout") => {
                            warn!("timeout: {reason:?}");
                            Ok(())
                        }
                        other => Err(other),
                    })?
            }
            AppEvent::Resize(physical_size) => {
                state.resize(physical_size);
            }
            AppEvent::Exit => std::process::exit(0),
            AppEvent::Tick => keyboard_state
                .0
                .iter()
                .filter_map(|(k, v)| v.is_pressed().then_some(k))
                .for_each(|key| match key {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        camera.position_mut(|p| *p += Vec3::X);
                    }
                    _ => {}
                }),
        }
    }

    Ok(())
}
