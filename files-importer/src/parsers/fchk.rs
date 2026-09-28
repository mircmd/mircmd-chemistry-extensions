// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(content
        .lines()
        .take(30)
        .any(|line| line.starts_with("Number of atoms") && line.contains(" I")))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut atomic_num = Vec::new();
    let mut coords = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.starts_with("Charge") {
            if let Some(value) = line.split_whitespace().last().and_then(|item| item.parse().ok()) {
                geometry.charge = value;
            }
        }
        if line.starts_with("Atomic numbers") {
            let count = block_count(line);
            atomic_num = qc::read_numbers(&mut lines, count)
                .into_iter()
                .map(|value| value.round() as i32)
                .collect();
        }
        if line.starts_with("Current cartesian coordinates") {
            coords = qc::read_numbers(&mut lines, block_count(line));
        }
    }

    geometry.push_frame(frame_from_fchk(atomic_num, coords)?);
    qc::to_molecule_tree(file_name, geometry)
}

fn block_count(line: &str) -> usize {
    line.split_whitespace()
        .last()
        .and_then(|item| item.parse().ok())
        .unwrap_or(0)
}

fn frame_from_fchk(atomic_num: Vec<i32>, coords: Vec<f64>) -> Result<CoordFrame, String> {
    if atomic_num.is_empty() || coords.len() != atomic_num.len() * 3 {
        return Err("Invalid FChk coordinate block".to_string());
    }
    let mut frame = CoordFrame::default();
    for (index, atomic_number) in atomic_num.into_iter().enumerate() {
        frame.push(
            atomic_number,
            qc::bohr_to_angstrom(coords[index * 3]),
            qc::bohr_to_angstrom(coords[index * 3 + 1]),
            qc::bohr_to_angstrom(coords[index * 3 + 2]),
        );
    }
    Ok(frame)
}
