use nalgebra::{Isometry3, Vector3};
use paya::command_recorder::CommandRecorder;
use paya::device::Device;
use paya::gpu_resources::BufferId;

use crate::engine::ecs::ecs_world::ECSWorld;
use crate::engine::graphics::device::{
    create_device_buffer_typed, stage_buffer_copy, DeviceResource,
};
use crate::engine::graphics::render_manager::RenderManager;
use crate::engine::resource::{Res, ResMut};
use crate::settings::Settings;

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

pub struct Camera {
    view: nalgebra::Matrix4<f32>,
    transform: nalgebra::Matrix4<f32>,
    projection: nalgebra::Matrix4<f32>,
    proj_view: nalgebra::Matrix4<f32>,
    resolution: (u32, u32),
    aspect_ratio: f32,
    fov: f32,

    buffer: BufferId,
}

impl Camera {
    pub fn new(device: &mut Device) -> Self {
        Self {
            view: nalgebra::Matrix4::identity(),
            transform: nalgebra::Matrix4::identity(),
            projection: nalgebra::Matrix4::identity(),
            proj_view: nalgebra::Matrix4::identity(),
            resolution: (0, 0),
            aspect_ratio: 0.0,
            fov: 0.0,

            buffer: create_device_buffer_typed::<CameraBuffer>(device, "camera_buffer"),
        }
    }

    pub fn update_cameras(
        mut ecs_world: ResMut<ECSWorld>,
        render_manager: Res<RenderManager>,
        device: Res<DeviceResource>,
        settings: Res<Settings>,
    ) {
        if let Some(backbuffer_id) = render_manager.backbuffer() {
            let backbuffer_extent = device.get_image(backbuffer_id).info.extent;
            for (_, (camera, transform)) in ecs_world.query_mut::<(&mut Camera, &Transform)>() {
                let eye = transform.isometry.translation.vector;
                let target = eye + transform.isometry * Vector3::z();
                let aspect_ratio = backbuffer_extent.width as f32 / backbuffer_extent.height as f32;

                camera.transform = transform.to_matrix().transpose();
                camera.view = Isometry3::look_at_rh(&eye.into(), &target.into(), &-Vector3::y())
                    .to_homogeneous();
                camera.projection =
                    nalgebra::Perspective3::new(aspect_ratio, settings.camera_fov, 0.1, 1000.0)
                        .into();
                camera.proj_view = camera.projection * camera.view;
                camera.resolution = (backbuffer_extent.width, backbuffer_extent.height);
                camera.aspect_ratio = aspect_ratio;
                camera.fov = settings.camera_fov;
            }
        }
    }

    pub fn record_copy_commands(
        &self,
        device: &mut Device,
        command_recorder: &mut CommandRecorder,
    ) {
        stage_buffer_copy(
            device,
            command_recorder,
            self.buffer,
            |ptr: *mut CameraBuffer| {
                let data = CameraBuffer {
                    transform_matrix: self.transform.as_slice().try_into().unwrap(),
                    view_matrix: self.view.as_slice().try_into().unwrap(),
                    proj_view_matrix: self.proj_view.as_slice().try_into().unwrap(),
                    resolution: self.resolution,
                    aspect: self.aspect_ratio,
                    fov: self.fov,
                };
                unsafe { ptr.write(data) };
            },
        )
    }

    pub fn buffer(&self) -> BufferId {
        self.buffer
    }
}
