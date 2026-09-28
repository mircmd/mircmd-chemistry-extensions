// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use std::iter::Peekable;

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(content, &["Gaussian, Inc."]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut standard = Vec::new();
    let mut input = Vec::new();
    let mut zmatrix = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        update_flags(&mut geometry, line);
        parse_charge(&mut geometry, line);
        if line.contains("Standard orientation:") {
            standard.push(read_orientation(&mut lines));
        } else if line.contains("Input orientation:") {
            input.push(read_orientation(&mut lines));
        } else if line.contains("Z-Matrix orientation:") {
            zmatrix.push(read_orientation(&mut lines));
        }
    }

    geometry.frames = qc::prefer_frames(standard, qc::prefer_frames(input, zmatrix));
    geometry.frames.retain(|frame| !frame.is_empty());
    qc::to_molecule_tree(file_name, geometry)
}

fn update_flags(geometry: &mut QcGeometry, line: &str) {
    if line.contains("Berny optimization") || line.contains("GradGradGrad") {
        geometry.is_optimization = true;
    }
    if line.contains("Optimization completed") || line.contains("Stationary point found") {
        geometry.is_optimization = true;
        geometry.optimized = true;
    }
    if line.contains("Optimization aborted") || line.contains("Optimization stopped") {
        geometry.is_optimization = true;
    }
}

fn parse_charge(geometry: &mut QcGeometry, line: &str) {
    if !(line.contains("Charge =") && line.contains("Multiplicity")) {
        return;
    }
    let items: Vec<&str> = line.split_whitespace().collect();
    if let Some(index) = items.iter().position(|item| *item == "Charge") {
        if let Some(value) = items.get(index + 2).and_then(|item| item.parse().ok()) {
            geometry.charge = value;
        }
    }
}

fn read_orientation<'a, I: Iterator<Item = &'a str>>(lines: &mut Peekable<I>) -> CoordFrame {
    qc::skip(lines, 4);
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        qc::atom_from_columns(&items, 1, 3)
    })
}
