use paya::{
    device::Device,
    pipeline::{ComputePipeline, ComputePipelineInfo},
    shader::ShaderInfo,
};
use voxei_macros::Resource;

use crate::engine::{
    assets::{
        asset::Assets,
        watched_shaders::{self, ShaderDependencySignal, WatchedShaders},
    },
    resource::{Res, ResMut},
};

use super::device::DeviceResource;

pub type PipelineId = usize;

struct ComputePipelineGroup {
    pipeline: Option<ComputePipeline>,
    signal: ShaderDependencySignal,
    shader_name: String,
    push_constants_size: u32,
}

#[derive(Resource)]
pub struct PipelineManager {
    compute_pipelines: Vec<ComputePipelineGroup>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            compute_pipelines: Vec::new(),
        }
    }

    pub fn create_compute_pipeline<T>(
        &mut self,
        assets: &mut Assets,
        watched_shaders: &mut WatchedShaders,
        file_path: String,
    ) -> PipelineId {
        let shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(assets, file_path.clone(), file_path.clone(), &shader_signal);

        let index = self.compute_pipelines.len();

        self.compute_pipelines.push(ComputePipelineGroup {
            pipeline: None,
            signal: shader_signal,
            shader_name: file_path,
            push_constants_size: std::mem::size_of::<T>() as u32,
        });

        index
    }

    pub fn update(
        mut pipeline_manager: ResMut<PipelineManager>,
        watched_shaders: Res<WatchedShaders>,
        device: Res<DeviceResource>,
    ) {
        for pipeline in pipeline_manager.compute_pipelines.iter_mut() {
            if watched_shaders.is_dependency_signaled(&pipeline.signal) {
                let shader = watched_shaders
                    .get_shader(pipeline.shader_name.clone())
                    .unwrap();
                pipeline.pipeline = Some(device.create_compute_pipeline(ComputePipelineInfo {
                    shader: ShaderInfo {
                        byte_code: shader,
                        entry_point: "main".to_string(),
                    },
                    push_constant_size: pipeline.push_constants_size,
                }))
            }
        }
    }

    pub fn get_compute_pipeline(&self, id: PipelineId) -> Option<&ComputePipeline> {
        self.compute_pipelines[id].pipeline.as_ref()
    }
}
