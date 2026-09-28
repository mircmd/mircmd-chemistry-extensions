// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["PROGRAM SYSTEM MOLPRO"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Geometry optimization converged") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("ATOMIC COORDINATES") {
            qc::skip(&mut lines, 3);
            geometry.push_frame(qc::read_table(&mut lines, |row| {
                let items: Vec<&str> = row.split_whitespace().collect();
                let (atomic_num, x, y, z) = qc::atom_from_columns(&items, 2, 3)?;
                Some((
                    atomic_num,
                    qc::bohr_to_angstrom(x),
                    qc::bohr_to_angstrom(y),
                    qc::bohr_to_angstrom(z),
                ))
            }));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}
