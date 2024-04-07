use std::ops::{Deref, DerefMut};

use paya::{
    common::ImageUsageFlags,
    device::Device,
    swapchain::{Swapchain, SwapchainCreateInfo},
};
use voxei_macros::Resource;

use crate::{constants, engine::window::window::Window};

#[derive(Resource)]
pub struct SwapchainResource {
    swapchain: Swapchain,
}

impl SwapchainResource {
    pub fn new(device: &mut Device, window: &Window) -> Self {
        Self {
            swapchain: device.create_swapchain(SwapchainCreateInfo {
                display_handle: window,
                window_handle: window,
                image_usage: ImageUsageFlags::TRANSFER_DST,
                preferred_extent: (window.width(), window.height()),
                preferred_image_count: 3,
                max_frames_in_flight: constants::MAX_FRAMES_IN_FLIGHT as u32,
            }),
        }
    }

    pub fn swapchain(&self) -> &Swapchain {
        &self.swapchain
    }

    pub fn swapchain_mut(&mut self) -> &mut Swapchain {
        &mut self.swapchain
    }
}

impl Deref for SwapchainResource {
    type Target = Swapchain;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl DerefMut for SwapchainResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.swapchain
    }
}
