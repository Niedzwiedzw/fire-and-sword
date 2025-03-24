pub const FRAMES_PER_SECOND: usize = 30;
pub const TICK_INTERVAL: tokio::time::Duration = tokio::time::Duration::from_micros(1_000_000 / (FRAMES_PER_SECOND as u64));
