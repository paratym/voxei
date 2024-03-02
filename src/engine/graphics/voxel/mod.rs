use paya::{
    device::Device,
    gpu_resources::{GpuResourceId, PackedGpuResourceId},
    pipeline::{ComputePipeline, ComputePipelineInfo, ShaderInfo},
};

use crate::engine::assets::{
    asset::Assets,
    watched_shaders::{DependencySignal, WatchedShaders},
};

const NAME: &str = "ray_march";
const PATH: &str = "shaders/ray_march.comp.glsl";

#[repr(C)]
pub struct RayMarchPushConstants {
    pub backbuffer_image: PackedGpuResourceId,
    pub camera_buffer: PackedGpuResourceId,
}

pub struct VoxelRayMarchPipeline {
    pipeline: Option<ComputePipeline>,
    shader_signal: DependencySignal,
}

impl VoxelRayMarchPipeline {
    pub fn new(assets: &mut Assets, watched_shaders: &mut WatchedShaders) -> Self {
        let shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(assets, PATH, NAME, &shader_signal);

        Self {
            pipeline: None,
            shader_signal,
        }
    }

    pub fn update(&mut self, device: &Device, watched_shaders: &WatchedShaders) {
        if watched_shaders.is_dependency_signaled(&self.shader_signal) {
            let shader = watched_shaders.get_shader(NAME).unwrap();
            self.pipeline = Some(device.create_compute_pipeline(ComputePipelineInfo {
                shader: ShaderInfo {
                    byte_code: shader,
                    entry_point: "main".to_string(),
                },
                push_constant_size: std::mem::size_of::<RayMarchPushConstants>() as u32,
            }));
        }
    }

    pub fn pipeline(&self) -> Option<&ComputePipeline> {
        self.pipeline.as_ref()
    }
}
