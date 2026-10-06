use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Deserialize, Serialize)]
struct AtomicCoordinates {
    atomic_num: Vec<i32>,
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    title: String,
    precision: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            title: String::new(),
            precision: 14,
        }
    }
}

const SYMBOLS: &[&str] = &[
    "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
    "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge",
    "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd",
    "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm",
    "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn",
    "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
];

fn symbol(number: i32) -> Result<&'static str, String> {
    match number {
        -1 => Ok("X"),
        -2 => Ok("Q"),
        1..=118 => Ok(SYMBOLS[number as usize]),
        _ => Err(format!("Invalid atomic number: {number}")),
    }
}

fn validate(coordinates: &AtomicCoordinates, settings: &Settings) -> Result<(), String> {
    let count = coordinates.atomic_num.len();
    if [
        coordinates.x.len(),
        coordinates.y.len(),
        coordinates.z.len(),
    ] != [count; 3]
    {
        return Err("Atomic numbers and coordinate arrays must have the same length".into());
    }
    if settings.precision > 16 || settings.title.contains(['\r', '\n']) {
        return Err(
            "Precision must be between 0 and 16 and the title must be a single line".into(),
        );
    }
    for number in &coordinates.atomic_num {
        symbol(*number)?;
    }
    if coordinates
        .x
        .iter()
        .chain(&coordinates.y)
        .chain(&coordinates.z)
        .any(|v| !v.is_finite())
    {
        return Err("Coordinates must be finite numbers".into());
    }
    Ok(())
}

pub fn render(data: &[u8], settings: &str) -> Result<String, String> {
    let (coordinates, remainder): (AtomicCoordinates, _) = postcard::take_from_bytes(data)
        .map_err(|error| format!("Invalid atomic coordinates: {error}"))?;
    if !remainder.is_empty() {
        return Err("Unexpected trailing bytes in atomic coordinates".into());
    }
    let settings: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(settings).map_err(|error| format!("Invalid XYZ settings: {error}"))?;
    let settings: Settings = serde_json::from_value(serde_json::Value::Object(settings))
        .map_err(|error| format!("Invalid XYZ settings: {error}"))?;
    validate(&coordinates, &settings)?;
    let mut output = format!("{}\n{}\n", coordinates.atomic_num.len(), settings.title);
    for (i, number) in coordinates.atomic_num.iter().enumerate() {
        writeln!(
            output,
            "{:<6} {:>20.*} {:>20.*} {:>20.*}",
            symbol(*number)?,
            settings.precision,
            coordinates.x[i],
            settings.precision,
            coordinates.y[i],
            settings.precision,
            coordinates.z[i]
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom() -> AtomicCoordinates {
        AtomicCoordinates {
            atomic_num: vec![8],
            x: vec![1.25],
            y: vec![-2.5],
            z: vec![0.0],
        }
    }

    #[test]
    fn exports_postcard_with_settings() {
        let bytes = postcard::to_allocvec(&atom()).unwrap();
        let output = render(&bytes, r#"{"title":"Вода", "precision":3}"#).unwrap();
        assert_eq!(output.lines().take(2).collect::<Vec<_>>(), ["1", "Вода"]);
        assert_eq!(
            output
                .lines()
                .nth(2)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>(),
            ["O", "1.250", "-2.500", "0.000"]
        );
    }

    #[test]
    fn rejects_corrupt_objects() {
        let mut coordinates = atom();
        coordinates.z.clear();
        assert!(render(&postcard::to_allocvec(&coordinates).unwrap(), "{}").is_err());
        coordinates = atom();
        coordinates.x[0] = f64::NAN;
        assert!(render(&postcard::to_allocvec(&coordinates).unwrap(), "{}").is_err());
        coordinates = atom();
        coordinates.atomic_num[0] = 119;
        assert!(render(&postcard::to_allocvec(&coordinates).unwrap(), "{}").is_err());
        assert!(render(b"broken", "{}").is_err());
    }

    #[test]
    fn rejects_invalid_settings_and_trailing_bytes() {
        let mut bytes = postcard::to_allocvec(&atom()).unwrap();
        for settings in [
            r#"{"precision":17}"#,
            r#"{"title":"a\nb"}"#,
            r#"{"extra":true}"#,
            "[]",
        ] {
            assert!(render(&bytes, settings).is_err());
        }
        bytes.push(0);
        assert!(render(&bytes, "{}").is_err());
    }

    #[test]
    fn supports_defaults_empty_objects_and_dummy_atoms() {
        assert_eq!(render(&[0, 0, 0, 0], "{}").unwrap(), "0\n\n");
        for number in [-2, -1, 1, 118] {
            let mut coordinates = atom();
            coordinates.atomic_num[0] = number;
            assert!(render(&postcard::to_allocvec(&coordinates).unwrap(), "{}").is_ok());
        }
    }
}
