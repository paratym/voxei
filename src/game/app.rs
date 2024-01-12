use winit::event_loop::EventLoop;

use crate::engine::resource::ResourceBank;
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
                        WinitWindowEvent::RedrawRequested => {
                            todo!("Redraw requested");
                        }
                        _ => {}
                    },
                    _ => {}
                }
            })
            .expect("Failed to run event loop");
    }
}
