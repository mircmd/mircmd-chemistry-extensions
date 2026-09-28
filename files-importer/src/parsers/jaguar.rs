// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["Jaguar version"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Geometry optimization complete") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("new geometry") || line.contains("Symmetrized geometry") || line.contains("Input geometry") {
            if line.contains("Symmetrized geometry") {
                geometry.frames.clear();
            }
            qc::skip(&mut lines, 2);
            geometry.push_frame(qc::read_table(&mut lines, |row| {
                let items: Vec<&str> = row.split_whitespace().collect();
                qc::atom_from_columns(&items, 0, 1)
            }));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}
