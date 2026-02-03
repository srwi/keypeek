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

    // Rotation state (persists across rows until changed)
    let mut rotation_angle: f32 = 0.0;
    let mut rotation_x: f32 = 0.0;
    let mut rotation_y: f32 = 0.0;

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

                // Handle rotation properties (these persist until changed)
                // When rx or ry changes, reset the current position to the rotation origin
                if let Some(rx) = obj.get("rx").and_then(|v| v.as_f64()) {
                    rotation_x = rx as f32;
                    current_x = 0.0; // Reset x relative to new rotation origin
                    current_y = 0.0; // Reset y relative to new rotation origin
                }
                if let Some(ry) = obj.get("ry").and_then(|v| v.as_f64()) {
                    rotation_y = ry as f32;
                    current_y = 0.0; // Reset y relative to new rotation origin
                }
                if let Some(r) = obj.get("r").and_then(|v| v.as_f64()) {
                    rotation_angle = r as f32;
                }

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
                    // Calculate the final position, applying rotation if needed
                    let (final_x, final_y) = if rotation_angle != 0.0 {
                        // Calculate the center of the key relative to rotation origin
                        let center_x = current_x + current_w / 2.0;
                        let center_y = current_y + current_h / 2.0;

                        // Apply rotation transformation to the center point
                        let angle_rad = rotation_angle.to_radians();
                        let cos_a = angle_rad.cos();
                        let sin_a = angle_rad.sin();

                        // Rotate the center position
                        let rotated_center_x = center_x * cos_a - center_y * sin_a;
                        let rotated_center_y = center_x * sin_a + center_y * cos_a;

                        // Convert back to top-left corner and add rotation origin
                        (
                            rotation_x + rotated_center_x - current_w / 2.0,
                            rotation_y + rotated_center_y - current_h / 2.0,
                        )
                    } else {
                        (current_x, current_y)
                    };

                    keys.push(Key {
                        row,
                        col,
                        x: final_x,
                        y: final_y,
                        w: current_w,
                        h: current_h,
                    });
                }

                current_x += current_w;

                current_w = 1.0;
                current_h = 1.0;
            }
        }

        // Only increment y for non-rotated keys
        // For rotated keys, y is relative to rotation origin
        if rotation_angle == 0.0 {
            current_y += 1.0;
        }
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
