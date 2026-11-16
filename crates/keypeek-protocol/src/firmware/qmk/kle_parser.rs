use keypeek_core::geometry::flattened_top_left_after_center_rotation;
use keypeek_core::{Key, KeyboardDefinition, KeyboardLayout};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;

pub fn parse_vial_definition(
    json: &Value,
    vid: u16,
    pid: u16,
) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let keymap = find_kle_keymap(json)
        .ok_or_else(|| Box::<dyn Error>::from("No 'keymap' found in definition"))?;
    parse_kle_definition_with_prefix(json, keymap, vid, pid, "Vial")
}

use super::catalog::resolve_vendor_product_ids;

pub fn parse_kle_definition(
    json: &Value,
    keymap: &[Value],
    vid: u16,
    pid: u16,
) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let prefix = if json.get("lighting").and_then(Value::as_str) == Some("vialrgb")
        || json.get("vial").is_some()
    {
        "Vial"
    } else {
        "VIA"
    };
    parse_kle_definition_with_prefix(json, keymap, vid, pid, prefix)
}

fn parse_kle_definition_with_prefix(
    json: &Value,
    keymap: &[Value],
    vid: u16,
    pid: u16,
    prefix: &str,
) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let (vid, pid) = resolve_vendor_product_ids(json, vid, pid);
    let matrix = json
        .get("matrix")
        .ok_or_else(|| Box::<dyn Error>::from("No 'matrix' found in definition"))?;

    let rows = matrix
        .get("rows")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Box::<dyn Error>::from("No 'rows' in matrix"))? as usize;

    let cols = matrix
        .get("cols")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| Box::<dyn Error>::from("No 'cols' in matrix"))? as usize;

    let labels = json
        .get("layouts")
        .and_then(|l| l.get("labels"))
        .or_else(|| json.get("labels"));

    let (base_keys, option_groups) = parse_kle_keymap_elements(keymap, labels)?;
    let layouts = build_layout_variants(base_keys, &option_groups, prefix);

    Ok(KeyboardDefinition {
        vid,
        pid,
        rows,
        cols,
        layouts,
    })
}

/// Locates a KLE raw keymap array in either `layouts.keymap` or top-level `keymap`.
pub fn find_kle_keymap(json: &Value) -> Option<&[Value]> {
    if let Some(layouts) = json.get("layouts") {
        if let Some(keymap) = layouts.get("keymap") {
            if let Some(arr) = keymap.as_array() {
                return Some(arr);
            }
            if let Some(arr) = keymap.get("layout").and_then(Value::as_array) {
                return Some(arr);
            }
        }
    }
    if let Some(keymap) = json.get("keymap") {
        if let Some(arr) = keymap.as_array() {
            return Some(arr);
        }
        if let Some(arr) = keymap.get("layout").and_then(Value::as_array) {
            return Some(arr);
        }
    }
    None
}

struct ViaOption {
    index: usize,
    name: String,
    keys: Vec<Key>,
}

struct ViaOptionGroup {
    index: usize,
    options: Vec<ViaOption>,
}

pub fn parse_via_definition(
    json: &Value,
    vid: u16,
    pid: u16,
) -> Result<KeyboardDefinition, Box<dyn Error>> {
    let (vid, pid) = resolve_vendor_product_ids(json, vid, pid);
    let matrix = json
        .get("matrix")
        .ok_or_else(|| Box::<dyn Error>::from("No 'matrix' found in VIA definition"))?;
    let rows = matrix
        .get("rows")
        .and_then(Value::as_u64)
        .ok_or_else(|| Box::<dyn Error>::from("No 'rows' in matrix"))? as usize;
    let cols = matrix
        .get("cols")
        .and_then(Value::as_u64)
        .ok_or_else(|| Box::<dyn Error>::from("No 'cols' in matrix"))? as usize;
    let layouts = json
        .get("layouts")
        .ok_or_else(|| Box::<dyn Error>::from("No 'layouts' in VIA definition"))?;
    let raw_keys = layouts
        .get("keys")
        .and_then(Value::as_array)
        .ok_or_else(|| Box::<dyn Error>::from("No 'keys' in VIA layouts"))?;

    let base_keys = parse_via_keys(raw_keys)?;
    let option_groups = parse_via_option_groups(layouts, layouts.get("labels"))?;
    let layouts = build_layout_variants(base_keys, &option_groups, "VIA");

    Ok(KeyboardDefinition {
        vid,
        pid,
        rows,
        cols,
        layouts,
    })
}

