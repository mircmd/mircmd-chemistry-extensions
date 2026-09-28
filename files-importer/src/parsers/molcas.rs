// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["MOLCAS"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Convergence after") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("Cartesian Coordinates / Bohr, Angstrom") {
            qc::skip(&mut lines, 3);
            geometry.push_frame(read_molcas_cartesian(&mut lines));
        }
        if line.contains("Nuclear coordinates for the next iteration / Angstrom")
            || line.contains("Nuclear coordinates of the final structure / Angstrom")
        {
            qc::skip(&mut lines, 3);
            geometry.push_frame(read_molcas_nuclear(&mut lines));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}

fn read_molcas_cartesian<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::read_table(lines, |line| {
        if line.trim() == "--" {
            return None;
        }
        let items: Vec<&str> = line.split_whitespace().collect();
        if items.len() < 8 {
            return None;
        }
        qc::atom_from_columns(&items, 1, 5)
    })
}

fn read_molcas_nuclear<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        qc::atom_from_columns(&items, 0, 1)
    })
}
