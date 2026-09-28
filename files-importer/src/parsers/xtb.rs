// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(
        content,
        &["x  T  B", "x T B", "xtb version", "* xtb version"],
    ))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.to_ascii_lowercase().contains("charge") && geometry.charge == 0 {
            parse_xtb_charge(&mut geometry, line);
        }
        if line.contains("GEOMETRY OPTIMIZATION CONVERGED") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.contains("FAILED TO CONVERGE GEOMETRY") {
            geometry.is_optimization = true;
        }
        if line.contains("final structure:") {
            skip_xtb_header(&mut lines);
            geometry.push_frame(read_xtb_structure(&mut lines));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}

fn parse_xtb_charge(geometry: &mut QcGeometry, line: &str) {
    let items: Vec<&str> = line.split_whitespace().collect();
    if let Some(index) = items
        .iter()
        .position(|item| item.eq_ignore_ascii_case("charge"))
    {
        if let Some(value) = items.get(index + 1).and_then(|item| item.parse().ok()) {
            geometry.charge = value;
        }
    }
}

fn skip_xtb_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    while let Some(line) = lines.peek() {
        if line.contains("====") || line.trim().is_empty() {
            lines.next();
            continue;
        }
        break;
    }
}

fn read_xtb_structure<'a, I: Iterator<Item = &'a str>>(
    lines: &mut std::iter::Peekable<I>,
) -> CoordFrame {
    if lines
        .peek()
        .map(|line| line.trim().starts_with("$coord"))
        .unwrap_or(false)
    {
        lines.next();
        return read_coord_bohr(lines);
    }
    read_xtb_xyz(lines)
}

fn read_coord_bohr<'a, I: Iterator<Item = &'a str>>(
    lines: &mut std::iter::Peekable<I>,
) -> CoordFrame {
    let mut frame = CoordFrame::default();
    while let Some(line) = lines.peek().copied() {
        if line.trim().starts_with('$') {
            break;
        }
        lines.next();
        if let Some((atomic_num, x, y, z)) = parse_coord_bohr(line) {
            frame.push(atomic_num, x, y, z);
        }
    }
    frame
}

fn parse_coord_bohr(line: &str) -> Option<(i32, f64, f64, f64)> {
    let items: Vec<&str> = line.split_whitespace().collect();
    let (atomic_num, x, y, z) = qc::atom_from_columns(&items, 3, 0)?;
    Some((
        atomic_num,
        qc::bohr_to_angstrom(x),
        qc::bohr_to_angstrom(y),
        qc::bohr_to_angstrom(z),
    ))
}

fn read_xtb_xyz<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    let Some(count_line) = lines.next() else {
        return CoordFrame::default();
    };
    if let Ok(count) = count_line.trim().parse::<usize>() {
        lines.next();
        let mut frame = CoordFrame::default();
        for _ in 0..count {
            let Some(line) = lines.next() else { break };
            let items: Vec<&str> = line.split_whitespace().collect();
            if let Some((atomic_num, x, y, z)) = qc::atom_from_columns(&items, 0, 1) {
                frame.push(atomic_num, x, y, z);
            }
        }
        return frame;
    }
    let mut frame = CoordFrame::default();
    if let Some((atomic_num, x, y, z)) = parse_mol_or_xyz(count_line) {
        frame.push(atomic_num, x, y, z);
    }
    frame.extend(qc::read_table(lines, parse_mol_or_xyz));
    frame
}

fn parse_mol_or_xyz(line: &str) -> Option<(i32, f64, f64, f64)> {
    let items: Vec<&str> = line.split_whitespace().collect();
    if items.len() >= 4 {
        if let Some(atom) = qc::atom_from_columns(&items, 0, 1) {
            return Some(atom);
        }
        return qc::atom_from_columns(&items, 3, 0);
    }
    None
}