fn parse_via_option_groups(
    layouts: &Value,
    labels: Option<&Value>,
) -> Result<Vec<ViaOptionGroup>, Box<dyn Error>> {
    let mut option_groups = Vec::new();
    let Some(groups) = layouts.get("optionKeys").and_then(Value::as_object) else {
        return Ok(option_groups);
    };

    for (group_str, options_val) in groups {
        let Some(options_obj) = options_val.as_object() else {
            continue;
        };
        let group_idx = group_str.parse::<usize>().unwrap_or(usize::MAX);
        let mut parsed_options = Vec::new();

        for (option_str, keys_val) in options_obj {
            let Some(keys_arr) = keys_val.as_array() else {
                continue;
            };
            let option_idx = option_str.parse::<usize>().unwrap_or(usize::MAX);
            let keys = parse_via_keys(keys_arr)?;
            let opt_name = resolve_via_option_name(labels, group_idx, option_idx)
                .unwrap_or_else(|| format!("{group_str}={option_str}"));
            parsed_options.push(ViaOption {
                index: option_idx,
                name: opt_name,
                keys,
            });
        }

        parsed_options.sort_by_key(|opt| opt.index);
        if !parsed_options.is_empty() {
            option_groups.push(ViaOptionGroup {
                index: group_idx,
                options: parsed_options,
            });
        }
    }

    option_groups.sort_by_key(|g| g.index);
    Ok(option_groups)
}

fn build_layout_variants(
    base_keys: Vec<Key>,
    option_groups: &[ViaOptionGroup],
    prefix: &str,
) -> Vec<KeyboardLayout> {
    if option_groups.is_empty() {
        return vec![KeyboardLayout {
            name: "default".to_string(),
            keys: base_keys,
        }];
    }

    let mut layouts = Vec::new();

    // 1. Always create the Default layout (option 0 for all groups)
    let mut default_keys = base_keys.clone();
    for group in option_groups {
        if let Some(first_opt) = group.options.first() {
            default_keys.extend(first_opt.keys.clone());
        }
    }
    layouts.push(KeyboardLayout {
        name: "default".to_string(),
        keys: default_keys,
    });

    // 2. Generate combinations or individual option variants
    let total_combinations: usize = option_groups
        .iter()
        .map(|g| g.options.len())
        .fold(1usize, |acc, count| acc.saturating_mul(count));

    if total_combinations > 1 && total_combinations <= 32 {
        let mut combinations: Vec<Vec<(String, Vec<Key>, bool)>> = vec![Vec::new()];
        for group in option_groups {
            let mut next = Vec::new();
            for comb in &combinations {
                for opt in &group.options {
                    let mut next_comb = comb.clone();
                    let is_default = opt.index == 0;
                    next_comb.push((opt.name.clone(), opt.keys.clone(), is_default));
                    next.push(next_comb);
                }
            }
            combinations = next;
        }

        for combination in combinations {
            let all_default = combination.iter().all(|(_, _, is_def)| *is_def);
            if all_default {
                continue; // Already added as "default"
            }

            let non_default_names: Vec<String> = combination
                .iter()
                .filter(|(_, _, is_def)| !*is_def)
                .map(|(name, _, _)| name.clone())
                .collect();

            let mut keys = base_keys.clone();
            for (_, opt_keys, _) in combination {
                keys.extend(opt_keys);
            }

            layouts.push(KeyboardLayout {
                name: format!("{prefix} ({})", non_default_names.join(", ")),
                keys,
            });
        }
    } else if total_combinations > 32 {
        // High permutation count: add one layout variant per non-default option
        for target_group in option_groups {
            for opt in &target_group.options {
                if opt.index == 0 {
                    continue;
                }
                let mut keys = base_keys.clone();
                for group in option_groups {
                    if group.index == target_group.index {
                        keys.extend(opt.keys.clone());
                    } else if let Some(def_opt) = group.options.first() {
                        keys.extend(def_opt.keys.clone());
                    }
                }
                layouts.push(KeyboardLayout {
                    name: format!("{prefix} ({})", opt.name),
                    keys,
                });
            }
        }
    }

    layouts
}

fn resolve_via_option_name(
    labels: Option<&Value>,
    group_idx: usize,
    option_idx: usize,
) -> Option<String> {
    let group_val = labels.and_then(|l| l.get(group_idx))?;
    if let Some(s) = group_val.as_str() {
        if option_idx == 0 {
            Some(format!("Default {s}"))
        } else {
            Some(s.to_string())
        }
    } else if let Some(arr) = group_val.as_array() {
        let opt_label = arr.get(option_idx + 1).and_then(Value::as_str)?;
        if let Some(group_title) = arr.first().and_then(Value::as_str) {
            Some(format!("{group_title}: {opt_label}"))
        } else {
            Some(opt_label.to_string())
        }
    } else {
        None
    }
}

fn parse_via_keys(raw_keys: &[Value]) -> Result<Vec<Key>, Box<dyn Error>> {
    let mut keys = Vec::with_capacity(raw_keys.len());
    for raw_key in raw_keys {
        let key = raw_key
            .as_object()
            .ok_or_else(|| Box::<dyn Error>::from("VIA layout key is not an object"))?;

        // Skip decal / cosmetic elements
        if key.get("d").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }

        // Keys without row or col are decals, plate borders, or spacers
        let (Some(row), Some(col)) = (
            key.get("row").and_then(Value::as_u64),
            key.get("col").and_then(Value::as_u64),
        ) else {
            continue;
        };

        let x = key
            .get("x")
            .and_then(Value::as_f64)
            .ok_or_else(|| Box::<dyn Error>::from("VIA layout key has no x"))?
            as f32;
        let y = key
            .get("y")
            .and_then(Value::as_f64)
            .ok_or_else(|| Box::<dyn Error>::from("VIA layout key has no y"))?
            as f32;
        let w = key.get("w").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        let h = key.get("h").and_then(Value::as_f64).unwrap_or(1.0) as f32;
        let r = key.get("r").and_then(Value::as_f64).unwrap_or(0.0) as f32;
        let rx = key
            .get("rx")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(x);
        let ry = key
            .get("ry")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(y);

        let (final_x, final_y) = flattened_top_left_after_center_rotation(x, y, w, h, r, rx, ry);

        keys.push(Key {
            row: row as usize,
            col: col as usize,
            x: final_x,
            y: final_y,
            w,
            h,
            r,
        });
    }
    Ok(keys)
}

