// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["G A M E S S - U K"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut initial = Vec::new();
    let mut steps = Vec::new();
    let mut first_nuclear = true;
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("optimization converged") || line.contains("Optimization converged") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.trim() == "molecular geometry" {
            skip_uk_geometry_header(&mut lines);
            initial.push(read_uk_molecular(&mut lines));
        }
        if line.contains("nuclear coordinates") {
            if first_nuclear && !initial.is_empty() {
                first_nuclear = false;
                continue;
            }
            first_nuclear = false;
            qc::skip(&mut lines, 4);
            steps.push(read_uk_nuclear(&mut lines));
        }
    }

    geometry.frames = qc::prefer_frames(steps, initial);
    geometry.frames.retain(|frame| !frame.is_empty());
    if geometry.frames.len() > 1 {
        geometry.is_optimization = true;
    }
    qc::to_molecule_tree(file_name, geometry)
}

fn skip_uk_geometry_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    qc::skip(lines, 4);
    if lines
        .peek()
        .map(|line| line.contains("basis selected is"))
        .unwrap_or(false)
    {
        qc::skip(lines, 4);
    }
    qc::skip(lines, 4);
}

fn read_uk_molecular<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    let mut frame = CoordFrame::default();
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            if !frame.is_empty() {
                break;
            }
            continue;
        }
        if line.trim().chars().all(|c| c == '*') {
            if !frame.is_empty() {
                break;
            }
            continue;
        }
        let items: Vec<&str> = line.split_whitespace().collect();
        if let Some((atomic_num, x, y, z)) = qc::atom_from_columns(&items, 2, 3) {
            frame.push(
                atomic_num,
                qc::bohr_to_angstrom(x),
                qc::bohr_to_angstrom(y),
                qc::bohr_to_angstrom(z),
            );
        }
    }
    frame
}

fn read_uk_nuclear<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::read_table(lines, |line| {
        if line.contains('=') {
            return None;
        }
        let items: Vec<&str> = line.split_whitespace().collect();
        let (atomic_num, x, y, z) = qc::atom_from_columns(&items, 3, 0)?;
        Some((
            atomic_num,
            qc::bohr_to_angstrom(x),
            qc::bohr_to_angstrom(y),
            qc::bohr_to_angstrom(z),
        ))
    })
}
