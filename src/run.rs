use {
    self::window::WindowHandle,
    anyhow::Result,
    tokio_stream::StreamExt,
    tracing::{info, instrument},
    winit::{event_loop::EventLoop, window::WindowAttributes},
};

pub mod window;

#[instrument]
pub async fn run() -> Result<()> {
    let WindowHandle { window, mut events, handle } =
        WindowHandle::new(WindowAttributes::default().with_title(concat!(clap::crate_name!(), " ", clap::crate_version!()))).await?;

    while let Some(event) = events.next().await {
        info!("{event:#?}");
    }
    Ok(())
}
