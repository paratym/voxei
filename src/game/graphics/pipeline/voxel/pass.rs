use voxei_macros::Resource;

#[derive(Resource)]
pub struct VoxelRenderPass {}

impl VoxelRenderPass {
    pub fn new() -> Self {
        Self {}
    }
}
