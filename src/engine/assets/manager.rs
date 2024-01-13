use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::Path,
};

use voxei_macros::Resource;

use super::{asset::Asset, loader::AssetLoader};

#[derive(Resource)]
pub struct Assets {
    assets: HashMap<String, AssetData>,
    loaders: HashMap<String, Box<dyn AssetLoader>>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            loaders: HashMap::new(),
        }
    }

    pub fn register_loader<T: AssetLoader + Clone + 'static>(
        &mut self,
        extensions: Vec<String>,
        loader: T,
    ) {
        for extension in extensions {
            self.loaders.insert(extension, Box::new(loader.clone()));
        }
    }

    pub fn load<T: Asset>(&mut self, path: String) {
        let extension = Path::new(&path)
            .extension()
            .expect("Failed to get extension")
            .to_str()
            .expect("Failed to convert extension to str")
            .to_string();

        let loader = self.loaders.get(&extension).expect(&format!(
            "Failed to get loader for extension: {}",
            extension
        ));

        if loader.type_id() != TypeId::of::<T>() {
            panic!(
                "Loader type id does not match asset type id: {:?} != {:?}",
                loader.type_id(),
                TypeId::of::<T>()
            );
        }

        let data = loader.load(path.clone());

        self.assets.insert(
            path,
            AssetData {
                type_id: TypeId::of::<T>(),
                data,
            },
        );
    }

    pub fn get<T: Asset>(&self, path: &Path) -> Option<&T> {
        self.assets.get(path.to_str().unwrap()).map(|asset| {
            asset.data.downcast_ref::<T>().expect(
                format!(
                    "Failed to downcast asset data to type {:?}",
                    TypeId::of::<T>()
                )
                .as_str(),
            )
        })
    }
}

pub struct AssetData {
    type_id: TypeId,
    data: Box<dyn Asset>,
}
