use super::{Key, KeyboardDefinition, KeyboardLayout};
use serde_json::Value;
use std::error::Error;

pub fn parse_vial_definition(
    json: &Value,
    vid: u16,
    pid: u16,
) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let matrix = json
        .get("matrix")
        .ok_or_else(|| Box::<dyn Error>::from("No 'matrix' found in VIAL definition"))?;

    let rows = matrix
        .get("rows")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Box::<dyn Error>::from("No 'rows' in matrix"))? as usize;

    let cols = matrix
        .get("cols")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Box::<dyn Error>::from("No 'cols' in matrix"))? as usize;

    let layouts_obj = json
        .get("layouts")
        .ok_or_else(|| Box::<dyn Error>::from("No 'layouts' found in VIAL definition"))?;

    let keymap = layouts_obj
        .get("keymap")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Box::<dyn Error>::from("No 'keymap' array in layouts"))?;

    let keys = parse_kle_keymap(keymap)?;

    let layout = KeyboardLayout {
        name: "default".to_string(),
        keys,
    };

    Ok(KeyboardDefinition {
        vid,
        pid,
        rows,
        cols,
        layouts: vec![layout],
    })
}

fn parse_kle_keymap(keymap: &[Value]) -> Result<Vec<Key>, Box<dyn Error>> {
    let mut keys = Vec::new();
    let mut current_y: f32 = 0.0;

    for row in keymap {
        let row_array = match row.as_array() {
            Some(arr) => arr,
            None => continue, // Skip non-array rows
        };

        let mut current_x: f32 = 0.0;
        let mut current_w: f32 = 1.0;
        let mut current_h: f32 = 1.0;

        for item in row_array {
            if let Some(obj) = item.as_object() {
                // This is a key property object - modifies the next key
                if let Some(w) = obj.get("w").and_then(|v| v.as_f64()) {
                    current_w = w as f32;
                }
                if let Some(h) = obj.get("h").and_then(|v| v.as_f64()) {
                    current_h = h as f32;
                }
                if let Some(x) = obj.get("x").and_then(|v| v.as_f64()) {
                    current_x += x as f32;
                }
                if let Some(y) = obj.get("y").and_then(|v| v.as_f64()) {
                    current_y += y as f32;
                }
            } else if let Some(label) = item.as_str() {
                // This is a key with a label
                if let Some((row, col)) = parse_matrix_label(label) {
                    keys.push(Key {
                        row,
                        col,
                        x: current_x,
                        y: current_y,
                        w: current_w,
                        h: current_h,
                    });
                }

                current_x += current_w;

                current_w = 1.0;
                current_h = 1.0;
            }
        }

        current_y += 1.0;
    }

    Ok(keys)
}

fn parse_matrix_label(label: &str) -> Option<(usize, usize)> {
    let first_line = label.lines().next().unwrap_or(label);

    let parts: Vec<&str> = first_line.split(',').collect();
    if parts.len() >= 2 {
        let row = parts[0].trim().parse::<usize>().ok()?;
        let col = parts[1].trim().parse::<usize>().ok()?;
        return Some((row, col));
    }

    None
}
