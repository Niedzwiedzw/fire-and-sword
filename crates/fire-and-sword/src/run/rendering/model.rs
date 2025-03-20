use {
    super::texture::Texture,
    shader_types::{bytemuck, model::ModelVertex, padding::pad, Vec2, Vec4},
    std::{
        fs::read_to_string,
        io::{BufReader, Cursor},
        ops::Range,
    },
    wgpu::{util::DeviceExt, RenderPass},
};

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub fn load_texture(file_name: &str, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<Texture> {
    let data = std::fs::read(file_name)?;
    Texture::from_bytes(device, queue, &data, file_name)
}

impl Model {
    /// this is absolutely horrible but it's for tutorial so whatever
    /// again I hate this and I pledge to get rid of this on first
    /// occasion
    pub fn load_learn_wgpu_way(file_name: &str, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> anyhow::Result<Self> {
        let obj_text = read_to_string(file_name)?;
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
                let mat_text = read_to_string(p).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )?;

        let mut materials = Vec::new();
        for m in obj_materials? {
            let diffuse_texture = load_texture(&m.diffuse_texture.unwrap(), device, queue)?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: m.name,
                diffuse_texture,
                bind_group,
            })
        }

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

                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", file_name)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", file_name)),
                    contents: bytemuck::cast_slice(&m.mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                Mesh {
                    name: file_name.to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: m.mesh.indices.len() as u32,
                    material: m.mesh.material_id.unwrap_or(0),
                }
            })
            .collect::<Vec<_>>();

        Ok(Model { meshes, materials })
    }
}

#[extension_traits::extension(pub trait DrawModel)]
impl<'a> RenderPass<'a> {
    fn draw_mesh_instanced(&mut self, mesh: &Mesh, instances: Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
    fn draw_mesh(&mut self, mesh: &Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }
}
