// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(
        content,
        &["Psi4: An Open-Source Ab Initio Electronic Structure Package"],
    ))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Optimization complete") || line.contains("**** Optimization complete") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("Optimization failed") {
            geometry.is_optimization = true;
        }
        if line.contains("Geometry (in") {
            parse_charge(&mut geometry, line);
            let scale = if line.contains("Bohr") { qc::BOHR2ANGSTROM } else { 1.0 };
            skip_geometry_header(&mut lines);
            geometry.push_frame(read_geometry(&mut lines, scale));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}

fn parse_charge(geometry: &mut QcGeometry, line: &str) {
    let items: Vec<&str> = line.split_whitespace().collect();
    if let Some(index) = items.iter().position(|item| *item == "charge") {
        if let Some(value) = items.get(index + 2) {
            if let Ok(charge) = value.trim_matches(|c| c == ',' || c == ':').parse() {
                geometry.charge = charge;
            }
        }
    }
}

fn skip_geometry_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    if lines.peek().map(|line| line.trim().is_empty()).unwrap_or(false) {
        lines.next();
    }
    if lines
        .peek()
        .map(|line| line.split_whitespace().next() == Some("Center"))
        .unwrap_or(false)
    {
        qc::skip(lines, 2);
    }
}

fn read_geometry<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>, scale: f64) -> CoordFrame {
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        let (atomic_num, x, y, z) = qc::atom_from_columns(&items, 0, 1)?;
        Some((atomic_num, x * scale, y * scale, z * scale))
    })
}
