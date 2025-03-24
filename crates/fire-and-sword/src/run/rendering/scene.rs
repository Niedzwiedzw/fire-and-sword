// use anyhow::{Context, Result};

pub struct NodeData;

pub struct Node {
    pub data: NodeData,
    pub children: Vec<Self>,
}

// impl Scene {
//     pub fn from_gltf((document, data, images): &GltfImport) -> Result<Self> {
//         let load_mesh = |mesh| todo!();
//         document.nodes().map(|node| node.mesh())
//     }
// }
