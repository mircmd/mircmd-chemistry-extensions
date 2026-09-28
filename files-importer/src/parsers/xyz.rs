// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use regex::Regex;

use super::super::bindings::mircmd::file_importer::types::{Node, NodeId, Tree};
use super::super::periodic_table::get_element_by_symbol;
use super::super::types::{AtomicCoordinates, Molecule};

#[derive(PartialEq)]
enum ParserState {
    Init,
    Comment,
    Cards,
}

const MAX_VALIDATION_LINES: usize = 10;

/// Validates if the file is in XYZ format by reading only first few lines.
/// Returns true if the file appears to be a valid XYZ file, false otherwise.
pub fn test(content: &str) -> Result<bool, String> {
    let lines: Vec<&str> = content.lines().take(MAX_VALIDATION_LINES).collect();

    if lines.is_empty() {
        return Ok(false);
    }

    // First line must be a number of atoms
    let numat: usize = match lines[0].trim().parse() {
        Ok(n) => n,
        Err(_) => return Ok(false),
    };

    if numat == 0 {
        return Ok(false);
    }

    // Second line is comment, it can be anything (even empty)
    // Validate coordinate cards starting from line 3 (index 2)
    // Regex pattern from Python: ^([A-Z][a-z]?|[0-9]+)([\s]+[-+]?[0-9]*\.?[0-9]+([eE][-+]?[0-9]+)?){3}$
    let card_validator =
        Regex::new(r"^([A-Z][a-z]?|[0-9]+)([\s]+[-+]?[0-9]*\.?[0-9]+([eE][-+]?[0-9]+)?){3}$")
            .map_err(|e| format!("Failed to compile regex: {}", e))?;

    // Validate available cards (from line 3 up to numat + 2, limited by what we've read)
    let cards_to_check = std::cmp::min(numat, MAX_VALIDATION_LINES.saturating_sub(2));
    for i in 0..cards_to_check {
        let line_idx = i + 2;
        if line_idx >= lines.len() {
            break;
        }
        if !card_validator.is_match(lines[line_idx].trim()) {
            return Ok(false);
        }
    }

    Ok(true)
}

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

    let mut state = ParserState::Init;
    let mut num_atoms: usize = 0;
    let mut num_read_cards: usize = 0;
    let mut title = String::new();
    let mut atom_atomic_num: Vec<i32> = vec![];
    let mut atom_coord_x: Vec<f64> = vec![];
    let mut atom_coord_y: Vec<f64> = vec![];
    let mut atom_coord_z: Vec<f64> = vec![];

    for (line_number, line) in content.lines().enumerate() {
        match state {
            ParserState::Init => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                num_atoms = trimmed.parse::<usize>().map_err(|_| {
                    format!(
                        "Invalid line {}, expected number of atoms.",
                        line_number + 1
                    )
                })?;
                if num_atoms == 0 {
                    return Err(format!(
                        "Invalid number of atoms {} at line {}.",
                        num_atoms,
                        line_number + 1
                    ));
                }
                state = ParserState::Comment;
            }
            ParserState::Comment => {
                title = line.trim().to_string();
                if title.is_empty() {
                    title = format!("Set@line={}", line_number);
                }
                state = ParserState::Cards;
                num_read_cards = 0;
                atom_atomic_num = Vec::with_capacity(num_atoms);
                atom_coord_x = Vec::with_capacity(num_atoms);
                atom_coord_y = Vec::with_capacity(num_atoms);
                atom_coord_z = Vec::with_capacity(num_atoms);
            }
            ParserState::Cards => {
                let items: Vec<&str> = line.trim().split_whitespace().collect();
                if items.len() < 4 {
                    return Err(format!("Invalid atom card at line {}.", line_number + 1));
                }

                let atomic_num = match items[0].parse::<i32>() {
                    Ok(num) => num,
                    Err(_) => {
                        get_element_by_symbol(items[0])
                            .ok_or(format!("Invalid atom at line {}.", line_number + 1))?
                            .atomic_number
                    }
                };

                let coord_x: f64 = items[1].parse().map_err(|_| {
                    format!("Invalid coordinate value(s) at line {}.", line_number + 1)
                })?;
                let coord_y: f64 = items[2].parse().map_err(|_| {
                    format!("Invalid coordinate value(s) at line {}.", line_number + 1)
                })?;
                let coord_z: f64 = items[3].parse().map_err(|_| {
                    format!("Invalid coordinate value(s) at line {}.", line_number + 1)
                })?;

                num_read_cards += 1;
                atom_atomic_num.push(atomic_num);
                atom_coord_x.push(coord_x);
                atom_coord_y.push(coord_y);
                atom_coord_z.push(coord_z);

                if num_read_cards == num_atoms {
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

                    // Update molecule data with parsed values
                    root_node.data = postcard::to_allocvec(&Molecule {
                        n_atoms: num_atoms as i32,
                        atomic_num: atom_atomic_num.clone(),
                        charge: 0,
                        name: file_name.to_string(),
                    })
                    .map_err(|e| format!("Failed to serialize molecule: {}", e))?;

                    state = ParserState::Init;
                }
            }
        }
    }

    result.nodes.push(root_node);

    Ok(result)
}
