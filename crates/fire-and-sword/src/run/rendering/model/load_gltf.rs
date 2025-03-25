use {
    super::{material::MaterialPlugin, mesh::MeshPlugin, Primitive},
    crate::run::rendering::{identify::WithId, texture::Texture},
    anyhow::{Context, Result},
    gltf::image::Source,
    image::{GenericImage, GenericImageView, Rgba},
    itertools::Itertools,
    nonempty::NonEmpty,
    shader_types::{model::ModelVertex, padding::pad, Vec2, Vec3},
    tap::prelude::*,
};

pub type GltfImport = (gltf::Document, Vec<gltf::buffer::Data>, Vec<gltf::image::Data>);

#[derive(Debug)]
pub struct Model {
    pub primitives: NonEmpty<Primitive>,
}

impl Model {
    pub fn load_all(context: &GltfImport) -> Result<Option<NonEmpty<Self>>> {
        context
            .0
            .meshes()
            .enumerate()
            .map(|(idx, m)| Self::load(context, m).with_context(|| format!("loading mesh #{idx}")))
            .collect::<Result<_>>()
            .map(NonEmpty::from_vec)
    }
    pub fn load(context: &GltfImport, mesh: gltf::Mesh<'_>) -> Result<Self> {
        mesh.primitives()
            .map(|primitive| Primitive::load_primitive(context, primitive))
            .collect::<Result<Vec<_>>>()
            .context("not all primitives could be loaded")
            .and_then(|v| NonEmpty::from_vec(v).context("model cannot be empty"))
            .map(|primitives| Self { primitives })
    }
}

impl Primitive {
    pub fn load_primitive((document, buffer_data, image_data): &GltfImport, primitive: gltf::Primitive<'_>) -> Result<Self> {
        primitive
            .reader(|buffer| {
                buffer_data
                    .get(buffer.index())
                    .map(|v| v.as_ref())
                    .tap_none(|| tracing::warn!("no buffer found in data at index [{}]", buffer.index()))
            })
            .pipe(|reader| {
                reader
                    .read_positions()
                    .context("no positions found")
                    .and_then(|positions| {
                        reader
                            .read_normals()
                            .context("no normals")
                            .and_then(|normals| {
                                reader
                                    .read_tex_coords(0)
                                    .with_context(|| "no text coords at set [0]".to_string())
                                    .map(|tex_coords| itertools::multizip((positions, normals, tex_coords.into_f32())))
                            })
                    })
                    .and_then(|vertices| {
                        reader.read_indices().context("indices").map(|indices| {
                            vertices
                                .map(|(position, normal, tex_coords)| ModelVertex {
                                    position: Vec3::from(position).extend(1.),
                                    normal: Vec3::from(normal).extend(1.),
                                    tex_coords: Vec2::from(tex_coords),
                                    padding: pad(()),
                                })
                                .collect_vec()
                                .pipe_deref(|vertices| MeshPlugin::load_mesh(vertices, indices.into_u32().collect_vec().as_slice()))
                        })
                    })
                    .and_then(|mesh| {
                        primitive
                            .material()
                            .pipe(|material| {
                                material.pbr_metallic_roughness().pipe(|pbr| {
                                    pbr.base_color_texture()
                                        .map(|info| {
                                            document
                                                .textures()
                                                .nth(info.texture().index())
                                                .with_context(|| format!("no texture at index [{}]", info.texture().index()))
                                                .and_then(|texture| {
                                                    document
                                                        .images()
                                                        .nth(texture.source().index())
                                                        .with_context(|| format!("no image at index [{}]", texture.index()))
                                                        .and_then(|image| match image.source() {
                                                            Source::View { view, mime_type: _ } => image_data
                                                                .get(view.index())
                                                                .with_context(|| format!("no image at index {}", view.index()))
                                                                .and_then(|image| Texture::from_bytes(&image.pixels, texture.name().unwrap_or("UNKNOWN"))),
                                                            Source::Uri { uri, mime_type } => {
                                                                todo!("Source::Uri {{ uri: {uri}, mime_type: {mime_type:?} }}")
                                                            }
                                                        })
                                                        .map(|data| MaterialPlugin::load(texture.name().unwrap_or("UNKNOWN"), data))
                                                })
                                        })
                                        .unwrap_or_else(|| {
                                            image::DynamicImage::new(32, 32, image::ColorType::Rgba8)
                                                .tap_mut(|image| {
                                                    image
                                                        .pixels()
                                                        .map(|(x, y, _)| (x, y))
                                                        .collect_vec()
                                                        .pipe(|pixels| {
                                                            pixels.into_iter().for_each(|(x, y)| {
                                                                image.put_pixel(x, y, Rgba(pbr.base_color_factor().map(|c| (c * u8::MAX as f32) as u8)))
                                                            })
                                                        })
                                                })
                                                .pipe(|image| Texture::from_image(&image, "BASE".into()))
                                                .pipe(|texture| MaterialPlugin::load("BASE", texture))
                                                .pipe(Ok)
                                        })
                                })
                            })
                            .map(
                                #[allow(deprecated)]
                                {
                                    |material| (WithId::register(mesh), WithId::register(material))
                                },
                            )
                            .map(|(mesh, material)| Primitive { mesh, material })
                    })
            })
    }
    // pub fn load_gltf_bytes(gltf_bytes: &[u8]) -> Result<Self> {
    //     gltf::import_slice(gltf_bytes)
    //         .context("invalid bytes")
    //         .and_then(|(document, buffer_data, image_data)| {
    //             tracing::debug!(
    //                 "loading document:\n{}",
    //                 serde_json::to_string_pretty(document.as_json()).expect("serializing json")
    //             );
    //             document
    //                 .meshes()
    //                 .flat_map(|mesh| {})
    //                 .reduce(|acc, next| {
    //                     acc.and_then(|acc| {
    //                         next.map(|next| {
    //                             acc.tap_mut(|acc| {
    //                                 acc.meshes.extend(next.meshes);
    //                                 acc.materials.extend(next.materials);
    //                             })
    //                         })
    //                     })
    //                 })
    //                 .context("model cannot be empty")
    //                 .and_then(|model| model.context("some data could not be loaded"))
    //         })
    // }
}
