use {
    self::window::WindowHandle,
    crate::game::GameState,
    anyhow::{Context, Result},
    futures::{FutureExt, Stream, StreamExt},
    rendering::{
        camera::{Camera, SENSITIVITY},
        render_pass::WithInstance,
        scene::Scene,
    },
    shader_types::{light_source::LightSource, Color, Vec2, Vec3, Vec4},
    std::{collections::BTreeMap, future::ready},
    tap::prelude::*,
    tokio::time::Instant,
    tracing::{instrument, warn},
    window::WindowingEvent,
    winit::{
        dpi::PhysicalSize,
        event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
        window::WindowAttributes,
    },
};

mod config;

pub mod rendering;
pub mod window;

fn direction_from_look_and_speed(look: Vec3, speed: Vec3) -> Vec3 {
    // Normalize the look vector to ensure it's a unit vector (forward direction)
    let forward = look.normalize();

    // Define a world "up" vector (assuming Y is up in your coordinate system)
    let world_up = Vec3::new(0.0, 1.0, 0.0);

    // Compute the right vector (perpendicular to forward and world_up)
    let right = forward.cross(world_up).normalize();

    // Compute the local up vector (perpendicular to forward and right)
    let up = forward.cross(right).normalize();

    // Transform the speed vector into the player's local space
    // speed.x = right, speed.y = up, speed.z = forward

    right * speed.x + up * speed.y + forward * speed.z
}

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
    MouseMoved(Vec2),
    Tick,
    Redraw,
    Resize(PhysicalSize<u32>),
    Exit,
}

#[derive(Default)]
struct KeyboardState(BTreeMap<KeyCode, ElementState>);

const LIGHT_POSITON: Vec4 = Vec4::new(20., 5., 20., 1.);

#[instrument]
pub async fn run() -> Result<()> {
    let WindowHandle {
        window,
        events,
        handle: _handle,
    } = WindowHandle::new(WindowAttributes::default().with_title(concat!(clap::crate_name!(), " ", clap::crate_version!()))).await?;

    let mut game_state = GameState {
        light_sources: vec![LightSource {
            position: LIGHT_POSITON,
            color: Color([1., 1., 1., 1.]),
        }],
        instances: Default::default(),
        camera: Camera::new(
            Default::default(),
            window
                .surface_size()
                .pipe_ref(|s| (s.width as _, s.height as _)),
        ),
    };

    let mut state = rendering::State::new(&*window, &game_state)
        .await
        .context("creating renderer state")?;

    let scene = gltf::import_slice(include_bytes!("../../../assets/test-map-1.glb"))
        .context("loading gltf map")
        .and_then(|gltf| Scene::load_all(&gltf).context("loading all models from gltf"))
        .map(|map| map.head)
        .context("loading blender scene")?;
    game_state
        .instances
        .push(scene.nodes.head.pipe(|node| WithInstance {
            instance: Default::default(),
            inner: node,
        }));

    let mut keyboard_state = KeyboardState::default();

    let mut events = events
        .filter_map(|event| match event {
            WindowingEvent::Device(DeviceEvent::PointerMotion { delta: (x, y) }) => Vec2::new(x as _, y as _)
                .pipe(AppEvent::MouseMoved)
                .pipe(Some)
                .pipe(ready),
            WindowingEvent::Window(window_event) => match window_event {
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
    state.render_game_state(&game_state).await?;
    while let Some(event) = events.next().await {
        match event {
            AppEvent::MouseMoved(by) => {
                if let Err(reason) = window
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                    .context("grabbing cursor")
                    .map(|_| window.set_cursor_visible(false))
                {
                    tracing::warn!("could not grab cursor:\n{reason:?}");
                }
                game_state
                    .camera
                    .update_rotation(by.x * SENSITIVITY, by.y * SENSITIVITY);
            }
            AppEvent::Key(key, state) => {
                keyboard_state.0.insert(key, state);
            }
            AppEvent::Redraw => state
                .render_game_state(&game_state)
                .await
                .context("rendering failed")
                .or_else(|reason| match reason {
                    reason if format!("{reason:?}").contains("timeout") => {
                        warn!("timeout: {reason:?}");
                        Ok(())
                    }
                    other => Err(other),
                })?,
            AppEvent::Resize(physical_size) => {
                state.resize(physical_size);
                game_state
                    .camera
                    .resize(physical_size.pipe(|PhysicalSize { width, height }| (width as _, height as _)));
            }
            AppEvent::Exit => std::process::exit(0),
            AppEvent::Tick => {
                // inputs
                keyboard_state
                    .0
                    .iter()
                    .filter_map(|(k, v)| v.is_pressed().then_some(k))
                    .filter_map(|key| match key {
                        KeyCode::KeyW | KeyCode::ArrowUp => Some(Vec3::Z),
                        KeyCode::KeyS | KeyCode::ArrowDown => Some(-Vec3::Z),
                        KeyCode::KeyA | KeyCode::ArrowLeft => Some(-Vec3::X),
                        KeyCode::KeyD | KeyCode::ArrowRight => Some(Vec3::X),
                        _ => None,
                    })
                    .fold(Vec3::ZERO, |acc, next| acc + next)
                    .pipe(|speed| direction_from_look_and_speed(game_state.camera.look(), speed))
                    .pipe(|delta| delta * 0.15)
                    .pipe(|delta| {
                        game_state.camera.position_mut(|position| {
                            *position += delta;
                        })
                    });

                // render
                state.window.request_redraw();
            }
        }
    }

    Ok(())
}
