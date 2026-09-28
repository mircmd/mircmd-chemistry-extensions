// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["MOPAC20"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut precise = Vec::new();
    let mut simple = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("CHARGE ON SYSTEM =") {
            if let Some(value) = line.split_whitespace().nth(5).and_then(|item| item.parse().ok()) {
                geometry.charge = value;
            }
        }
        if line.contains("GEOMETRY OPTIMISED") || line.contains("GRADIENTS WERE SATISFIED") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line
            .split_whitespace()
            .eq(["NUMBER", "SYMBOL", "(ANGSTROMS)", "(ANGSTROMS)", "(ANGSTROMS)"])
        {
            qc::skip(&mut lines, 1);
            precise.push(qc::read_table(&mut lines, parse_mopac_precise));
        }
        if line.trim() == "CARTESIAN COORDINATES" {
            skip_mopac_cartesian_header(&mut lines);
            simple.push(qc::read_table(&mut lines, parse_mopac_simple));
        }
    }

    geometry.frames = qc::prefer_frames(precise, simple);
    geometry.frames.retain(|frame| !frame.is_empty());
    qc::to_molecule_tree(file_name, geometry)
}

fn skip_mopac_cartesian_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    for _ in 0..4 {
        let Some(line) = lines.peek() else { break };
        let items: Vec<&str> = line.split_whitespace().collect();
        if items.len() >= 5 && qc::atom_from_columns(&items, 1, 2).is_some() {
            break;
        }
        lines.next();
    }
}

fn parse_mopac_precise(line: &str) -> Option<(i32, f64, f64, f64)> {
    let items: Vec<&str> = line.split_whitespace().collect();
    if items.len() < 7 {
        return None;
    }
    let atomic_num = qc::atomic_number_from_token(items[1])?;
    let x = items[2].parse().ok()?;
    let y = items[4].parse().ok()?;
    let z = items[6].parse().ok()?;
    Some((atomic_num, x, y, z))
}

fn parse_mopac_simple(line: &str) -> Option<(i32, f64, f64, f64)> {
    let items: Vec<&str> = line.split_whitespace().collect();
    qc::atom_from_columns(&items, 1, 2)
}
