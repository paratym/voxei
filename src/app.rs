use winit::event_loop::EventLoop;

use crate::engine::{
    graphics::{device::DeviceResource, swapchain::SwapchainResource},
    input::Input,
    resource::ResourceBank,
};
use winit::event::{Event as WinitEvent, WindowEvent as WinitWindowEvent};

pub struct App {
    event_loop: Option<EventLoop<()>>,
    resource_bank: ResourceBank,
}

impl App {
    pub fn new() -> Self {
        Self {
            event_loop: Some(EventLoop::new().expect("Failed to create event loop")),
            resource_bank: ResourceBank::new(),
        }
    }

    pub fn event_loop(&self) -> &EventLoop<()> {
        self.event_loop.as_ref().unwrap()
    }

    pub fn resource_bank(&self) -> &ResourceBank {
        &self.resource_bank
    }

    pub fn resource_bank_mut(&mut self) -> &mut ResourceBank {
        &mut self.resource_bank
    }

    pub fn run(mut self) {
        let event_loop = self.event_loop.take().unwrap();
        event_loop
            .run(move |event, window| {
                window.set_control_flow(winit::event_loop::ControlFlow::Poll);

                match event {
                    WinitEvent::WindowEvent { event, .. } => match event {
                        WinitWindowEvent::CloseRequested => {
                            window.exit();
                        }
                        WinitWindowEvent::Resized(new_size) => {
                            self.resource_bank()
                                .get_resource_mut::<SwapchainResource>()
                                .resize(
                                    &mut self.resource_bank().get_resource_mut::<DeviceResource>(),
                                    new_size.width,
                                    new_size.height,
                                );
                        }
                        event => {
                            self.resource_bank_mut()
                                .get_resource_mut::<Input>()
                                .handle_winit_window_event(event);
                        }
                    },
                    WinitEvent::DeviceEvent { device_id, event } => {
                        self.resource_bank_mut()
                            .get_resource_mut::<Input>()
                            .handle_winit_device_event(device_id, event);
                    }
                    WinitEvent::AboutToWait => {
                        crate::game_loop::game_loop(&mut self);
                    }
                    _ => {}
                }
            })
            .expect("Failed to run event loop");
    }
}
