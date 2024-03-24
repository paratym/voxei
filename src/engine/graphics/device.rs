use std::ops::{Deref, DerefMut};

use paya::{
    allocator::MemoryFlags,
    command_recorder::{self, CommandRecorder},
    common::{AccessFlags, BufferTransition, BufferUsageFlags},
    device::{Device, DeviceType, TypedMappedPtr},
    gpu_resources::{BufferId, BufferInfo},
    instance::{Instance, InstanceCreateInfo},
};
use raw_window_handle::HasDisplayHandle;
use voxei_macros::Resource;

#[derive(Resource)]
pub struct DeviceResource {
    instance: Instance,
    device: Device,
}

impl Deref for DeviceResource {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl DerefMut for DeviceResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}

impl DeviceResource {
    pub fn new(display_handle: &dyn HasDisplayHandle) -> Self {
        let instance = Instance::new(InstanceCreateInfo {
            display_handle: Some(display_handle),
        });
        let device = Device::new(&instance, |properies| match properies.device_type {
            DeviceType::Discrete => 100,
            _ => 0,
        });

        Self { instance, device }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut Device {
        &mut self.device
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}

pub fn create_device_buffer(device: &mut Device, name: impl Into<String>, size: u64) -> BufferId {
    device.create_buffer(BufferInfo {
        name: name.into(),
        size,
        memory_flags: MemoryFlags::DEVICE_LOCAL,
        usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
    })
}

pub fn create_device_buffer_typed<T>(device: &mut Device, name: impl Into<String>) -> BufferId {
    create_device_buffer(device, name, std::mem::size_of::<T>() as u64)
}

pub fn stage_buffer_copy<T>(
    device: &mut Device,
    command_recorder: &mut CommandRecorder,
    dst_buffer_id: BufferId,
    copy_fn: impl Fn(*mut T),
) {
    let dst_buffer = device.get_buffer(dst_buffer_id).info.clone();
    let staging_buffer = device.create_buffer(BufferInfo {
        name: format!("{}_staging_buffer", dst_buffer.name).to_owned(),
        size: dst_buffer.size,
        memory_flags: MemoryFlags::HOST_VISIBLE | MemoryFlags::HOST_COHERENT,
        usage: BufferUsageFlags::TRANSFER_SRC,
    });

    let ptr = device.map_buffer_typed::<T>(staging_buffer);
    copy_fn(*ptr);

    command_recorder.copy_buffer_to_buffer(
        device,
        staging_buffer,
        0,
        dst_buffer_id,
        0,
        dst_buffer.size,
    );

    command_recorder.destroy_buffer_deferred(staging_buffer);

    command_recorder.pipeline_barrier_buffer_transition(
        device,
        BufferTransition {
            buffer: dst_buffer_id,
            src_access: AccessFlags::TRANSFER_WRITE,
            dst_access: AccessFlags::SHADER_READ,
        },
    );
}
