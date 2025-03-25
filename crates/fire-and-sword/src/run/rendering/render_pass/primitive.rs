use {
    super::{DrawMe, InstanceSyncBuffer, RenderPass, WithInstance},
    crate::run::rendering::model::Primitive,
    tap::prelude::*,
};

pub const MAX_INSTANCES: usize = 1024;

impl DrawMe for WithInstance<&Primitive> {
    fn draw_me<'a, 'b>(&self, RenderPass { buffer, .. }: &mut RenderPass<'a, 'b>) -> anyhow::Result<()> {
        self.pipe(|WithInstance { instance, inner: primitive }| match buffer.queue.get_mut(primitive) {
            Some(exists) => exists.as_mut().push(*instance),
            None => buffer
                .queue
                .insert((*primitive).clone(), InstanceSyncBuffer::new_init(MAX_INSTANCES, vec![*instance]))
                .pipe(|_| ()),
        })
        .pipe(Ok)
    }
}
