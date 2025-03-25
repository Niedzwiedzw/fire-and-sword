use {
    super::{DrawMe, RenderPass, WithInstance},
    crate::run::rendering::scene::{Node, NodeData, WithTransform},
    anyhow::Context,
    shader_types::{
        glam::{Affine3A, Vec4Swizzles},
        Instance,
    },
    tap::prelude::*,
    tracing::trace,
};

#[extension_traits::extension(pub trait TransformInstanceExt)]
impl Instance {
    fn transformed(self, transform: &Affine3A) -> Self {
        transform
            .to_scale_rotation_translation()
            .pipe(|(_scale, rot, trans)| {
                self.pipe(|Self { position, rotation }| Self {
                    position: (trans + position.xyz()).extend(1.),
                    rotation: rotation * rot,
                })
            })
    }
}

impl WithInstance<&WithTransform<&Node>> {
    fn draw_me_recursively<'a, 'b>(&self, pass: &mut RenderPass<'a, 'b>) -> anyhow::Result<()> {
        self.as_ref()
            .copied()
            .pipe(|Self { instance, inner: node }| {
                node.pipe(
                    |WithTransform {
                         inner: Node { data, children },
                         transform: parent_transform,
                     }| {
                        (match data {
                            Some(parent) => match parent {
                                NodeData::Camera => Ok(()),
                                NodeData::Model(model) => WithInstance {
                                    instance: instance.transformed(parent_transform),
                                    inner: model,
                                }
                                .draw_me(pass)
                                .tap_ok_dbg(|_| trace!("drawing {model:?} at [{:?}] ({instance:?})", instance.transformed(parent_transform))),
                            },
                            None => Ok(()),
                        })
                        .context("drawing parent")
                        .and_then(|_| {
                            children
                                .iter()
                                .map(|WithTransform { inner, transform }| WithTransform {
                                    inner,
                                    transform: *parent_transform * *transform,
                                })
                                .map(|child| WithInstance { instance, inner: child })
                                .enumerate()
                                .try_for_each(|(idx, child)| {
                                    child
                                        .as_ref()
                                        .draw_me_recursively(pass)
                                        .with_context(|| format!("rendering child [{idx}]"))
                                })
                        })
                    },
                )
            })
    }
}

impl DrawMe for WithInstance<&WithTransform<&Node>> {
    fn draw_me<'a, 'b>(&self, pass: &mut RenderPass<'a, 'b>) -> anyhow::Result<()> {
        self.draw_me_recursively(pass)
    }
}
