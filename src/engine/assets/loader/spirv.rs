use std::time::Duration;

use paya::shader::ShaderCompiler;

use crate::engine::assets::asset::{AssetLoadError, AssetLoader};

pub struct SpirVLoader {
    compiler: ShaderCompiler,
}

impl AssetLoader for SpirVLoader {
    type Asset = Vec<u32>;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            compiler: ShaderCompiler::new(),
        }
    }

    fn load(&self, file_path: String) -> Result<Self::Asset, AssetLoadError>
    where
        Self: Sized,
    {
        Ok(self.compiler.load_from_file(file_path))
    }

    fn identifiers() -> &'static [&'static str] {
        &["glsl", "vert", "frag", "comp"]
    }
}
