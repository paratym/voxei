use std::f32::consts::FRAC_PI_2;

use nalgebra::{Isometry3, Matrix4, Point3, UnitQuaternion, Vector3};
use paya::allocator::MemoryFlags;
use paya::common::BufferUsageFlags;
use paya::device::Device;
use paya::gpu_resources::{BufferId, BufferInfo};
use voxei_macros::Resource;

use crate::constants;
use crate::engine::graphics::device::DeviceResource;
use crate::engine::graphics::render_manager::RenderManager;
use crate::engine::input::keyboard::Key;
use crate::engine::input::Input;
use crate::engine::resource::{Res, ResMut};
use crate::engine::window::window::Window;
use crate::settings::Settings;

use super::time::Time;
use super::transform::Transform;

#[repr(C)]
pub struct CameraBuffer {
    pub transform_matrix: [f32; 16],
    pub view_matrix: [f32; 16],
    pub proj_view_matrix: [f32; 16],
    pub resolution: (u32, u32),
    pub aspect: f32,
    pub fov: f32,
}

#[derive(Resource)]
pub struct PrimaryCamera {
    camera: Camera,
    buffers: Vec<BufferId>,

    transform: Transform,
    euler_angles: Vector3<f32>,
    focused: bool,

    resolution: (u32, u32),
    fov: f32,
    aspect_ratio: f32,
}

impl PrimaryCamera {
    pub fn new(device: &mut Device) -> Self {
        let mut transform = Transform::new();
        transform.isometry.translation.vector = Vector3::new(0.0, 0.0, 0.0);

        let buffers = (0..constants::MAX_FRAMES_IN_FLIGHT)
            .map(|i| {
                device.create_buffer(BufferInfo {
                    name: format!("camera_buffer_{}", i).to_owned(),
                    size: std::mem::size_of::<CameraBuffer>() as u64,
                    memory_flags: MemoryFlags::DEVICE_LOCAL,
                    usage: BufferUsageFlags::STORAGE | BufferUsageFlags::TRANSFER_DST,
                })
            })
            .collect::<Vec<_>>();

        Self {
            camera: Camera::new(),
            buffers,

            transform,
            euler_angles: Vector3::new(0.0, 0.0, 0.0),
            focused: false,

            resolution: (0, 0),
            fov: 0.0,
            aspect_ratio: 0.0,
        }
    }

    pub fn update(
        mut primary_camera: ResMut<PrimaryCamera>,
        mut window: ResMut<Window>,
        render_manager: Res<RenderManager>,
        device: Res<DeviceResource>,
        settings: Res<Settings>,
        input: Res<Input>,
        time: Res<Time>,
    ) {
        if input.is_key_pressed(Key::Tab) {
            primary_camera.focused = !primary_camera.focused;
            window.set_cursor_grabbed(primary_camera.focused);
            window.set_cursor_visible(!primary_camera.focused);
        }

        let mouse_delta = input.mouse().mouse_delta();
        if mouse_delta != (0.0, 0.0) && primary_camera.focused {
            let rx = mouse_delta.0.to_radians();
            let ry = mouse_delta.1.to_radians();

            let euler_angles = nalgebra::Vector3::new(
                (primary_camera.euler_angles.x + ry * 0.5)
                    .clamp(-FRAC_PI_2 + 0.001, FRAC_PI_2 - 0.001),
                primary_camera.euler_angles.y + rx * 0.5,
                0.0,
            );

            primary_camera.euler_angles = euler_angles;
            primary_camera.transform.isometry.rotation =
                UnitQuaternion::from_euler_angles(euler_angles.x, euler_angles.y, euler_angles.z);
        }

        let mut delta = Vector3::new(0.0, 0.0, 0.0);
        if input.is_key_down(Key::W) {
            delta.z += 1.0;
        }
        if input.is_key_down(Key::S) {
            delta.z -= 1.0;
        }
        if input.is_key_down(Key::A) {
            delta.x -= 1.0;
        }
        if input.is_key_down(Key::D) {
            delta.x += 1.0;
        }
        if input.is_key_down(Key::Space) {
            delta.y += 1.0;
        }
        if input.is_key_down(Key::LShift) {
            delta.y -= 1.0;
        }

        let mut speed = 4.0;
        if input.is_key_down(Key::LControl) {
            speed = 8.0;
        }

        if delta.x != 0.0 || delta.y != 0.0 || delta.z != 0.0 {
            let rotation = primary_camera.transform.isometry.rotation;
            let up = Vector3::<f32>::y();
            let mut forward = (rotation * Vector3::<f32>::z()).normalize();
            let mut right = (rotation * Vector3::<f32>::x()).normalize();
            forward.y = 0.0;
            right.y = 0.0;

            let translation = (delta.z * forward + delta.y * up + delta.x * right).normalize();

            primary_camera.transform.isometry.translation.vector +=
                translation * speed * time.delta_time().as_secs_f32();
        }

        // Update camera matrices
        if let Some(backbuffer_id) = render_manager.backbuffer() {
            let extent = &device.get_image(backbuffer_id).info.extent;
            let aspect_ratio = extent.width as f32 / extent.height as f32;
            let fov = settings.camera_fov;

            let fov_changed = fov != primary_camera.fov;
            let aspect_ratio_changed = aspect_ratio != primary_camera.aspect_ratio;
            if fov_changed || aspect_ratio_changed {
                primary_camera.resolution = (extent.width, extent.height);
                primary_camera.aspect_ratio = aspect_ratio;
                primary_camera.fov = fov;

                primary_camera
                    .camera
                    .refresh_projection(aspect_ratio, fov, 0.1, 1000.0);
            }
        }
        let transform = primary_camera.transform.clone();
        primary_camera.camera.refresh_view(transform);
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    pub fn buffer(&self, buffer_index: u64) -> BufferId {
        self.buffers[buffer_index as usize]
    }
}

pub struct Camera {
    view: nalgebra::Matrix4<f32>,
    transform: nalgebra::Matrix4<f32>,
    projection: nalgebra::Matrix4<f32>,
    proj_view: nalgebra::Matrix4<f32>,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            view: nalgebra::Matrix4::identity(),
            transform: nalgebra::Matrix4::identity(),
            projection: nalgebra::Matrix4::identity(),
            proj_view: nalgebra::Matrix4::identity(),
        }
    }

    pub fn refresh_projection(&mut self, aspect_ratio: f32, fov: f32, near: f32, far: f32) {
        self.projection = nalgebra::Perspective3::new(aspect_ratio, fov, near, far)
            .as_matrix()
            .clone();
        self.proj_view = self.projection * self.view;
    }

    pub fn refresh_view(&mut self, transform: Transform) {
        self.transform = transform.to_matrix().transpose();

        let eye = transform.isometry.translation.vector;
        let target = eye + transform.isometry * Vector3::z();
        self.view =
            Isometry3::look_at_rh(&eye.into(), &target.into(), &-Vector3::y()).to_homogeneous();
        self.proj_view = self.projection * self.view;
    }

    pub fn transform(&self) -> &nalgebra::Matrix4<f32> {
        &self.transform
    }

    pub fn view(&self) -> &nalgebra::Matrix4<f32> {
        &self.view
    }

    pub fn projection(&self) -> &nalgebra::Matrix4<f32> {
        &self.projection
    }

    pub fn proj_view(&self) -> &nalgebra::Matrix4<f32> {
        &self.proj_view
    }
}
