// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    if qc::content_has_signature(content, &["TURBOMOLE"]) {
        return Ok(true);
    }
    Ok(qc::content_has_signature_n(content, 20, &["$coord"]))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut log_frames = Vec::new();
    let mut coord_frames = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("CONVERGENCE ACHIEVED") || line.contains("convergence criteria satisfied") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.starts_with("$coord") && !line.starts_with("$coordinate") {
            coord_frames.push(read_coord_block(&mut lines));
        }
        if line.contains("atomic coordinates") {
            log_frames.push(read_atomic_coordinates(&mut lines));
        }
    }

    geometry.frames = qc::prefer_frames(log_frames, coord_frames);
    geometry.frames.retain(|frame| !frame.is_empty());
    qc::to_molecule_tree(file_name, geometry)
}

fn read_coord_block<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    if lines.peek().map(|line| line.starts_with("$user")).unwrap_or(false) {
        return CoordFrame::default();
    }
    let mut frame = CoordFrame::default();
    while let Some(line) = lines.peek().copied() {
        if line.starts_with('$') {
            break;
        }
        lines.next();
        let items: Vec<&str> = line.split_whitespace().collect();
        if items.len() >= 4 {
            if let Some(atomic_num) = qc::atomic_number_from_token(items[3]) {
                if let (Ok(x), Ok(y), Ok(z)) = (items[0].parse(), items[1].parse(), items[2].parse()) {
                    frame.push(
                        atomic_num,
                        qc::bohr_to_angstrom(x),
                        qc::bohr_to_angstrom(y),
                        qc::bohr_to_angstrom(z),
                    );
                }
            }
        }
    }
    frame
}

fn read_atomic_coordinates<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        if items.len() < 4 || items.iter().all(|item| *item == ".") {
            return None;
        }
        let atomic_num = qc::atomic_number_from_token(items[3])?;
        let x: f64 = items[0].parse().ok()?;
        let y: f64 = items[1].parse().ok()?;
        let z: f64 = items[2].parse().ok()?;
        Some((
            atomic_num,
            qc::bohr_to_angstrom(x),
            qc::bohr_to_angstrom(y),
            qc::bohr_to_angstrom(z),
        ))
    })
}
