use std::time::Duration;

use shaderc::CompileOptions;

use crate::engine::assets::asset::{AssetLoadError, AssetLoader};

pub struct SpirVLoader {
    compiler: shaderc::Compiler,
}

impl AssetLoader for SpirVLoader {
    type Asset = Vec<u32>;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            compiler: shaderc::Compiler::new().unwrap(),
        }
    }

    fn load(&self, file_path: String) -> Result<Self::Asset, AssetLoadError>
    where
        Self: Sized,
    {
        let mut file_extension = file_path.split('.').last().unwrap();
        if file_extension == "glsl" {
            file_extension = file_path.split('.').nth_back(1).unwrap();
        }

        let shader_kind = match file_extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => panic!("Unknown shader extension: {}", file_extension),
        };

        // TODO: remove this cause its not an issue with this app its nvim
        std::thread::sleep(Duration::from_millis(100));
        let source = std::fs::read_to_string(file_path.clone()).unwrap();

        let mut options = CompileOptions::new().unwrap();
        options.set_include_callback(|name, include_type, source_path, _depth| {
            let mut path = std::path::PathBuf::from(source_path);
            path.pop();
            path.push(name);
            let source = std::fs::read_to_string(path.clone()).unwrap();
            let mut result = shaderc::ResolvedInclude {
                resolved_name: name.to_string(),
                content: source,
            };
            if include_type == shaderc::IncludeType::Relative {
                result.resolved_name = path.to_str().unwrap().to_string();
            }
            Ok(result)
        });

        if cfg!(debug_assertions) {
            options.set_optimization_level(shaderc::OptimizationLevel::Zero);
        } else {
            options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        }

        let binary_result = self
            .compiler
            .compile_into_spirv(&source, shader_kind, &file_path, "main", Some(&options))
            .map_err(|err| AssetLoadError::new_invalid_file(file_path, err.to_string()))?;

        Ok(binary_result.as_binary().to_vec())
    }

    fn identifiers() -> &'static [&'static str] {
        &["glsl", "vert", "frag", "comp"]
    }
}
