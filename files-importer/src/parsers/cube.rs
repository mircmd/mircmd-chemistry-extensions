// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use super::super::bindings::mircmd::file_importer::types::{Node, NodeId, Tree};
use super::super::types::{AtomicCoordinates, VolumeCube};

const MAX_VALIDATION_LINES: usize = 10;
const BOHR2ANGSTROM: f64 = 0.529177210903;

/// Parses a line containing grid dimensions and step vector.
/// Format: "N1 vx vy vz"
fn parse_grid_line(line: &str, line_number: usize) -> Result<(i32, Vec<f64>), String> {
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.len() < 4 {
        return Err(format!(
            "Invalid grid line at line {}, expected 4 values.",
            line_number
        ));
    }

    let n: i32 = parts[0]
        .parse()
        .map_err(|_| format!("Invalid grid count at line {}.", line_number))?;

    let vec: Vec<f64> = parts[1..4]
        .iter()
        .map(|s| {
            s.parse::<f64>()
                .map_err(|_| format!("Invalid grid vector value at line {}.", line_number))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((n, vec))
}

/// Validates if the file is in Gaussian cube format by reading only first few lines.
/// Returns true if the file appears to be a valid cube file, false otherwise.
pub fn test(content: &str) -> Result<bool, String> {
    let lines: Vec<&str> = content.lines().take(MAX_VALIDATION_LINES).collect();

    // Need at least 6 lines: 2 comments + 1 header + 3 grid lines
    if lines.len() < 6 {
        return Ok(false);
    }

    // Line 3 (index 2): number of atoms and origin coordinates
    // Format: "N_atom Ox Oy Oz [nval]"
    let header_parts: Vec<&str> = lines[2].trim().split_whitespace().collect();
    if header_parts.len() < 4 {
        return Ok(false);
    }

    // First value should be an integer (number of atoms, can be negative)
    if header_parts[0].parse::<i32>().is_err() {
        return Ok(false);
    }

    // Next 3 values should be floats (origin coordinates)
    for part in &header_parts[1..4] {
        if part.parse::<f64>().is_err() {
            return Ok(false);
        }
    }

    // Lines 4-6 (indices 3-5): grid dimensions and vectors
    // Format: "N vx vy vz"
    for i in 3..6 {
        let grid_parts: Vec<&str> = lines[i].trim().split_whitespace().collect();
        if grid_parts.len() < 4 {
            return Ok(false);
        }

        // First value should be an integer
        if grid_parts[0].parse::<i32>().is_err() {
            return Ok(false);
        }

        // Next 3 values should be floats
        for part in &grid_parts[1..4] {
            if part.parse::<f64>().is_err() {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Parses a Gaussian cube file.
///
/// Format:
/// ```text
/// Comment line 1
/// Comment line 2
/// N_atom Ox Oy Oz [nval]  # number of atoms, origin coordinates, optional values per voxel
/// N1 vx1 vy1 vz1          # grid dimensions and step vectors
/// N2 vx2 vy2 vz2
/// N3 vx3 vy3 vz3
/// Atom1 Z1 x y z          # Atomic number, charge, and coordinates (in Bohr)
/// ...
/// AtomN ZN x y z
/// [DSET_IDS]              # Data set identifiers if N_atom is negative
/// Data on grids           # Volumetric data
/// ```
///
/// References:
/// - http://paulbourke.net/dataformats/cube/
/// - https://h5cube-spec.readthedocs.io/en/latest/cubeformat.html
/// - http://gaussian.com/cubegen/
pub fn parse(content: &str, file_name: &str) -> Result<Tree, String> {
    let mut lines = content.lines().enumerate();

    // Line 1: Comment 1
    let (_, comment_1) = lines
        .next()
        .ok_or_else(|| "File is empty, expected comment line 1.".to_string())?;
    let comment_1 = comment_1.trim().to_string();

    // Line 2: Comment 2
    let (_, comment_2) = lines
        .next()
        .ok_or_else(|| "Unexpected end of file, expected comment line 2.".to_string())?;
    let comment_2 = comment_2.trim().to_string();

    // Line 3: Number of atoms and origin coordinates
    let (line_number, header_line) = lines
        .next()
        .ok_or_else(|| "Unexpected end of file, expected header line.".to_string())?;
    let header_parts: Vec<&str> = header_line.trim().split_whitespace().collect();

    if header_parts.len() < 4 {
        return Err(format!(
            "Invalid header at line {}, expected at least 4 values.",
            line_number + 1
        ));
    }

    let natm_raw: i32 = header_parts[0]
        .parse()
        .map_err(|_| format!("Invalid number of atoms at line {}.", line_number + 1))?;

    // Check for multiple values per voxel (unsupported)
    if natm_raw > 0 && header_parts.len() > 4 {
        let nval: i32 = header_parts[4].parse().unwrap_or(1);
        if nval > 1 {
            return Err(format!(
                "Unsupported number of data values per voxel {} in cube file.",
                nval
            ));
        }
    }

    let dset_ids = natm_raw < 0;
    let natm = natm_raw.unsigned_abs() as usize;

    let box_origin: Vec<f64> = header_parts[1..4]
        .iter()
        .map(|s| {
            s.parse::<f64>()
                .map_err(|_| format!("Invalid origin coordinate at line {}.", line_number + 1))
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Lines 4-6: Grid dimensions and step vectors
    let mut steps_number: Vec<i32> = Vec::with_capacity(3);
    let mut steps_size: Vec<Vec<f64>> = Vec::with_capacity(3);

    for _ in 0..3 {
        let (line_number, grid_line) = lines
            .next()
            .ok_or_else(|| "Unexpected end of file, expected grid line.".to_string())?;
        let (n, vec) = parse_grid_line(grid_line, line_number + 1)?;
        steps_number.push(n);
        steps_size.push(vec);
    }

    // Read atom data
    let mut atom_atomic_num: Vec<i32> = Vec::with_capacity(natm);
    let mut atom_coord_x: Vec<f64> = Vec::with_capacity(natm);
    let mut atom_coord_y: Vec<f64> = Vec::with_capacity(natm);
    let mut atom_coord_z: Vec<f64> = Vec::with_capacity(natm);

    for _ in 0..natm {
        let (line_number, atom_line) = lines
            .next()
            .ok_or_else(|| "Unexpected end of file, expected atom data.".to_string())?;
        let parts: Vec<&str> = atom_line.trim().split_whitespace().collect();

        if parts.len() < 5 {
            return Err(format!(
                "Invalid atom data at line {}, expected 5 values.",
                line_number + 1
            ));
        }

        let atomic_num: i32 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid atomic number at line {}.", line_number + 1))?;

        // parts[1] is charge (skipped in output)
        let x: f64 = parts[2]
            .parse::<f64>()
            .map_err(|_| format!("Invalid x coordinate at line {}.", line_number + 1))?
            * BOHR2ANGSTROM;
        let y: f64 = parts[3]
            .parse::<f64>()
            .map_err(|_| format!("Invalid y coordinate at line {}.", line_number + 1))?
            * BOHR2ANGSTROM;
        let z: f64 = parts[4]
            .parse::<f64>()
            .map_err(|_| format!("Invalid z coordinate at line {}.", line_number + 1))?
            * BOHR2ANGSTROM;

        atom_atomic_num.push(atomic_num);
        atom_coord_x.push(x);
        atom_coord_y.push(y);
        atom_coord_z.push(z);
    }

    // Handle DSET_IDS line if present
    if dset_ids {
        let (line_number, dset_line) = lines
            .next()
            .ok_or_else(|| "Unexpected end of file, expected DSET_IDS line.".to_string())?;
        let parts: Vec<&str> = dset_line.trim().split_whitespace().collect();

        if !parts.is_empty() {
            let num_ids: i32 = parts[0].parse().unwrap_or(1);
            if num_ids != 1 {
                return Err(format!(
                    "Unsupported number of identifiers per voxel {} at line {}.",
                    num_ids,
                    line_number + 1
                ));
            }
        }
    }

    // Read volumetric data
    let total_points =
        (steps_number[0] as usize) * (steps_number[1] as usize) * (steps_number[2] as usize);
    let mut cube_data_flat: Vec<f64> = Vec::with_capacity(total_points);

    // Collect remaining lines and parse all values
    for (line_number, data_line) in lines {
        for value_str in data_line.trim().split_whitespace() {
            let value: f64 = value_str.parse().map_err(|_| {
                format!("Invalid volumetric data value at line {}.", line_number + 1)
            })?;
            cube_data_flat.push(value);
        }
    }

    if cube_data_flat.len() != total_points {
        return Err(format!(
            "Mismatch in volumetric data: expected {} points, found {}.",
            total_points,
            cube_data_flat.len()
        ));
    }

    // Reshape flat data into 3D array [n1][n2][n3]
    let n1 = steps_number[0] as usize;
    let n2 = steps_number[1] as usize;
    let n3 = steps_number[2] as usize;

    let mut cube_data: Vec<Vec<Vec<f64>>> = Vec::with_capacity(n1);
    let mut idx = 0;

    for _ in 0..n1 {
        let mut plane: Vec<Vec<f64>> = Vec::with_capacity(n2);
        for _ in 0..n2 {
            let row: Vec<f64> = cube_data_flat[idx..idx + n3].to_vec();
            plane.push(row);
            idx += n3;
        }
        cube_data.push(plane);
    }

    // Create VolumeCube data
    let volume_cube = VolumeCube {
        comment1: comment_1,
        comment2: comment_2,
        box_origin,
        steps_number: (n1, n2, n3),
        steps_size,
        cube_data,
    };

    // Create atomic coordinates node
    let coords = AtomicCoordinates {
        atomic_num: atom_atomic_num,
        x: atom_coord_x,
        y: atom_coord_y,
        z: atom_coord_z,
    };

    let mut node_id: NodeId = 0;
    let mut result = Tree {
        root: 0,
        nodes: vec![],
    };

    let mut root_node = Node {
        id: node_id,
        name: file_name.to_string(),
        type_id: "mircmd:chemistry:volume_cube".to_string(),
        data: postcard::to_allocvec(&volume_cube)
            .map_err(|e| format!("Failed to serialize volume cube: {}", e))?,
        children: vec![],
    };

    node_id += 1;
    let at_coord_node = Node {
        id: node_id,
        name: "CubeMol".to_string(),
        type_id: "mircmd:chemistry:atomic_coordinates".to_string(),
        data: postcard::to_allocvec(&coords)
            .map_err(|e| format!("Failed to serialize coordinates: {}", e))?,
        children: vec![],
    };

    result.nodes.push(at_coord_node);
    root_node.children.push(node_id);
    result.nodes.push(root_node);

    Ok(result)
}
