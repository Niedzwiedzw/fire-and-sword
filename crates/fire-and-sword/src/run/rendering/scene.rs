use {
    super::model::load_gltf::{GltfImport, Model},
    anyhow::{Context, Result},
    itertools::Itertools,
    nonempty::NonEmpty,
    shader_types::{
        glam::{Affine3A, Mat4},
        Quat,
        Vec3,
        Vec4,
    },
    tap::prelude::*,
};

#[allow(clippy::large_enum_variant)]
pub enum NodeData {
    Camera,
    Model(Model),
}

pub struct Node {
    pub data: Option<NodeData>,
    pub children: Vec<Self>,
    pub transform: Affine3A,
}

impl Node {
    fn load(context: &GltfImport, node: gltf::Node<'_>) -> Result<Self> {
        None.or_else(|| node.camera().map(|_| NodeData::Camera.pipe(Ok)))
            .or_else(|| {
                node.mesh()
                    .map(|m| Model::load(context, m).map(NodeData::Model))
            })
            .transpose()
            .and_then(|data| {
                node.children()
                    .map(|child| Self::load(context, child))
                    .collect::<Result<Vec<_>>>()
                    .context("loading children failed")
                    .map(|children| Node {
                        data,
                        children,
                        transform: node
                            .transform()
                            .decomposed()
                            .pipe(|(translation, rotation, scale)| (Vec3::from(translation), Quat::from_array(rotation), Vec3::from(scale)))
                            .pipe(|(translation, rotation, scale)| Affine3A::from_scale_rotation_translation(scale, rotation, translation)),
                    })
            })
    }
}

pub struct Scene {
    pub nodes: NonEmpty<Node>,
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
