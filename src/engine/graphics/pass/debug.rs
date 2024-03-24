use paya::{
    common::{Format, PolygonMode, Topology},
    device::Device,
    gpu_resources::PackedGpuResourceId,
    pipeline::{RasterPipeline, RasterPipelineInfo, RasterVertexAttributeType},
    shader::ShaderInfo,
};

use crate::engine::assets::{
    asset::Assets,
    watched_shaders::{DependencySignal, WatchedShaders},
};

const VERTEX_SHADER_NAME: &str = "debug_vertex";
const VERTEX_PATH: &str = "shaders/debug.vert.glsl";
const FRAGMENT_SHADER_NAME: &str = "debug_fragment";
const FRAGMENT_PATH: &str = "shaders/debug.frag.glsl";

#[repr(C)]
pub struct DebugPushConstants {
    pub camera: PackedGpuResourceId,
}

pub struct DebugPipeline {
    debug_raster_pipeline: Option<RasterPipeline>,
    debug_shader_signal: DependencySignal,
}

impl DebugPipeline {
    pub fn new(assets: &mut Assets, watched_shaders: &mut WatchedShaders) -> Self {
        let debug_shader_signal = watched_shaders.create_dependency_signal();
        watched_shaders.load_shader(
            assets,
            VERTEX_PATH,
            VERTEX_SHADER_NAME,
            &debug_shader_signal,
        );
        watched_shaders.load_shader(
            assets,
            FRAGMENT_PATH,
            FRAGMENT_SHADER_NAME,
            &debug_shader_signal,
        );
        Self {
            debug_raster_pipeline: None,
            debug_shader_signal,
        }
    }

    pub fn update(&mut self, device: &Device, watched_shaders: &WatchedShaders) {
        if watched_shaders.is_dependency_signaled(&self.debug_shader_signal) {
            let vertex_shader = watched_shaders.get_shader(VERTEX_SHADER_NAME).unwrap();
            let fragment_shader = watched_shaders.get_shader(FRAGMENT_SHADER_NAME).unwrap();
            self.debug_raster_pipeline = Some(device.create_raster_pipeline(RasterPipelineInfo {
                vertex_shader: ShaderInfo {
                    byte_code: vertex_shader,
                    entry_point: "main".to_string(),
                },
                fragment_shader: ShaderInfo {
                    byte_code: fragment_shader,
                    entry_point: "main".to_string(),
                },
                push_constant_size: std::mem::size_of::<DebugPushConstants>() as u32,
                vertex_attributes: vec![RasterVertexAttributeType::Vec3],
                polygon_mode: PolygonMode::Line,
                color_attachments: vec![Format::R8G8B8A8Unorm],
                topology: Topology::TriangleList,
                primitive_restart_enable: false,
                line_width: 2.0,
            }));
        }
    }

    pub fn debug_pipeline(&self) -> Option<&RasterPipeline> {
        self.debug_raster_pipeline.as_ref()
    }
}
