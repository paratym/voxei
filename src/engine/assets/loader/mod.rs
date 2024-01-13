use std::any::TypeId;

use super::asset::Asset;

pub trait AssetLoader: Send + Sync {
    fn load(&self, path: String) -> Box<dyn Asset>;
    fn type_id(&self) -> TypeId;
}