fn parse_kle_keymap_elements(
    keymap: &[Value],
    labels: Option<&Value>,
) -> Result<(Vec<Key>, Vec<ViaOptionGroup>), Box<dyn Error>> {
    let mut base_keys = Vec::new();
    // group -> choice -> (physical keys, bounding-box top-left for aligning alternatives to choice 0)
    let mut groups: BTreeMap<usize, BTreeMap<usize, (Vec<Key>, Option<(f32, f32)>)>> =
        BTreeMap::new();

    let mut current_y: f32 = 0.0;
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
        let mut current_decal = false;

        for item in row_array {
            if let Some(obj) = item.as_object() {
                // Handle rotation properties (these persist until changed)
                // When rx or ry changes, reset the current position to the rotation origin
                let mut origin_changed = false;
                if let Some(rx) = obj.get("rx").and_then(|v| v.as_f64()) {
                    rotation_x = rx as f32;
                    origin_changed = true;
                }
                if let Some(ry) = obj.get("ry").and_then(|v| v.as_f64()) {
                    rotation_y = ry as f32;
                    origin_changed = true;
                }

                if origin_changed {
                    current_x = 0.0;
                    current_y = 0.0;
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
                if let Some(d) = obj.get("d").and_then(|v| v.as_bool()) {
                    current_decal = d;
                }
            } else if let Some(label) = item.as_str() {
                // Skip rotary encoder rotation controls before tracking option positions
                // so encoder decals/controls never skew switch bounding boxes.
                let is_encoder = label.split('\n').nth(9).map(str::trim) == Some("e");
                if !is_encoder {
                    let absolute_x = rotation_x + current_x;
                    let absolute_y = rotation_y + current_y;
                    let (final_x, final_y) = flattened_top_left_after_center_rotation(
                        absolute_x,
                        absolute_y,
                        current_w,
                        current_h,
                        rotation_angle,
                        rotation_x,
                        rotation_y,
                    );

                    let key = (!current_decal)
                        .then(|| {
                            parse_matrix_label(label).map(|(row, col)| Key {
                                row,
                                col,
                                x: final_x,
                                y: final_y,
                                w: current_w,
                                h: current_h,
                                r: rotation_angle,
                            })
                        })
                        .flatten();

                    if let Some((grp, opt)) = parse_option_choice(label) {
                        let (keys, min_pos) =
                            groups.entry(grp).or_default().entry(opt).or_default();
                        *min_pos = Some(match *min_pos {
                            Some((mx, my)) => (mx.min(final_x), my.min(final_y)),
                            None => (final_x, final_y),
                        });
                        if let Some(key) = key {
                            keys.push(key);
                        }
                    } else if let Some(key) = key {
                        base_keys.push(key);
                    }
                }

                current_x += current_w;

                current_w = 1.0;
                current_h = 1.0;
                current_decal = false;
            }
        }

        current_y += 1.0;
    }

    let mut option_groups = Vec::new();
    for (grp, mut choices) in groups {
        choices.entry(0).or_default();
        let c0_min = choices.get(&0).and_then(|(_, min)| *min);

        let mut parsed_options = Vec::new();
        for (choice_idx, (mut keys, cn_min)) in choices {
            if choice_idx > 0 {
                if let (Some((c0_x, c0_y)), Some((cn_x, cn_y))) = (c0_min, cn_min) {
                    let shift_x = cn_x - c0_x;
                    let shift_y = cn_y - c0_y;
                    for key in &mut keys {
                        key.x -= shift_x;
                        key.y -= shift_y;
                    }
                }
            }

            let opt_name = resolve_via_option_name(labels, grp, choice_idx)
                .unwrap_or_else(|| format!("{grp}={choice_idx}"));
            parsed_options.push(ViaOption {
                index: choice_idx,
                name: opt_name,
                keys,
            });
        }

        if parsed_options.iter().any(|opt| !opt.keys.is_empty()) {
            option_groups.push(ViaOptionGroup {
                index: grp,
                options: parsed_options,
            });
        }
    }

    Ok((base_keys, option_groups))
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

fn parse_option_choice(label: &str) -> Option<(usize, usize)> {
    let slot = label.split('\n').nth(3)?.trim();
    let (opt, choice) = slot.split_once(',')?;
    Some((opt.trim().parse().ok()?, choice.trim().parse().ok()?))
}
