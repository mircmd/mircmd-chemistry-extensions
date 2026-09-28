// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(cjson_coords(content).is_some())
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let (atomic_num, coords) = cjson_coords(content).ok_or("Invalid CJSON coordinates")?;
    if coords.len() != atomic_num.len() * 3 {
        return Err("CJSON coordinate count does not match atoms".to_string());
    }
    let mut frame = CoordFrame::default();
    for (index, atomic_number) in atomic_num.into_iter().enumerate() {
        frame.push(
            atomic_number,
            coords[index * 3],
            coords[index * 3 + 1],
            coords[index * 3 + 2],
        );
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        geometry.charge = cjson_charge(&value);
    }
    geometry.push_frame(frame);
    qc::to_molecule_tree(file_name, geometry)
}

fn cjson_coords(content: &str) -> Option<(Vec<i32>, Vec<f64>)> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let coords = value
        .pointer("/atoms/coords/3d")?
        .as_array()?
        .iter()
        .map(|item| item.as_f64())
        .collect::<Option<Vec<_>>>()?;
    let atomic_num = atomic_numbers(&value)?;
    Some((atomic_num, coords))
}

fn atomic_numbers(value: &serde_json::Value) -> Option<Vec<i32>> {
    if let Some(numbers) = value.pointer("/atoms/elements/number").and_then(|item| item.as_array()) {
        return numbers
            .iter()
            .map(|item| item.as_i64().map(|number| number as i32))
            .collect();
    }
    let symbols = value.pointer("/atoms/elements/symbol")?.as_array()?;
    symbols
        .iter()
        .map(|item| qc::atomic_number_from_token(item.as_str()?))
        .collect()
}

fn cjson_charge(value: &serde_json::Value) -> i32 {
    value
        .pointer("/properties/totalCharge")
        .and_then(|item| item.as_i64())
        .or_else(|| value.pointer("/atoms/charge").and_then(|item| item.as_i64()))
        .unwrap_or(0) as i32
}
