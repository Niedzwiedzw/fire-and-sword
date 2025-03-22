use {
    std::sync::OnceLock,
    wgpu::{Device, Queue},
};

pub static DEVICE: OnceLock<Device> = OnceLock::new();

pub fn init_device(device: Device) {
    DEVICE.get_or_init(move || device);
}

pub fn device<'a>() -> &'a Device {
    DEVICE.get().expect("device must be initialized")
}

pub static QUEUE: OnceLock<Queue> = OnceLock::new();

pub fn init_queue(queue: Queue) {
    QUEUE.get_or_init(move || queue);
}

pub fn queue<'a>() -> &'a Queue {
    QUEUE.get().expect("queue must be initialized")
}
