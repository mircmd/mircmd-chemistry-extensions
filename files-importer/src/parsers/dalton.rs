// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::Tree;
use super::qc::{self, CoordFrame, QcGeometry};

pub fn test(content: &str) -> Result<bool, String> {
    Ok(qc::content_has_signature(
        content,
        &["Dalton - An Electronic Structure Program"],
    ))
}

pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut geometry = QcGeometry::default();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Optimization completed") || line.contains("*** Optimization converged") {
            geometry.is_optimization = true;
            geometry.optimized = true;
        }
        if line.trim() == "Cartesian Coordinates (a.u.)" {
            skip_dalton_header(&mut lines);
            geometry.push_frame(read_dalton_cartesian(&mut lines));
        }
    }

    qc::to_molecule_tree(file_name, geometry)
}

fn skip_dalton_header<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) {
    while let Some(line) = lines.peek() {
        if line.contains("Total number of coordinates") {
            lines.next();
            if lines.peek().map(|next| next.trim().is_empty()).unwrap_or(false) {
                lines.next();
            }
            break;
        }
        lines.next();
    }
}

fn read_dalton_cartesian<'a, I: Iterator<Item = &'a str>>(lines: &mut std::iter::Peekable<I>) -> CoordFrame {
    qc::read_table(lines, |line| {
        let items: Vec<&str> = line.split_whitespace().collect();
        let numbers: Vec<f64> = items.iter().filter_map(|item| item.parse().ok()).collect();
        if numbers.len() < 3 {
            return None;
        }
        let atomic_num = items.iter().find_map(|item| {
            let symbol = qc::normalize_element_symbol(item);
            if symbol.is_empty() {
                None
            } else {
                qc::atomic_number_from_token(item)
            }
        })?;
        let len = numbers.len();
        Some((
            atomic_num,
            qc::bohr_to_angstrom(numbers[len - 3]),
            qc::bohr_to_angstrom(numbers[len - 2]),
            qc::bohr_to_angstrom(numbers[len - 1]),
        ))
    })
}
