pub struct PlayerTag;
use nalgebra::Vector3;
use paya::device::Device;

use crate::engine::{
    common::{camera::Camera, time::Time, transform::Transform},
    ecs::ecs_world::ECSWorld,
    input::Input,
    resource::Res,
};

struct PlayerController {
    euler_angles: Vector3<f32>,
}

pub fn spawn_player(world: &mut ECSWorld, device: &mut Device) {
    world.spawn((
        PlayerTag,
        Camera::new(device),
        Transform::new(),
        PlayerController {
            euler_angles: Vector3::new(0.0, 0.0, 0.0),
        },
    ));
}

pub fn update_player_controller(ecs_world: Res<ECSWorld>, input: Res<Input>, time: Res<Time>) {
    let (_, (transform, controller)) = ecs_world
        .query::<(&mut Transform, &mut PlayerController)>()
        .with::<&PlayerTag>()
        .iter()
        .next()
        .expect("Player was not spawned.");
}
