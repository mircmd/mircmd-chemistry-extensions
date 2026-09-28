// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::{Node, NodeId, Tree};
use super::super::periodic_table::get_element_by_symbol;
use super::super::types::{AtomicCoordinates, Molecule};

const MAX_VALIDATION_LINES: usize = 4;

#[derive(PartialEq)]
enum ParserState {
    Init,
    Control,
    Atom,
}

/// Validates if the file is in MDL Mol V2000 format.
pub fn test(content: &str) -> Result<bool, String> {
    let lines: Vec<&str> = content.lines().take(MAX_VALIDATION_LINES).collect();
    if lines.len() < 4 {
        return Ok(false);
    }
    Ok(lines[3].contains(" V2000"))
}

/// Parses a MDL Mol V2000 file.
pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut node_id: NodeId = 0;
    let mut result = Tree {
        root: 0,
        nodes: vec![],
    };

    let mut root_node = Node {
        id: node_id,
        name: file_name.to_string(),
        type_id: "mircmd:chemistry:molecule".to_string(),
        data: postcard::to_allocvec(&Molecule {
            n_atoms: 0,
            atomic_num: vec![],
            charge: 0,
            name: file_name.to_string(),
        })
        .map_err(|e| format!("Failed to serialize molecule: {}", e))?,
        children: vec![],
    };

    let mut title = String::new();
    let mut state = ParserState::Init;
    let mut num_atoms: usize = 0;
    let mut num_read_at_cards: usize = 0;
    let mut atom_atomic_num: Vec<i32> = vec![];
    let mut atom_coord_x: Vec<f64> = vec![];
    let mut atom_coord_y: Vec<f64> = vec![];
    let mut atom_coord_z: Vec<f64> = vec![];

    for (line_number, line) in content.lines().enumerate() {
        match state {
            ParserState::Init => {
                if title.is_empty() {
                    title = line.trim().to_string();
                }
                if line_number == 2 {
                    state = ParserState::Control;
                }
            }
            ParserState::Control => {
                let items: Vec<&str> = line.trim().split_whitespace().collect();

                if items.is_empty() {
                    return Err(format!(
                        "Invalid control line {}, expected number of atoms.",
                        line_number + 1
                    ));
                }

                num_atoms = items[0]
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid control line {}, expected number of atoms.", line_number + 1))?;

                if items.len() < 2 {
                    return Err(format!(
                        "Invalid control line {}, expected number of bonds.",
                        line_number + 1
                    ));
                }

                let num_bonds: i32 = items[1]
                    .parse()
                    .map_err(|_| format!("Invalid control line {}, expected number of bonds.", line_number + 1))?;

                if num_atoms == 0 {
                    return Err(format!(
                        "Invalid number of atoms {} defined in line {}.",
                        num_atoms,
                        line_number + 1
                    ));
                }

                if num_bonds < 0 {
                    return Err(format!(
                        "Invalid number of bonds {} defined in line {}.",
                        num_bonds,
                        line_number + 1
                    ));
                }

                num_read_at_cards = 0;
                atom_atomic_num = Vec::with_capacity(num_atoms);
                atom_coord_x = Vec::with_capacity(num_atoms);
                atom_coord_y = Vec::with_capacity(num_atoms);
                atom_coord_z = Vec::with_capacity(num_atoms);
                state = ParserState::Atom;
            }
            ParserState::Atom => {
                let items: Vec<&str> = line.trim().split_whitespace().collect();

                if items.len() < 4 {
                    return Err(format!("Invalid atom coordinate value(s) at line {}.", line_number + 1));
                }

                let atomic_num = get_element_by_symbol(items[3])
                    .ok_or(format!("Invalid atom symbol at line {}.", line_number + 1))?
                    .atomic_number;
                let coord_x: f64 = items[0]
                    .parse()
                    .map_err(|_| format!("Invalid atom coordinate value(s) at line {}.", line_number + 1))?;
                let coord_y: f64 = items[1]
                    .parse()
                    .map_err(|_| format!("Invalid atom coordinate value(s) at line {}.", line_number + 1))?;
                let coord_z: f64 = items[2]
                    .parse()
                    .map_err(|_| format!("Invalid atom coordinate value(s) at line {}.", line_number + 1))?;

                num_read_at_cards += 1;
                atom_atomic_num.push(atomic_num);
                atom_coord_x.push(coord_x);
                atom_coord_y.push(coord_y);
                atom_coord_z.push(coord_z);

                if num_read_at_cards == num_atoms {
                    if title.is_empty() {
                        title = file_name.to_string();
                    }

                    let coords = AtomicCoordinates {
                        atomic_num: atom_atomic_num.clone(),
                        x: atom_coord_x.clone(),
                        y: atom_coord_y.clone(),
                        z: atom_coord_z.clone(),
                    };

                    node_id += 1;
                    let at_coord_node = Node {
                        id: node_id,
                        name: title.clone(),
                        type_id: "mircmd:chemistry:atomic_coordinates".to_string(),
                        data: postcard::to_allocvec(&coords)
                            .map_err(|e| format!("Failed to serialize coordinates: {}", e))?,
                        children: vec![],
                    };

                    result.nodes.push(at_coord_node);
                    root_node.children.push(node_id);
                    break; // Stop after reading atoms (skip bonds section)
                }
            }
        }
    }

    result.nodes.push(root_node);
    Ok(result)
}
