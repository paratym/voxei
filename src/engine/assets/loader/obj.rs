use tobj::GPU_LOAD_OPTIONS;

use crate::engine::assets::asset::{AssetLoadError, AssetLoader};

pub struct ObjLoader;

impl AssetLoader for ObjLoader {
    type Asset = Vec<tobj::Model>;

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
        let (models, _) = tobj::load_obj(&file_path, &GPU_LOAD_OPTIONS).unwrap();
        Ok(models)
    }

    fn identifiers() -> &'static [&'static str] {
        &["obj"]
    }
}
