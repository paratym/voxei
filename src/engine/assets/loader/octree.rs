use tobj::GPU_LOAD_OPTIONS;

use crate::engine::{
    assets::asset::{AssetLoadError, AssetLoader},
    voxel::octree::VoxelSVO,
};

pub struct OctreeLoader;

impl AssetLoader for OctreeLoader {
    type Asset = VoxelSVO;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self
    }

    fn load(&self, file_path: String) -> Result<Self::Asset, AssetLoadError>
    where
        Self: Sized,
    {
        let bytes = std::fs::read(file_path.clone()).unwrap();

        Ok(ron::de::from_bytes(&bytes).unwrap())
    }

    fn identifiers() -> &'static [&'static str] {
        &["svo"]
    }
}
