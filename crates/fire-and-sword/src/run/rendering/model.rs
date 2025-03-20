use {
    super::texture::Texture,
    anyhow::Context,
    shader_types::{bytemuck, model::ModelVertex, padding::pad, Vec2, Vec4},
    std::{
        fs::read_to_string,
        io::{BufReader, Cursor},
        ops::Range,
        path::{Path, PathBuf},
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
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub num_elements: u32,
    pub material: usize,
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub fn load_texture(file_name: &Path, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<Texture> {
    let data = std::fs::read(file_name).with_context(|| format!("loading file at [{file_name:?}]"))?;
    Texture::from_bytes(device, queue, &data, &file_name.display().to_string())
}

fn assets_root() -> PathBuf {
    PathBuf::from("assets")
}

impl Model {
    /// this is absolutely horrible but it's for tutorial so whatever
    /// again I hate this and I pledge to get rid of this on first
    /// occasion
    pub fn load_learn_wgpu_way(file_name: &str, device: &wgpu::Device, queue: &wgpu::Queue) -> anyhow::Result<Self> {
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
                .and_then(|path| load_texture(&path, device, queue).context("loading texture"))?;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bind group layout"),
                    entries: &[
                        // TEXTURE
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                }),
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
                let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer Bind Group Layout", file_name)),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &bind_group_layout,
                    label: Some(&format!("{:?} Vertex Buffer Bind Group", file_name)),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: vertex_buffer.as_entire_binding(),
                    }],
                });
                Mesh {
                    name: file_name.display().to_string(),
                    vertex_buffer,
                    index_buffer,
                    bind_group_layout,
                    bind_group,
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
        self.set_bind_group(1, &mesh.bind_group, &[]);
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
    fn draw_mesh(&mut self, mesh: &Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }
}
