// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::{Node, NodeId, Tree};
use super::super::types::{AtomicCoordinates, Molecule};

const MAX_VALIDATION_LINES: usize = 20;
const BOHR2ANGSTROM: f64 = 0.529177210903;

const CFOUR_SIGNATURE: &str = "<<<     CCCCCC     CCCCCC   |||     CCCCCC     CCCCCC   >>>";

/// Validates if the file is in Cfour log format.
pub fn test(content: &str) -> Result<bool, String> {
    Ok(content
        .lines()
        .take(MAX_VALIDATION_LINES)
        .skip(1)
        .any(|line| line.contains(CFOUR_SIGNATURE)))
}

/// Parses a Cfour log file.
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

    let mut cart_set_number = 0;
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Z-matrix   Atomic            Coordinates (in bohr)") {
            cart_set_number += 1;

            // Skip header of the table (2 lines)
            for _ in 0..2 {
                lines.next();
            }

            // Read the table
            let mut atomic_num: Vec<i32> = vec![];
            let mut atom_coord_x: Vec<f64> = vec![];
            let mut atom_coord_y: Vec<f64> = vec![];
            let mut atom_coord_z: Vec<f64> = vec![];

            for block_line in lines.by_ref() {
                if block_line.contains("--") {
                    break;
                }

                let items: Vec<&str> = block_line.split_whitespace().collect();
                if items.len() >= 5 {
                    let at_num = if items[1] == "0" {
                        -1
                    } else {
                        items[1].parse::<i32>().unwrap_or(-1)
                    };

                    let x: f64 = items[2].parse::<f64>().unwrap_or(0.0) * BOHR2ANGSTROM;
                    let y: f64 = items[3].parse::<f64>().unwrap_or(0.0) * BOHR2ANGSTROM;
                    let z: f64 = items[4].parse::<f64>().unwrap_or(0.0) * BOHR2ANGSTROM;

                    atomic_num.push(at_num);
                    atom_coord_x.push(x);
                    atom_coord_y.push(y);
                    atom_coord_z.push(z);
                }
            }

            let coords = AtomicCoordinates {
                atomic_num,
                x: atom_coord_x,
                y: atom_coord_y,
                z: atom_coord_z,
            };

            node_id += 1;
            let at_coord_node = Node {
                id: node_id,
                name: format!("Set#{}", cart_set_number),
                type_id: "mircmd:chemistry:atomic_coordinates".to_string(),
                data: postcard::to_allocvec(&coords)
                    .map_err(|e| format!("Failed to serialize coordinates: {}", e))?,
                children: vec![],
            };

            result.nodes.push(at_coord_node);
            root_node.children.push(node_id);
        }
    }

    result.nodes.push(root_node);
    Ok(result)
}
