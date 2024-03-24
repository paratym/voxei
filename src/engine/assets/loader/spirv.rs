use paya::shader::{CompilationError, ShaderCompiler};

use crate::engine::assets::asset::{AssetLoadError, AssetLoadErrorKind, AssetLoader};

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
        self.compiler
            .load_from_file(file_path.clone())
            .map_err(|err| match err {
                CompilationError::Undefined { message } => {
                    AssetLoadError::new_invalid_file(file_path, message)
                }
                CompilationError::CompilationErrors { message } => {
                    AssetLoadError::new_invalid_file(file_path, message)
                }
            })
    }

    fn identifiers() -> &'static [&'static str] {
        &["glsl", "vert", "frag", "comp"]
    }
}
