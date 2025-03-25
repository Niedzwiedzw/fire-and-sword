use {super::identify::WithId, material::LoadedMaterial, mesh::LoadedMesh, std::ops::Range, wgpu::RenderPass};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Primitive {
    pub mesh: WithId<LoadedMesh>,
    pub material: WithId<LoadedMaterial>,
}

pub mod material;
pub mod mesh;

pub mod load_gltf;
pub mod load_obj;

#[extension_traits::extension(pub trait RenderPassDrawModelExt)]
impl<'a> RenderPass<'a> {
    fn draw_primitive_instanced(&mut self, model: &Primitive, instances: Range<u32>) {
        // MESH (1)
        let (mesh, material) = (model.mesh.as_ref(), model.material.as_ref());
        self.set_bind_group(1, &mesh.bind_group, &[]);
        self.set_index_buffer(mesh.index_buffer.as_ref().slice(..), wgpu::IndexFormat::Uint32);
        // MATERIAL (2)
        self.set_bind_group(2, &material.bind_group, &[]);

        self.draw_indexed(0..mesh.index_buffer.len(), 0, instances);
    }

    fn draw_primitive(&mut self, mesh: &Primitive) {
        self.draw_primitive_instanced(mesh, 0..1);
    }
}
