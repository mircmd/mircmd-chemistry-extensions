// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use regex::Regex;

use super::super::bindings::mircmd::file_importer::types::{Node, NodeId, Tree};
use super::super::periodic_table::get_element_by_symbol;
use super::super::types::AtomicCoordinates;

#[derive(PartialEq)]
enum Unex2XyzFormat {
    Invalid,
    Unex,
    Mol,
}

/// Returns UNEX version number encoded in a single integer number.
fn get_format_version(line: &str) -> Option<i32> {
    let version_regex = Regex::new(r"^([0-9]+)\.([0-9]+)-([0-9]+)-([a-z0-9]+)$").ok()?;

    if line.trim().starts_with("UNEX") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Some(caps) = version_regex.captures(parts[1]) {
                let major: i32 = caps.get(1)?.as_str().parse().ok()?;
                let minor: i32 = caps.get(2)?.as_str().parse().ok()?;
                let patch: i32 = caps.get(3)?.as_str().parse().ok()?;
                return Some(1_000_000 * major + 10_000 * minor + patch);
            }
        }
    }
    None
}

/// Validates if the file is in UNEX format.
pub fn test(content: &str) -> Result<bool, String> {
    let Some(first) = content.lines().next() else {
        return Ok(false);
    };
    Ok(get_format_version(first).is_some())
}

/// Parses UNEX 1.x format.
fn parse_unex1x(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut node_id: NodeId = 0;
    let mut result = Tree {
        root: 0,
        nodes: vec![],
    };

    let mut root_node = Node {
        id: node_id,
        name: file_name.to_string(),
        type_id: "mircmd:chemistry:unex".to_string(),
        data: vec![],
        children: vec![],
    };

    let mut molecules: std::collections::HashMap<String, NodeId> = std::collections::HashMap::new();
    let mut molecule_nodes: std::collections::HashMap<NodeId, Node> =
        std::collections::HashMap::new();
    let mut mol_cart_set_number: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("> Cartesian coordinates of all atoms (Angstroms) in") {
            let molecule_name = line.split('>').next().unwrap_or("").trim().to_string();

            let mol_id = if let Some(&id) = molecules.get(&molecule_name) {
                id
            } else {
                node_id += 1;
                let mol_node = Node {
                    id: node_id,
                    name: molecule_name.clone(),
                    type_id: "mircmd:chemistry:molecule".to_string(),
                    data: vec![],
                    children: vec![],
                };
                molecules.insert(molecule_name.clone(), node_id);
                molecule_nodes.insert(node_id, mol_node);
                root_node.children.push(node_id);
                node_id
            };

            let set_num = mol_cart_set_number
                .entry(molecule_name.clone())
                .or_insert(0);
            *set_num += 1;

            // Skip header of the table (3 lines)
            for _ in 0..3 {
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
                if items.len() >= 7 {
                    if let (Ok(num), Ok(x), Ok(y), Ok(z)) = (
                        items[2].parse::<i32>(),
                        items[4].parse::<f64>(),
                        items[5].parse::<f64>(),
                        items[6].parse::<f64>(),
                    ) {
                        atomic_num.push(num);
                        atom_coord_x.push(x);
                        atom_coord_y.push(y);
                        atom_coord_z.push(z);
                    }
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
                name: format!("Set#{}", set_num),
                type_id: "mircmd:chemistry:atomic_coordinates".to_string(),
                data: postcard::to_allocvec(&coords)
                    .map_err(|e| format!("Failed to serialize coordinates: {}", e))?,
                children: vec![],
            };

            result.nodes.push(at_coord_node);
            molecule_nodes
                .get_mut(&mol_id)
                .ok_or_else(|| "Missing molecule node".to_string())?
                .children
                .push(node_id);
        }
    }

    for &id in &root_node.children {
        let mol_node = molecule_nodes
            .remove(&id)
            .ok_or_else(|| "Missing molecule node".to_string())?;
        result.nodes.push(mol_node);
    }
    result.nodes.push(root_node);

    Ok(result)
}

