// use {
//     crate::run::rendering::texture::Texture,
//     anyhow::{Context, Result},
//     std::path::Path,
// };

// fn assets_root() -> &'static Path {
//     Path::new("assets")
// }

// pub fn load_texture(file_name: &Path) -> Result<Texture> {
//     let data = std::fs::read(file_name).with_context(|| format!("loading file at [{file_name:?}]"))?;
//     Texture::from_bytes(&data, &file_name.display().to_string())
// }

// // impl Primitive {
// //     /// this is absolutely horrible but it's for tutorial so whatever
// //     /// again I hate this and I pledge to get rid of this on first
// //     /// occasion
// //     pub fn load_obj_learn_wgpu_way(file_name: &str) -> Result<Self> {
// //         let file_name = assets_root().join(file_name);
// //         let obj_text = read_to_string(&file_name).with_context(|| format!("loading text from [{file_name:?}]"))?;
// //         let obj_cursor = Cursor::new(obj_text);
// //         let mut obj_reader = BufReader::new(obj_cursor);

// //         let (models, obj_materials) = tobj::load_obj_buf(
// //             &mut obj_reader,
// //             &tobj::LoadOptions {
// //                 triangulate: true,
// //                 single_index: true,
// //                 ..Default::default()
// //             },
// //             |p| {
// //                 let mat_text = read_to_string(assets_root().join(p)).unwrap();
// //                 tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
// //             },
// //         )?;

// //         let mut materials = Vec::new();
// //         for m in obj_materials? {
// //             let diffuse_texture = m
// //                 .diffuse_texture
// //                 .context("no diffuse texture name")
// //                 .map(|texture| assets_root().join(texture))
// //                 .and_then(|path| load_texture(&path).context("loading texture"))?;
// //             let material = MaterialPlugin::load(&m.name, diffuse_texture);

// //             materials.push(material)
// //         }
// //         // HACK
// //         materials.push(load_texture(Path::new("assets/cube-diffuse.jpg")).map(|texture| MaterialPlugin::load("cube-diffuse.jpg", texture))?);
// //         let materials = NonEmpty::from_vec(materials).context("empty materials?")?;
// //         let meshes = models
// //             .into_iter()
// //             .map(|m| {
// //                 let vertices = (0..m.mesh.positions.len() / 3)
// //                     .map(|i| {
// //                         let normal = if m.mesh.normals.is_empty() {
// //                             // Assume every 3 vertices form a triangle, and i is the vertex index
// //                             let tri_idx = i - (i % 3); // Round down to the start of the triangle
// //                             let v0 = Vec3::new(
// //                                 m.mesh.positions[(tri_idx * 3) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 1) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 2) % m.mesh.positions.len()],
// //                             );
// //                             let v1 = Vec3::new(
// //                                 m.mesh.positions[(tri_idx * 3 + 3) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 4) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 5) % m.mesh.positions.len()],
// //                             );
// //                             let v2 = Vec3::new(
// //                                 m.mesh.positions[(tri_idx * 3 + 6) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 7) % m.mesh.positions.len()],
// //                                 m.mesh.positions[(tri_idx * 3 + 8) % m.mesh.positions.len()],
// //                             );
// //                             // Compute edges
// //                             let edge1 = v1 - v0;
// //                             let edge2 = v2 - v0;
// //                             // Cross product for normal
// //                             let n = edge1.cross(edge2).normalize_or_zero(); // Normalize, handle degenerate case
// //                             Vec4::new(n.x, n.y, n.z, 0.0)
// //                         } else {
// //                             Vec4::new(m.mesh.normals[i * 3], m.mesh.normals[i * 3 + 1], m.mesh.normals[i * 3 + 2], 0.0)
// //                         };

// //                         if m.mesh.texcoords.is_empty() {
// //                             ModelVertex {
// //                                 position: Vec4::new(m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2], 0.0),
// //                                 tex_coords: Vec2::new(0.0, 0.0), // Default texture coordinates
// //                                 normal,
// //                                 padding: pad(()),
// //                             }
// //                         } else {
// //                             ModelVertex {
// //                                 position: Vec4::new(m.mesh.positions[i * 3], m.mesh.positions[i * 3 + 1], m.mesh.positions[i * 3 + 2], 0.0),
// //                                 tex_coords: Vec2::new(m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]),
// //                                 normal,
// //                                 padding: pad(()),
// //                             }
// //                         }
// //                     })
// //                     .collect::<Vec<_>>();
// //                 MeshPlugin::load_mesh(&vertices, &m.mesh.indices)
// //             })
// //             .pipe(NonEmpty::collect)
// //             .context("empty meshes")?;

// //         Ok(Primitive {
// //             mesh: meshes,
// //             material: materials,
// //         })
// //     }
// // }
