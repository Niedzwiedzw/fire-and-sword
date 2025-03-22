use {
    super::texture::Texture,
    anyhow::Context,
    material::{LoadedMaterial, MaterialPlugin},
    mesh::{LoadedMesh, MeshPlugin},
    nonempty::NonEmpty,
    shader_types::{model::ModelVertex, padding::pad, Vec2, Vec4},
    std::{
        fs::read_to_string,
        io::{BufReader, Cursor},
        ops::Range,
        path::{Path, PathBuf},
    },
    tap::prelude::*,
    wgpu::RenderPass,
};

pub struct Model {
    pub meshes: NonEmpty<LoadedMesh>,
    pub materials: NonEmpty<LoadedMaterial>,
}

pub fn load_texture(file_name: &Path) -> anyhow::Result<Texture> {
    let data = std::fs::read(file_name).with_context(|| format!("loading file at [{file_name:?}]"))?;
    Texture::from_bytes(&data, &file_name.display().to_string())
}

fn assets_root() -> PathBuf {
    PathBuf::from("assets")
}

pub mod material;
pub mod mesh;

pub struct ModelDraw<'a> {
    pub mesh: &'a LoadedMesh,
    pub material: &'a LoadedMaterial,
}

impl Model {
    pub fn draw<'a, F: FnOnce(&'a NonEmpty<LoadedMaterial>) -> &'a LoadedMaterial>(&'a self, with_material: F) -> ModelDraw<'a> {
        ModelDraw {
            mesh: self.meshes.first(),
            material: with_material(&self.materials),
        }
    }
    /// this is absolutely horrible but it's for tutorial so whatever
    /// again I hate this and I pledge to get rid of this on first
    /// occasion
    pub fn load_learn_wgpu_way(file_name: &str) -> anyhow::Result<Self> {
        let file_name = assets_root().join(file_name);
        let obj_text = read_to_string(&file_name).with_context(|| format!("loading text from [{file_name:?}]"))?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mat_text = read_to_string(assets_root().join(p)).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let diffuse_texture = m
                .diffuse_texture
                .context("no diffuse texture name")
                .map(|texture| assets_root().join(texture))
                .and_then(|path| load_texture(&path).context("loading texture"))?;
            let material = MaterialPlugin::load(&m.name, diffuse_texture);

            materials.push(material)
        }
        let materials = NonEmpty::from_vec(materials).context("empty meshes?")?;
        let meshes = models
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| {
                        if m.mesh.normals.is_empty() {
                            ModelVertex {
                                position: Vec4::new(m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2], 0.),
                                tex_coords: Vec2::new(m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]),
                                normal: Vec4::new(0.0, 0.0, 0.0, 0.),
                                padding: pad(()),
                            }
                        } else {
                            ModelVertex {
                                position: Vec4::new(m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2], 0.),
                                tex_coords: Vec2::new(m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]),
                                normal: Vec4::new(m.mesh.normals[i * 3], m.mesh.normals[i * 3 + 1], m.mesh.normals[i * 3 + 2], 0.),
                                padding: pad(()),
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                MeshPlugin::load_mesh(&vertices, &m.mesh.indices)
            })
            .pipe(NonEmpty::collect)
            .context("empty meshes")?;

        Ok(Model { meshes, materials })
    }
}

#[extension_traits::extension(pub trait RenderPassDrawModelExt)]
impl<'a> RenderPass<'a> {
    fn draw_mesh_instanced(&mut self, model: ModelDraw<'_>, instances: Range<u32>) {
        // MESH (1)
        self.set_bind_group(1, &model.mesh.bind_group, &[]);
        self.set_index_buffer(model.mesh.index_buffer.as_ref().slice(..), wgpu::IndexFormat::Uint32);
        // MATERIAL (2)
        self.set_bind_group(2, &model.material.bind_group, &[]);

        self.draw_indexed(0..model.mesh.index_buffer.len(), 0, instances);
    }
    fn draw_mesh(&mut self, mesh: ModelDraw<'_>) {
        self.draw_mesh_instanced(mesh, 0..1);
    }
}
