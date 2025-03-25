use {
    super::{
        camera::{Camera, CameraPlugin},
        model::{Primitive, RenderPassDrawModelExt},
        wgpu_ext::{bind_group::HasBindGroup, buffer::storage::StorageBuffer, global_context::device},
    },
    crate::bind_group_layout,
    anyhow::{Context, Result},
    futures::{FutureExt, StreamExt, TryStreamExt},
    shader_types::Instance,
    std::{collections::BTreeMap, ops::Range},
    tap::prelude::*,
};

bind_group_layout!(
    InstanceSyncBuffer,
    wgpu::BindGroupLayoutDescriptor {
        label: struct_label!(),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },],
    }
);

pub struct InstanceSyncBuffer {
    staging: Vec<Instance>,
    commit: StorageBuffer<Instance>,
    bind_group: wgpu::BindGroup,
}

impl AsMut<Vec<Instance>> for InstanceSyncBuffer {
    fn as_mut(&mut self) -> &mut Vec<Instance> {
        &mut self.staging
    }
}

impl InstanceSyncBuffer {
    pub fn new_init(size: usize, init: Vec<Instance>) -> Self {
        Self::new(size).tap_mut(|b| b.staging.extend(init))
    }
    pub fn new(size: usize) -> Self {
        let commit = StorageBuffer::new_empty(size);
        Self {
            staging: Default::default(),
            bind_group: device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: struct_label!(),
                layout: Self::bind_group_layout(),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: commit.as_ref().as_entire_binding(),
                }],
            }),
            commit,
        }
    }
    pub async fn finish(&mut self) -> Result<Option<(&wgpu::BindGroup, Range<u32>)>> {
        if self.staging.is_empty() {
            Ok(None)
        } else {
            let current_len = self.staging.len();

            let commit = std::mem::replace(&mut self.staging, Vec::with_capacity(current_len));
            self.commit
                .write(0..self.staging.len() as _, move |data| data.copy_from_slice(&commit))
                .await
                .map(|_| Some((&self.bind_group, (0..current_len as u32))))
        }
    }
}

pub struct WithInstance<T> {
    pub instance: Instance,
    pub inner: T,
}

impl<T> WithInstance<&T> {
    pub fn copied(&self) -> WithInstance<T>
    where
        T: Copy,
    {
        WithInstance {
            inner: *self.inner,
            instance: self.instance,
        }
    }
}

impl<T> WithInstance<T> {
    pub fn as_ref(&self) -> WithInstance<&T> {
        WithInstance {
            inner: &self.inner,
            instance: self.instance,
        }
    }
}

pub mod model;
pub mod node;
pub mod primitive;

#[derive(Default)]
pub struct PassBuffer {
    queue: BTreeMap<Primitive, InstanceSyncBuffer>,
}

pub struct RenderPass<'pass, 'encoder> {
    pub(crate) buffer: &'pass mut PassBuffer,
    pub(crate) camera: Option<Camera>,
    pub(crate) camera_plugin: &'pass mut CameraPlugin,
    pub(crate) pass: &'pass mut wgpu::RenderPass<'encoder>,
}

pub trait DrawMe {
    fn draw_me<'a, 'b>(&self, pass: &mut RenderPass<'a, 'b>) -> Result<()>;
}

impl<'pass, 'renderer> RenderPass<'pass, 'renderer> {
    pub async fn finish(self) -> Result<()> {
        if let Some(camera) = self.camera {
            self.camera_plugin
                .buffer
                .write(0..1u64, move |buf| {
                    buf[0] = camera.get_view_projection();
                })
                .await
                .context("writing camera")?;
        }

        self.buffer
            .queue
            .iter_mut()
            .pipe(futures::stream::iter)
            .filter_map(|(primitive, buffer)| async move {
                buffer
                    .finish()
                    .map(|finished| {
                        finished
                            .transpose()
                            .map(|finished| finished.map(|finished| (primitive, finished)))
                    })
                    .await
            })
            .try_collect::<Vec<_>>()
            .await
            .context("not everyt buffer could be flushed")
            .map(|flushed| {
                flushed
                    .into_iter()
                    .for_each(|(primitive, (instance_buffer, instances))| {
                        self.pass.set_bind_group(3, instance_buffer, &[]);
                        self.pass.draw_primitive_instanced(primitive, instances);
                    })
            })
    }
    pub fn draw<T: DrawMe>(&mut self, item: &T) -> Result<()> {
        item.draw_me(self)
            .with_context(|| format!("rendering [{}]", std::any::type_name::<T>()))
    }
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = Some(camera);
    }
}
