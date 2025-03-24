use {load_gltf::Model, material::LoadedMaterial, mesh::LoadedMesh, std::ops::Range, wgpu::RenderPass};

pub struct Primitive {
    pub mesh: LoadedMesh,
    pub material: LoadedMaterial,
}

pub mod material;
pub mod mesh;

pub mod load_gltf;
pub mod load_obj;

#[extension_traits::extension(pub trait RenderPassDrawModelExt)]
impl<'a> RenderPass<'a> {
    fn draw_model_instanced(&mut self, model: &Model, instances: Range<u32>) {
        model
            .primitives
            .iter()
            .for_each(move |p| self.draw_primitive_instanced(p, instances.clone()))
    }
    fn draw_primitive_instanced(&mut self, model: &Primitive, instances: Range<u32>) {
        // MESH (1)
        self.set_bind_group(1, &model.mesh.bind_group, &[]);
        self.set_index_buffer(model.mesh.index_buffer.as_ref().slice(..), wgpu::IndexFormat::Uint32);
        // MATERIAL (2)
        self.set_bind_group(2, &model.material.bind_group, &[]);

        self.draw_indexed(0..model.mesh.index_buffer.len(), 0, instances);
    }
    fn draw_primitive(&mut self, mesh: &Primitive) {
        self.draw_primitive_instanced(mesh, 0..1);
    }
}
