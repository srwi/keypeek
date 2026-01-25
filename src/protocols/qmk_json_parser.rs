use super::{Key, KeyboardDefinition, KeyboardLayout};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::num::ParseIntError;

pub fn parse_qmk_json(json_path: &str) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let file = File::open(json_path).map_err(|e| {
        Box::<dyn Error>::from(format!(
            "Failed to open keyboard info JSON '{}': {}",
            json_path, e
        ))
    })?;
    let reader = BufReader::new(file);
    let json: Value = serde_json::from_reader(reader).map_err(|e| {
        Box::<dyn Error>::from(format!("Failed to parse JSON '{}': {}", json_path, e))
    })?;

    parse_qmk_json_value(&json)
}

pub fn parse_qmk_json_value(json: &Value) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let mut layouts = Vec::new();
    let raw_layouts = json["layouts"]
        .as_object()
        .ok_or_else(|| Box::<dyn Error>::from("No layouts found in keyboard info JSON."))?;

    for layout_name in raw_layouts.keys() {
        let raw_layout = &raw_layouts[layout_name];
        let keys = collect_layout_keys(raw_layout)?;
        let layout = KeyboardLayout {
            name: layout_name.clone(),
            keys,
        };
        layouts.push(layout);
    }

    let is_split_keyboard = json
        .get("split")
        .unwrap_or(&Value::Null)
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let row_multiplier = if is_split_keyboard { 2 } else { 1 };

    let matrix_pins = json
        .get("matrix_pins")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'matrix_pins' in keyboard info JSON."))?;

    let rows = matrix_pins
        .get("rows")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'rows' in 'matrix_pins'."))?
        .as_array()
        .ok_or_else(|| Box::<dyn Error>::from("Rows in matrix_pins is not an array."))?
        .len()
        * row_multiplier;

    let cols = matrix_pins
        .get("cols")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'cols' in 'matrix_pins'."))?
        .as_array()
        .ok_or_else(|| Box::<dyn Error>::from("Cols in matrix_pins is not an array."))?
        .len();

    let usb = json
        .get("usb")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'usb' in keyboard info JSON."))?;

    let vid_str = usb
        .get("vid")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'vid' in 'usb'."))?
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("Unable to convert 'vid' to string."))?;
    let vid = hex_to_u16(vid_str)
        .map_err(|e| Box::<dyn Error>::from(format!("Invalid value for 'vid': {}", e)))?;

    let pid_str = usb
        .get("pid")
        .ok_or_else(|| Box::<dyn Error>::from("Unable to find 'pid' in 'usb'."))?
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("Unable to convert 'pid' to string."))?;
    let pid = hex_to_u16(pid_str)
        .map_err(|e| Box::<dyn Error>::from(format!("Invalid value for 'pid': {}", e)))?;

    Ok(KeyboardDefinition {
        vid,
        pid,
        rows,
        cols,
        layouts,
    })
}

fn collect_layout_keys(layout: &Value) -> Result<Vec<Key>, Box<dyn Error>> {
    let layout = layout["layout"]
        .as_array()
        .ok_or_else(|| Box::<dyn Error>::from("No layout array found."))?;

    let mut keys = Vec::new();
    for key in layout {
        let matrix_values = key["matrix"].as_array().ok_or_else(|| {
            Box::<dyn Error>::from("Unable to find 'matrix' array in key definition.")
        })?;

        let matrix_u64 = matrix_values
            .iter()
            .map(|v| {
                v.as_u64()
                    .ok_or_else(|| Box::<dyn Error>::from("Unable to parse 'matrix' value."))
            })
            .collect::<Result<Vec<u64>, Box<dyn Error>>>()?;

        let matrix: Vec<usize> = matrix_u64.into_iter().map(|n| n as usize).collect();

        let x = key["x"].as_f64().unwrap_or(0.0) as f32;
        let y = key["y"].as_f64().unwrap_or(0.0) as f32;
        let w = key["w"].as_f64().unwrap_or(1.0) as f32;
        let h = key["h"].as_f64().unwrap_or(1.0) as f32;

        keys.push(Key {
            row: matrix[0],
            col: matrix[1],
            x,
            y,
            w,
            h,
        });
    }

    Ok(keys)
}

fn hex_to_u16(hex_string: &str) -> Result<u16, ParseIntError> {
    let cleaned_hex = hex_string.trim_start_matches("0x");
    u16::from_str_radix(cleaned_hex, 16)
}
