use hecs::{Query, QueryBorrow, With};
use voxei_macros::Resource;

use crate::{
    engine::system::SystemParam,
    game::player::player::{PlayerQuery, PlayerTag},
};

#[derive(Resource)]
pub struct ECSWorld {
    world: hecs::World,
}

impl ECSWorld {
    pub fn new() -> ECSWorld {
        ECSWorld {
            world: hecs::World::new(),
        }
    }

    pub fn world_mut(&mut self) -> &mut hecs::World {
        &mut self.world
    }

    pub fn player_query<'a, Q: Query>(&'a self) -> PlayerQuery<Q> {
        PlayerQuery::new(
            self.query::<Q>().with::<&'a PlayerTag>() as QueryBorrow<'a, With<Q, &'a PlayerTag>>
        )
    }
}

impl std::ops::Deref for ECSWorld {
    type Target = hecs::World;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl std::ops::DerefMut for ECSWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}
