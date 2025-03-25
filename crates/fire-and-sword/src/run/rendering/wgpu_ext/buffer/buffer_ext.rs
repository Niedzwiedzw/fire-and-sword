use {
    crate::run::rendering::RangeMapExt,
    anyhow::{Context, Result},
    futures::channel::oneshot,
    shader_types::bytemuck::{self, AnyBitPattern, NoUninit},
    tap::prelude::*,
    tracing::{error, trace, warn},
    wgpu::{MapMode, WasmNotSend},
};

#[extension_traits::extension(pub(super) trait AsyncBufferWriteExt)]
impl wgpu::Buffer {
    async fn write_async<'a, T, F>(&'a self, device: &wgpu::Device, bounds: std::ops::Range<u64>, write: F) -> Result<()>
    where
        T: NoUninit + AnyBitPattern + 'a,
        F: FnOnce(&mut [T]) + WasmNotSend + 'static,
    {
        let bounds = bounds.map_range(|address| address * (core::mem::size_of::<T>() as u64));
        if bounds.is_empty() {
            warn!("writing to an empty slice [{bounds:?}] is a noop");
            return Ok(());
        }
        let (tx, rx) = oneshot::channel();

        self.slice(bounds.clone()).pipe(|slice| {
            self.clone().pipe(|slice_access| {
                slice.map_async(MapMode::Write, move |w| {
                    w.context("bad write")
                        .and_then(|_| {
                            let mut slice = slice_access.slice(bounds).get_mapped_range_mut();
                            let data = bytemuck::try_cast_slice_mut::<u8, T>(&mut slice)
                                .map_err(|bytes| anyhow::anyhow!("{bytes:?}"))
                                .context("casting failed")?;
                            write(data);
                            drop(slice);
                            slice_access.unmap();
                            Ok(())
                        })
                        .and_then(|_| {
                            tx.send(())
                                .map_err(|_| anyhow::anyhow!("send failed"))
                                .context("sending")
                        })
                        .pipe(|r| {
                            if let Err(reason) = r {
                                error!("write failed:\n{reason:?}");
                            }
                        })
                })
            });
        });
        device.poll(wgpu::Maintain::Wait);
        trace!("waiting for async operation to finish");
        rx.await.context("task cancelled")
    }
}
