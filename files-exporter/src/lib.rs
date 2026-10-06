// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

#[cfg(any(target_arch = "wasm32", test))]
mod xyz;

#[cfg(target_arch = "wasm32")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "file-exporter-extension",
    });
}

#[cfg(target_arch = "wasm32")]
struct Extension;

#[cfg(target_arch = "wasm32")]
impl bindings::exports::mircmd::file_exporter::exporter::Guest for Extension {
    fn formats() -> Result<Vec<bindings::mircmd::file_exporter::types::Format>, String> {
        Ok(vec![bindings::mircmd::file_exporter::types::Format {
            id: "xyz".into(),
            name: "XYZ".into(),
            extensions: vec!["xyz".into()],
            supported_node_types: vec!["mircmd:chemistry:atomic_coordinates".into()],
            settings_schema: Some(include_str!("xyz.schema.json").into()),
        }])
    }

    fn dump(
        data: Vec<u8>,
        format_id: String,
        settings: String,
        file_path: String,
    ) -> Result<(), String> {
        let content = match format_id.as_str() {
            "xyz" => xyz::render(&data, &settings)?,
            _ => return Err(format!("Unsupported export format: {format_id}")),
        };
        std::fs::write(&file_path, content)
            .map_err(|error| format!("Cannot write {file_path}: {error}"))
    }
}

#[cfg(target_arch = "wasm32")]
bindings::export!(Extension with_types_in bindings);
