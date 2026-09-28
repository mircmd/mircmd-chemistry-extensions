// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    let has_data = qc::content_has_signature_n(content, 40, &["$DATA", "$data"]);
    let has_log = qc::content_has_signature(content, &["GAMESS VERSION", "Firefly (PC GAMESS)"]);
    Ok(has_data && !has_log)
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut data_frames = Vec::new();
    let mut centre_frames = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.to_ascii_uppercase().contains("$DATA") {
            data_frames.push(read_data_block(&mut lines));
        }
        if line.contains("(CENTRE") {
            if let Some(atom) = parse_centre_line(line) {
                append_centre(&mut centre_frames, atom);
            }
        }
    }

    geometry.frames = qc::prefer_frames(centre_frames, data_frames);
    geometry.frames.retain(|frame| !frame.is_empty());
    qc::to_molecule_tree(file_name, geometry)
}

fn read_data_block<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::skip(lines, 2);
    let mut frame = CoordFrame::default();
    while let Some(line) = lines.next() {
        if line.to_ascii_uppercase().contains("$END") {
            break;
        }
        let items: Vec<&str> = line.split_whitespace().collect();
        if items.len() >= 5 {
            if let Some((atomic_num, x, y, z)) = qc::atom_from_columns(&items, 0, 2) {
                frame.push(atomic_num, x, y, z);
            }
        }
    }
    frame
}

fn parse_centre_line(line: &str) -> Option<(i32, f64, f64, f64)> {
    let items: Vec<&str> = line.split_whitespace().collect();
    let (atomic_num, x, y, z) = qc::atom_from_columns(&items, 0, 4)?;
    Some((
        atomic_num,
        qc::bohr_to_angstrom(x),
        qc::bohr_to_angstrom(y),
        qc::bohr_to_angstrom(z),
    ))
}

fn append_centre(frames: &mut Vec<CoordFrame>, atom: (i32, f64, f64, f64)) {
    if frames.is_empty() {
        frames.push(CoordFrame::default());
    }
    frames[0].push(atom.0, atom.1, atom.2, atom.3);
}
