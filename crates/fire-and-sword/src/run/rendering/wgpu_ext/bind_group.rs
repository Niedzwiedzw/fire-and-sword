use wgpu::BindGroupLayout;

pub trait HasBindGroup {
    fn bind_group_layout() -> &'static BindGroupLayout;
}

#[macro_export]
macro_rules! bind_group_layout {
    ($ty:ty, $layout:expr) => {
        bind_group_layout!($ty, STATIC_BIND_GROUP, $layout);
    };

    ($ty:ty, $name:ident, $layout:expr) => {
        static $name: std::sync::OnceLock<&'static wgpu::BindGroupLayout> = std::sync::OnceLock::new();
        impl $crate::run::rendering::wgpu_ext::bind_group::HasBindGroup for $ty {
            fn bind_group_layout() -> &'static wgpu::BindGroupLayout {
                use tap::prelude::*;
                self::$name
                    .get_or_init(|| {
                        tracing::debug!("registering new layout: {:#?}", $layout);
                        device()
                            .create_bind_group_layout(&$layout)
                            .pipe(Box::new)
                            .pipe(Box::leak)
                    })
                    .clone()
            }
        }
    };
}
