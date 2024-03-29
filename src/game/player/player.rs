pub struct PlayerTag;
use nalgebra::{Quaternion, UnitQuaternion, Vector3};
use paya::device::Device;

use crate::{
    engine::{
        common::{camera::Camera, time::Time, transform::Transform},
        ecs::ecs_world::ECSWorld,
        input::{keyboard::Key, Input},
        resource::{Res, ResMut},
        window::window::Window,
    },
    settings::Settings,
};

struct PlayerController {
    // In terms of radians
    euler_angles: Vector3<f32>,

    walk_speed: f32,
    paused: bool,
}

pub fn spawn_player(world: &mut ECSWorld, device: &mut Device) {
    world.spawn((
        PlayerTag,
        Camera::new(device),
        Transform::new(),
        PlayerController {
            euler_angles: Vector3::new(0.0, 0.0, 0.0),
            walk_speed: 5.0,
            paused: true,
        },
    ));
}

pub fn update_player_controller(
    ecs_world: Res<ECSWorld>,
    input: Res<Input>,
    time: Res<Time>,
    settings: Res<Settings>,
    mut window: ResMut<Window>,
) {
    let mut query = ecs_world
        .query::<(&mut Transform, &mut PlayerController)>()
        .with::<&PlayerTag>();
    let (_, (transform, controller)) = query.iter().next().expect("Player was not spawned.");

    if input.is_key_pressed(Key::Tab) {
        controller.paused = !controller.paused;

        window.set_cursor_grabbed(!controller.paused);
        window.set_cursor_visible(controller.paused);
    }

    let mouse_delta = input.mouse_delta();
    if (mouse_delta.0 != 0.0 || mouse_delta.1 != 0.0) && !controller.paused {
        controller.euler_angles.y +=
            (mouse_delta.0 as f32 * settings.mouse_sensitivity).to_radians();
        controller.euler_angles.x +=
            (mouse_delta.1 as f32 * settings.mouse_sensitivity).to_radians();

        transform.isometry.rotation = UnitQuaternion::from_euler_angles(
            controller.euler_angles.x,
            controller.euler_angles.y,
            0.0,
        );
    }

    let mut delta = Vector3::new(input.horizontal_axis(), 0.0, input.vertical_axis());
    if input.is_key_down(Key::Space) {
        delta.y = 1.0;
    }
    if input.is_key_down(Key::LShift) {
        delta.y = -1.0;
    }

    let mut translation = Vector3::new(0.0, 0.0, 0.0);
    if delta.x != 0.0 || delta.z != 0.0 {
        let xz_delta = transform.isometry.rotation
            * Vector3::new(delta.x, 0.0, delta.z).normalize()
            * controller.walk_speed;

        translation.x = xz_delta.x;
        translation.z = xz_delta.z;
    }
    translation.y += delta.y * controller.walk_speed;

    transform.isometry.translation.vector += translation * time.delta_time().as_secs_f32();
}
