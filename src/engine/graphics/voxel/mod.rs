use paya::{
    device::Device,
    gpu_resources::{GpuResourceId, PackedGpuResourceId},
    pipeline::{ComputePipeline, ComputePipelineInfo},
    shader::ShaderInfo,
};

use crate::engine::assets::{
    asset::Assets,
    watched_shaders::{DependencySignal, WatchedShaders},
};

const RAY_MARCH_NAME: &str = "ray_march";
const RAY_MARCH_PATH: &str = "shaders/ray_march.comp.glsl";
const VOXELIZE_NAME: &str = "voxelize";
const VOXELIZE_PATH: &str = "shaders/voxelize.comp.glsl";
const SVO_NAME: &str = "svo_builder";
const SVO_PATH: &str = "shaders/build_voxel_svo.comp.glsl";

#[repr(C)]
pub struct RayMarchPushConstants {
    pub backbuffer_image: PackedGpuResourceId,
    pub camera_buffer: PackedGpuResourceId,
    pub voxel_model_buffer: PackedGpuResourceId,
    pub subdivisions: u32,
}

#[repr(C)]
pub struct VoxelizePushConstants {
    pub vertices_buffer: PackedGpuResourceId,
    pub indices_buffer: PackedGpuResourceId,
    pub voxel_buffer: PackedGpuResourceId,
    pub side_length: u32,
    pub triangle_count: u32,
}

#[repr(C)]
pub struct BuildSVOPushConstants {
    pub voxel_buffer: PackedGpuResourceId,
    pub svo_buffer: PackedGpuResourceId,
    pub head: u32,
    pub size: u32,
}

pub struct VoxelPipeline {
    ray_march_pipeline: Option<ComputePipeline>,
    ray_march_shader_signal: DependencySignal,
    voxelize_pipeline: Option<ComputePipeline>,
    voxelize_shader_signal: DependencySignal,
    build_svo_pipeline: Option<ComputePipeline>,
    build_svo_shader_signal: DependencySignal,
}

impl VoxelPipeline {
    pub fn new(assets: &mut Assets, watched_shaders: &mut WatchedShaders) -> Self {
        let ray_march_shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            RAY_MARCH_PATH,
            RAY_MARCH_NAME,
            &ray_march_shader_signal,
        );
        let voxelize_shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            VOXELIZE_PATH,
            VOXELIZE_NAME,
            &voxelize_shader_signal,
        );
        let build_svo_shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(assets, SVO_PATH, SVO_NAME, &build_svo_shader_signal);

        Self {
            ray_march_pipeline: None,
            ray_march_shader_signal,
            voxelize_pipeline: None,
            voxelize_shader_signal,
            build_svo_pipeline: None,
            build_svo_shader_signal,
        }
    }

    pub fn update(&mut self, device: &Device, watched_shaders: &WatchedShaders) {
        if watched_shaders.is_dependency_signaled(&self.ray_march_shader_signal) {
            let ray_march_shader = watched_shaders.get_shader(RAY_MARCH_NAME).unwrap();
            self.ray_march_pipeline = Some(device.create_compute_pipeline(ComputePipelineInfo {
                shader: ShaderInfo {
                    byte_code: ray_march_shader,
                    entry_point: "main".to_string(),
                },
                push_constant_size: std::mem::size_of::<RayMarchPushConstants>() as u32,
            }));
        }
        if watched_shaders.is_dependency_signaled(&self.voxelize_shader_signal) {
            let voxelize_shader = watched_shaders.get_shader(VOXELIZE_NAME).unwrap();
            self.voxelize_pipeline = Some(device.create_compute_pipeline(ComputePipelineInfo {
                shader: ShaderInfo {
                    byte_code: voxelize_shader,
                    entry_point: "main".to_string(),
                },
                push_constant_size: std::mem::size_of::<VoxelizePushConstants>() as u32,
            }));
        }
        if watched_shaders.is_dependency_signaled(&self.build_svo_shader_signal) {
            let build_svo_shader = watched_shaders.get_shader(SVO_NAME).unwrap();
            self.build_svo_pipeline = Some(device.create_compute_pipeline(ComputePipelineInfo {
                shader: ShaderInfo {
                    byte_code: build_svo_shader,
                    entry_point: "main".to_string(),
                },
                push_constant_size: std::mem::size_of::<BuildSVOPushConstants>() as u32,
            }));
        }
    }

    pub fn ray_march_pipeline(&self) -> Option<&ComputePipeline> {
        self.ray_march_pipeline.as_ref()
    }

    pub fn voxelize_pipeline(&self) -> Option<&ComputePipeline> {
        self.voxelize_pipeline.as_ref()
    }

    pub fn build_svo_pipeline(&self) -> Option<&ComputePipeline> {
        self.build_svo_pipeline.as_ref()
    }
}
