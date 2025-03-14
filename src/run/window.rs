//! Simple winit window example.

use {
    anyhow::{Context, Result},
    futures::StreamExt,
    futures_util::FutureExt,
    std::{convert::identity, future::ready, thread::JoinHandle},
    tap::prelude::*,
    tokio::sync::mpsc::Sender,
    tokio_stream::wrappers::ReceiverStream,
    tracing::{debug, instrument},
    winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder},
        platform::wayland::EventLoopBuilderExtWayland,
        window::{Window, WindowAttributes, WindowId},
    },
};

pub struct App {
    events: Sender<WindowingEvent>,
    window_attributes: WindowAttributes,
}

#[derive(derive_more::From, Debug)]
pub enum WindowingEvent {
    Winit(WindowEvent),
    WindowCreated(Result<Box<dyn Window>>),
}

pub struct WindowHandle {
    pub window: Box<dyn Window>,
    pub events: ReceiverStream<WindowingEvent>,
    pub handle: JoinHandle<Result<()>>,
}

impl App {
    fn new(window_attributes: WindowAttributes) -> (ReceiverStream<WindowingEvent>, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel::<WindowingEvent>(2);
        (ReceiverStream::new(rx), Self { events: tx, window_attributes })
    }
}

impl ApplicationHandler for App {
    #[instrument(skip(self), ret, level = "TRACE")]
    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.events
            .blocking_send(event.pipe(WindowingEvent::Winit))
            .expect("application died")
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        event_loop
            .create_window(self.window_attributes.clone())
            .context("creating window")
            .pipe(|window| {
                self.events
                    .blocking_send(WindowingEvent::WindowCreated(window))
                    .expect("application died when resuming")
            })
    }
}

impl WindowHandle {
    pub async fn new(window_attributes: WindowAttributes) -> Result<Self> {
        App::new(window_attributes)
            .pipe(|(mut events, mut app)| {
                std::thread::spawn(move || {
                    EventLoopBuilder::default()
                        .with_any_thread(true)
                        .build()
                        .context("creating event loop")
                        .tap_ok(|event_loop| {
                            event_loop.set_control_flow(ControlFlow::Poll);
                        })
                        .and_then(|event_loop| {
                            event_loop
                                .run_app(&mut app)
                                .context("running app")
                                .tap(|reason| tracing::error!("main event loop finished \n{reason:?}"))
                        })
                })
                .pipe(ready)
                .then(async move |handle| {
                    (&mut events)
                        .inspect(|e| debug!("EVENT: {e:#?}"))
                        .filter_map(|ev| match ev {
                            WindowingEvent::WindowCreated(window) => window.pipe(Some).pipe(ready),
                            _ => ready(None),
                        })
                        .next()
                        .await
                        .context("waiting for window")
                        .and_then(identity)
                        .map(move |window| Self { handle, window, events })
                })
            })
            .await
            .context("creating event handle and event loop")
    }
}
