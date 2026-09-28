// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(
        content,
        &["Northwest Computational Chemistry Package"],
    ))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut angstrom = Vec::new();
    let mut bohr = Vec::new();
    let mut module = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Optimization converged") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("NWChem Geometry Optimization") {
            geometry.is_optimization = true;
        }
        if line.contains("Output coordinates in angstroms") {
            skip_nwchem_header(&mut lines);
            angstrom.push(read_nwchem_table(&mut lines, 1.0));
        } else if line.contains("Output coordinates in a.u.") {
            skip_nwchem_header(&mut lines);
            bohr.push(read_nwchem_table(&mut lines, qc::BOHR2ANGSTROM));
        } else if line.trim() == "Geometry \"geometry\" -> \"\""
            || line.trim() == "Geometry \"geometry\" -> \"geometry\""
        {
            qc::skip(&mut lines, 6);
            module.push(read_nwchem_table(&mut lines, qc::BOHR2ANGSTROM));
        }
    }

    geometry.frames = qc::prefer_frames(angstrom, qc::prefer_frames(module, bohr));
    geometry.frames.retain(|frame| !frame.is_empty());
    qc::to_molecule_tree(file_name, geometry)
}

fn skip_nwchem_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    for _ in 0..6 {
        let Some(line) = lines.peek() else { break };
        if line.contains("----") {
            lines.next();
            break;
        }
        lines.next();
    }
}

fn read_nwchem_table<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>, scale: f64) -> CoordFrame {
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        let (z_col, x_col) = if items.len() >= 7 { (3, 4) } else { (2, 3) };
        let (atomic_num, x, y, z) = qc::atom_from_columns(&items, z_col, x_col)?;
        Some((atomic_num, x * scale, y * scale, z * scale))
    })
}
