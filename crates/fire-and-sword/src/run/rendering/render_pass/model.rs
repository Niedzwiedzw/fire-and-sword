use {
    super::{DrawMe, RenderPass, WithInstance},
    crate::run::rendering::model::load_gltf::Model,
    anyhow::Context,
    tap::prelude::*,
};

impl DrawMe for WithInstance<&Model> {
    fn draw_me<'a, 'b>(&self, pass: &mut RenderPass<'a, 'b>) -> anyhow::Result<()> {
        self.pipe(|WithInstance { instance, inner: model }| {
            model
                .primitives
                .iter()
                .map(|primitive| WithInstance {
                    instance: *instance,
                    inner: primitive,
                })
                .try_for_each(|primitive| primitive.draw_me(pass))
        })
        .context("drawing model")
    }
}
