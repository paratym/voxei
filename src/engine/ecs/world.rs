use voxei_macros::Resource;

use crate::engine::system::SystemParam;

#[derive(Resource)]
pub struct World {
    world: hecs::World,
}

impl World {
    pub fn new() -> World {
        World {
            world: hecs::World::new(),
        }
    }

    pub fn world(&mut self) -> &mut hecs::World {
        &mut self.world
    }
}
