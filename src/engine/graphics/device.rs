use std::ops::{Deref, DerefMut};

use paya::{
    device::{Device, DeviceType},
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
