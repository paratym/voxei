use ash::vk;
use voxei_macros::Resource;

use crate::constants;
use crate::engine::graphics::render_manager::FrameIndex;
use crate::engine::graphics::vulkan::objects::glsl::GlslMat4f;
use crate::engine::graphics::vulkan::objects::image::Image;
use crate::engine::resource::Res;
use crate::game::graphics::gfx_constants;

use crate::engine::{
    graphics::{
        resource_manager::RenderResourceManager,
        vulkan::{
            allocator::VulkanMemoryAllocator,
            objects::buffer::{Buffer, BufferCreateInfo},
            vulkan::Vulkan,
        },
    },
    resource::ResMut,
};
use crate::settings::Settings;

use super::transform::Transform;

#[derive(Resource)]
pub struct PrimaryCamera {
    camera: Camera,
    transform: Transform,
}

impl PrimaryCamera {
    pub fn new(vulkan: &Vulkan, vulkan_memory_allocator: &mut VulkanMemoryAllocator) -> Self {
        let transform = Transform::new();

        Self {
            camera: Camera::new(vulkan, vulkan_memory_allocator),
            transform,
        }
    }

    pub fn update(
        mut primary_camera: ResMut<PrimaryCamera>,
        settings: Res<Settings>,
        render_resource_manager: Res<RenderResourceManager>,
        frame_index: Res<FrameIndex>,
    ) {
        let primary_camera = &mut *primary_camera;

        primary_camera
            .camera
            .update(&primary_camera.transform, frame_index.index());

        if let Some(backbuffer_info) =
            render_resource_manager.get_image(gfx_constants::BACKBUFFER_IMAGE_NAME)
        {
            let backbuffer_info = backbuffer_info.instance().info();
            let aspect_ratio = backbuffer_info.width() as f32 / backbuffer_info.height() as f32;
            if settings.camera_fov != primary_camera.camera.fov
                || primary_camera.camera.aspect_ratio != aspect_ratio
            {
                primary_camera.camera.fov = settings.camera_fov;
                primary_camera.camera.aspect_ratio = aspect_ratio;

                primary_camera.camera.refresh_projection(
                    primary_camera.camera.aspect_ratio,
                    primary_camera.camera.fov,
                    0.1,
                    1000.0,
                );
            }
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }
}

pub struct Camera {
    view: nalgebra::Matrix4<f32>,
    projection: nalgebra::Matrix4<f32>,
    proj_view: nalgebra::Matrix4<f32>,
    buffer_data: CameraBufferData,
    buffers: [Buffer; constants::FRAMES_IN_FLIGHT],

    fov: f32,
    aspect_ratio: f32,
}

pub struct CameraBufferData {
    view: GlslMat4f,
    projection: GlslMat4f,
    proj_view: GlslMat4f,
}

impl Default for CameraBufferData {
    fn default() -> Self {
        Self {
            view: GlslMat4f::default(),
            projection: GlslMat4f::default(),
            proj_view: GlslMat4f::default(),
        }
    }
}

impl Camera {
    pub fn new(vulkan: &Vulkan, vulkan_memory_allocator: &mut VulkanMemoryAllocator) -> Self {
        let buffers = (0..2)
            .map(|_| {
                Buffer::new(
                    vulkan,
                    vulkan_memory_allocator,
                    &BufferCreateInfo {
                        size: std::mem::size_of::<CameraBufferData>() as u64,
                        usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                        memory_usage: vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    },
                )
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("Failed to create buffers");

        Self {
            view: nalgebra::Matrix4::identity(),
            projection: nalgebra::Matrix4::identity(),
            proj_view: nalgebra::Matrix4::identity(),
            buffer_data: CameraBufferData::default(),
            buffers,
            fov: 0.0,
            aspect_ratio: 0.0,
        }
    }

    pub fn refresh_projection(&mut self, aspect_ratio: f32, fov: f32, near: f32, far: f32) {
        self.projection =
            nalgebra::Perspective3::new(aspect_ratio, fov, near, far).to_homogeneous();
    }

    pub fn update(&mut self, transform: &Transform, frame_index: usize) {
        self.view = transform.to_matrix();

        self.proj_view = self.projection * self.view;
        self.buffer_data
            .view
            .arr
            .clone_from_slice(self.view.as_slice());
        self.buffer_data
            .projection
            .arr
            .clone_from_slice(self.projection.as_slice());
        self.buffer_data
            .proj_view
            .arr
            .clone_from_slice(self.proj_view.as_slice());

        let map_ptr = self.buffers[frame_index]
            .instance()
            .allocation()
            .instance()
            .map_memory(0);

        unsafe {
            std::ptr::copy_nonoverlapping(
                &self.buffer_data as *const CameraBufferData as *const u8,
                map_ptr as *mut u8,
                std::mem::size_of::<CameraBufferData>(),
            );
        }

        self.buffers[frame_index]
            .instance()
            .allocation()
            .instance()
            .unmap_memory();
    }

    pub fn uniform_buffer(&self, frame_index: usize) -> &Buffer {
        &self.buffers[frame_index]
    }
}
