// Copyright (c) 2026 Valery Vishnevskiy and Yury Vishnevskiy
// Licensed under the Apache 2.0 License

use serde::Serialize;

#[derive(Serialize)]
pub struct AtomicCoordinates {
    pub atomic_num: Vec<i32>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub z: Vec<f64>,
}

#[derive(Serialize)]
pub struct Molecule {
    pub n_atoms: i32,
    pub atomic_num: Vec<i32>,
    pub charge: i32,
    pub name: String,
}

#[derive(Serialize)]
pub struct VolumeCube {
    pub comment1: String,
    pub comment2: String,
    pub box_origin: Vec<f64>,
    pub steps_number: (usize, usize, usize),
    pub steps_size: Vec<Vec<f64>>,
    pub cube_data: Vec<Vec<Vec<f64>>>,
}