/// Parses UNEX 2.x format.
fn parse_unex2x(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut node_id: NodeId = 0;
    let mut result = Tree {
        root: 0,
        nodes: vec![],
    };

    let mut root_node = Node {
        id: node_id,
        name: file_name.to_string(),
        type_id: "mircmd:chemistry:unex".to_string(),
        data: vec![],
        children: vec![],
    };

    let mut molecules: std::collections::HashMap<String, NodeId> = std::collections::HashMap::new();
    let mut molecule_nodes: std::collections::HashMap<NodeId, Node> =
        std::collections::HashMap::new();
    let mut mol_cart_set_number: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        if line.contains("Cartesian coordinates (Angstroms) of atoms in") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let molecule_name = if parts.len() > 6 {
                parts[6].trim().to_string()
            } else {
                "unknown".to_string()
            };

            let mol_id = if let Some(&id) = molecules.get(&molecule_name) {
                id
            } else {
                node_id += 1;
                let mol_node = Node {
                    id: node_id,
                    name: molecule_name.clone(),
                    type_id: "mircmd:chemistry:molecule".to_string(),
                    data: vec![],
                    children: vec![],
                };
                molecules.insert(molecule_name.clone(), node_id);
                molecule_nodes.insert(node_id, mol_node);
                root_node.children.push(node_id);
                node_id
            };

            let set_num = mol_cart_set_number
                .entry(molecule_name.clone())
                .or_insert(0);
            *set_num += 1;

            let mut xyz_format = Unex2XyzFormat::Invalid;
            let mut delimiter_number = 0;

            // Read header to determine format
            for header_line in lines.by_ref() {
                if header_line.contains("Format:") {
                    let format_parts: Vec<&str> = header_line.split_whitespace().collect();
                    if format_parts.len() >= 2 {
                        match format_parts[1].trim() {
                            "UNEX" => xyz_format = Unex2XyzFormat::Unex,
                            "MOL" => xyz_format = Unex2XyzFormat::Mol,
                            _ => {
                                return Err(format!(
                                    "Invalid or unknown XYZ format {}",
                                    format_parts[1]
                                ));
                            }
                        }
                    }
                } else if header_line.contains("--") {
                    if xyz_format == Unex2XyzFormat::Unex {
                        delimiter_number += 1;
                        if delimiter_number == 2 {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }

            if xyz_format == Unex2XyzFormat::Mol {
                // Skip header of the MOL format (2 lines)
                for _ in 0..2 {
                    lines.next();
                }
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

                match xyz_format {
                    Unex2XyzFormat::Unex => {
                        if items.len() >= 7 {
                            if let (Ok(num), Ok(x), Ok(y), Ok(z)) = (
                                items[2].parse::<i32>(),
                                items[4].parse::<f64>(),
                                items[5].parse::<f64>(),
                                items[6].parse::<f64>(),
                            ) {
                                atomic_num.push(num);
                                atom_coord_x.push(x);
                                atom_coord_y.push(y);
                                atom_coord_z.push(z);
                            }
                        }
                    }
                    Unex2XyzFormat::Mol => {
                        if items.len() >= 4 {
                            let at_num = get_element_by_symbol(items[0])
                                .ok_or(format!("Invalid atom symbol {}", items[0]))?
                                .atomic_number;

                            let x: f64 = items[1]
                                .parse()
                                .map_err(|_| "Invalid x coordinate".to_string())?;
                            let y: f64 = items[2]
                                .parse()
                                .map_err(|_| "Invalid y coordinate".to_string())?;
                            let z: f64 = items[3]
                                .parse()
                                .map_err(|_| "Invalid z coordinate".to_string())?;

                            atomic_num.push(at_num);
                            atom_coord_x.push(x);
                            atom_coord_y.push(y);
                            atom_coord_z.push(z);
                        }
                    }
                    Unex2XyzFormat::Invalid => {}
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
                name: format!("Set#{}", set_num),
                type_id: "mircmd:chemistry:atomic_coordinates".to_string(),
                data: postcard::to_allocvec(&coords)
                    .map_err(|e| format!("Failed to serialize coordinates: {}", e))?,
                children: vec![],
            };

            result.nodes.push(at_coord_node);
            molecule_nodes
                .get_mut(&mol_id)
                .ok_or_else(|| "Missing molecule node".to_string())?
                .children
                .push(node_id);
        }
    }

    for &id in &root_node.children {
        let mol_node = molecule_nodes
            .remove(&id)
            .ok_or_else(|| "Missing molecule node".to_string())?;
        result.nodes.push(mol_node);
    }
    result.nodes.push(root_node);

    Ok(result)
}

/// Parses a UNEX file.
pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let first_line = content.lines().next().unwrap_or("");

    let version =
        get_format_version(first_line).ok_or_else(|| "Invalid UNEX file format.".to_string())?;

    // UNEX 1.x
    if version < 2_000_000 {
        parse_unex1x(content, file_name)
    } else {
        // UNEX >= 2.x
        parse_unex2x(content, file_name)
    }
}
