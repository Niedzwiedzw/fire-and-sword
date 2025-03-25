use {
    super::model::load_gltf::{GltfImport, Model},
    anyhow::{Context, Result},
    itertools::Itertools,
    nonempty::NonEmpty,
    shader_types::{glam::Affine3A, Quat, Vec3},
    tap::prelude::*,
};

#[allow(clippy::large_enum_variant)]
pub enum NodeData {
    Camera,
    Model(Model),
}

pub struct WithTransform<T> {
    pub inner: T,
    pub transform: Affine3A,
}

impl<T> WithTransform<&T> {
    pub fn copied(&self) -> WithTransform<T>
    where
        T: Copy,
    {
        WithTransform {
            inner: *self.inner,
            transform: self.transform,
        }
    }
}

impl<T> WithTransform<T> {
    pub fn as_ref(&self) -> WithTransform<&T> {
        WithTransform {
            inner: &self.inner,
            transform: self.transform,
        }
    }
}

pub struct Node {
    pub data: Option<NodeData>,
    pub children: Vec<WithTransform<Self>>,
}

impl Node {
    fn load(context: &GltfImport, node_data: gltf::Node<'_>) -> Result<WithTransform<Self>> {
        None.or_else(|| node_data.camera().map(|_| NodeData::Camera.pipe(Ok)))
            .or_else(|| {
                node_data
                    .mesh()
                    .map(|m| Model::load(context, m).map(NodeData::Model))
            })
            .transpose()
            .and_then(|data| {
                node_data
                    .children()
                    .map(|child| Self::load(context, child))
                    .collect::<Result<Vec<_>>>()
                    .context("loading children failed")
                    .map(|children| Node { data, children })
                    .map(|node| WithTransform {
                        transform: node_data
                            .transform()
                            .decomposed()
                            .pipe(|(translation, rotation, scale)| (Vec3::from(translation), Quat::from_array(rotation), Vec3::from(scale)))
                            .pipe(|(translation, rotation, scale)| Affine3A::from_scale_rotation_translation(scale, rotation, translation)),
                        inner: node,
                    })
            })
    }
}

pub struct Scene {
    pub nodes: NonEmpty<WithTransform<Node>>,
}

impl Scene {
    pub fn load_all(context: &GltfImport) -> Result<NonEmpty<Self>> {
        context
            .0
            .scenes()
            .map(|scene| {
                scene
                    .nodes()
                    .map(|node| Node::load(context, node))
                    .collect::<Result<Vec<_>>>()
                    .context("loading nodes for a scene")
                    .and_then(|nodes| NonEmpty::from_vec(nodes).context("scenes without nodes are not supported"))
            })
            .map_ok(|nodes| Self { nodes })
            .collect::<Result<Vec<_>>>()
            .context("loading all scenes")
            .and_then(|scenes| NonEmpty::from_vec(scenes).context("file should have at least one scene"))
    }
}
