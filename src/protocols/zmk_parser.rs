//! ZMK keymap parser
//!
//! Parses ZMK `.keymap` files (devicetree format) into `LayoutKey` structures.
//! Based on the approach used by keymap-drawer.
//!
//! # Limitations
//! - Does not handle C preprocessor directives (`#include`, `#define`)
//!   - User should pre-process the file or use a fully expanded keymap
//! - Custom behaviors beyond standard ZMK behaviors need manual mapping
//!
//! # Example
//! ```ignore
//! let keymap = ZmkKeymap::parse_file("path/to/keyboard.keymap")?;
//! for (layer_name, keys) in &keymap.layers {
//!     println!("Layer {}: {} keys", layer_name, keys.len());
//! }
//! ```

use crate::layout_key::{KeycodeKind, Label, LayoutKey};
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{Key, KeyboardDefinition, KeyboardLayout};

/// Parsed ZMK keymap containing layers and combos
#[derive(Debug, Clone)]
pub struct ZmkKeymap {
    /// Layer name -> list of keys (in matrix order)
    pub layers: HashMap<String, Vec<Option<LayoutKey>>>,
    /// Layer names in order they appear in the file
    pub layer_order: Vec<String>,
    /// Combo definitions
    pub combos: Vec<ZmkCombo>,
}

/// A ZMK combo definition
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ZmkCombo {
    /// Key positions that trigger the combo
    pub key_positions: Vec<usize>,
    /// The key action when combo triggers
    pub key: LayoutKey,
    /// Layers this combo is active on (empty = all layers)
    pub layers: Vec<String>,
}

impl ZmkKeymap {
    /// Parse a ZMK keymap from file contents
    pub fn parse(content: &str) -> Result<Self, Box<dyn Error>> {
        let mut keymap = ZmkKeymap {
            layers: HashMap::new(),
            layer_order: Vec::new(),
            combos: Vec::new(),
        };

        // Extract custom behaviors first (hold-taps, mod-morphs, etc.)
        let behaviors = extract_behaviors(content);

        // Parse layers from keymap node
        keymap.parse_layers(content, &behaviors)?;

        // Parse combos
        keymap.parse_combos(content, &behaviors)?;

        Ok(keymap)
    }

    /// Parse from a file path
    pub fn parse_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    fn parse_layers(
        &mut self,
        content: &str,
        behaviors: &BehaviorRegistry,
    ) -> Result<(), Box<dyn Error>> {
        // Find keymap node: keymap { compatible = "zmk,keymap"; ... }
        let keymap_re =
            Regex::new(r#"(?s)keymap\s*\{[^}]*compatible\s*=\s*"zmk,keymap"\s*;(.*?)\n\s*\};"#)
                .unwrap();

        let keymap_match = keymap_re
            .captures(content)
            .ok_or("Could not find keymap node with compatible = \"zmk,keymap\"")?;

        let keymap_content = keymap_match.get(1).unwrap().as_str();

        // Find layer nodes within keymap
        // Pattern: layer_name { bindings = <...>; };
        let layer_re = Regex::new(r"(?s)(\w+)\s*\{[^}]*bindings\s*=\s*<([^>]*)>").unwrap();

        for cap in layer_re.captures_iter(keymap_content) {
            let layer_name = cap.get(1).unwrap().as_str().to_string();
            let bindings_str = cap.get(2).unwrap().as_str();

            let keys = parse_bindings(bindings_str, behaviors);

            self.layer_order.push(layer_name.clone());
            self.layers.insert(layer_name, keys);
        }

        if self.layers.is_empty() {
            return Err("No layers found in keymap".into());
        }

        Ok(())
    }

    fn parse_combos(
        &mut self,
        content: &str,
        behaviors: &BehaviorRegistry,
    ) -> Result<(), Box<dyn Error>> {
        // Find combos node: combos { compatible = "zmk,combos"; ... }
        let combos_re =
            Regex::new(r#"(?s)combos\s*\{[^}]*compatible\s*=\s*"zmk,combos"\s*;(.*?)\n\s*\};"#)
                .unwrap();

        let Some(combos_match) = combos_re.captures(content) else {
            return Ok(()); // No combos section is fine
        };

        let combos_content = combos_match.get(1).unwrap().as_str();

        // Find individual combo definitions
        // Pattern: combo_name { key-positions = <...>; bindings = <...>; [layers = <...>;] };
        let combo_re = Regex::new(
            r"(?s)(\w+)\s*\{[^}]*key-positions\s*=\s*<([^>]*)>[^}]*bindings\s*=\s*<([^>]*)>([^}]*)\}",
        )
        .unwrap();

        for cap in combo_re.captures_iter(combos_content) {
            let positions_str = cap.get(2).unwrap().as_str();
            let bindings_str = cap.get(3).unwrap().as_str();
            let rest = cap.get(4).unwrap().as_str();

            // Parse key positions
            let positions: Vec<usize> = positions_str
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();

            // Parse binding (usually just one)
            let keys = parse_bindings(bindings_str, behaviors);
            let key = keys.into_iter().next().flatten().unwrap_or_default();

            // Parse layers if present
            let layers_re = Regex::new(r"layers\s*=\s*<([^>]*)>").unwrap();
            let layers: Vec<String> = layers_re
                .captures(rest)
                .map(|c| {
                    c.get(1)
                        .unwrap()
                        .as_str()
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            self.combos.push(ZmkCombo {
                key_positions: positions,
                key,
                layers,
            });
        }

        Ok(())
    }

    /// Convert the keymap to a 3D key matrix placed at proper (row, col) positions.
    ///
    /// Uses the physical key list to map the i-th binding to its (row, col) in the matrix.
    /// The resulting shape is [layer][row][col], matching VIA/VIAL format.
    pub fn to_matrix(
        &self,
        physical_keys: &[Key],
        rows: usize,
        cols: usize,
    ) -> Vec<Vec<Vec<Option<LayoutKey>>>> {
        self.layer_order
            .iter()
            .map(|name| {
                let bindings = self.layers.get(name).cloned().unwrap_or_default();
                let mut layer_matrix: Vec<Vec<Option<LayoutKey>>> = vec![vec![None; cols]; rows];

                for (i, key_def) in physical_keys.iter().enumerate() {
                    if let Some(layout_key) = bindings.get(i).cloned().flatten() {
                        if key_def.row < rows && key_def.col < cols {
                            layer_matrix[key_def.row][key_def.col] = Some(layout_key);
                        }
                    }
                }

                layer_matrix
            })
            .collect()
    }
}

/// Physical layout data parsed from a ZMK .overlay file
#[derive(Debug, Clone)]
pub struct ZmkPhysicalLayout {
    /// Physical key definitions with position, size, and matrix coordinates
    pub keys: Vec<Key>,
    /// Matrix row count from the matrix transform
    pub rows: usize,
    /// Matrix column count from the matrix transform
    pub cols: usize,
}

impl ZmkPhysicalLayout {
    /// Parse a physical layout from a .overlay file content
    pub fn parse(content: &str) -> Result<Self, Box<dyn Error>> {
        let (rows, cols) = parse_matrix_transform(content)?;
        let keys = parse_physical_keys(content)?;

        if keys.is_empty() {
            return Err("No physical key definitions found in overlay".into());
        }

        Ok(ZmkPhysicalLayout { keys, rows, cols })
    }

    /// Parse from a file path
    pub fn parse_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Build a KeyboardDefinition from the physical layout
    pub fn to_keyboard_definition(&self, vid: u16, pid: u16) -> KeyboardDefinition {
        let layout = KeyboardLayout {
            name: "Default Layout".to_string(),
            keys: self.keys.clone(),
        };

        KeyboardDefinition {
            vid,
            pid,
            rows: self.rows,
            cols: self.cols,
            layouts: vec![layout],
        }
    }
}

/// Parse matrix transform rows/columns from overlay content
///
/// Looks for: compatible = "zmk,matrix-transform"; columns = <N>; rows = <N>;
fn parse_matrix_transform(content: &str) -> Result<(usize, usize), Box<dyn Error>> {
    let transform_re = Regex::new(
        r#"(?s)compatible\s*=\s*"zmk,matrix-transform"[^}]*columns\s*=\s*<(\d+)>[^}]*rows\s*=\s*<(\d+)>"#,
    )
    .unwrap();

    // Try columns-then-rows order first
    if let Some(cap) = transform_re.captures(content) {
        let cols: usize = cap.get(1).unwrap().as_str().parse()?;
        let rows: usize = cap.get(2).unwrap().as_str().parse()?;
        return Ok((rows, cols));
    }

    // Try rows-then-columns order
    let transform_re2 = Regex::new(
        r#"(?s)compatible\s*=\s*"zmk,matrix-transform"[^}]*rows\s*=\s*<(\d+)>[^}]*columns\s*=\s*<(\d+)>"#,
    )
    .unwrap();

    if let Some(cap) = transform_re2.captures(content) {
        let rows: usize = cap.get(1).unwrap().as_str().parse()?;
        let cols: usize = cap.get(2).unwrap().as_str().parse()?;
        return Ok((rows, cols));
    }

    Err("Could not find matrix transform with rows/columns in overlay".into())
}

/// Parse key_physical_attrs entries from overlay content
///
/// Format: &key_physical_attrs WIDTH HEIGHT X Y ROTATION ROW COL
/// Width/Height/X/Y are in centi-units (100 = 1 key unit)
fn parse_physical_keys(content: &str) -> Result<Vec<Key>, Box<dyn Error>> {
    let key_re =
        Regex::new(r"&key_physical_attrs\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)")
            .unwrap();

    let keys: Vec<Key> = key_re
        .captures_iter(content)
        .map(|cap| {
            let w_centi: f32 = cap[1].parse().unwrap();
            let h_centi: f32 = cap[2].parse().unwrap();
            let x_centi: f32 = cap[3].parse().unwrap();
            let y_centi: f32 = cap[4].parse().unwrap();
            // cap[5] is rotation - not used for now
            let row: usize = cap[6].parse().unwrap();
            let col: usize = cap[7].parse().unwrap();

            Key {
                row,
                col,
                x: x_centi / 100.0,
                y: y_centi / 100.0,
                w: w_centi / 100.0,
                h: h_centi / 100.0,
            }
        })
        .collect();

    Ok(keys)
}

/// Discover and parse ZMK config files from a config directory.
///
/// Scans the directory structure for:
/// - `.overlay` files in `boards/shields/*/` for physical layout
/// - `.keymap` files in `config/` for key assignments
///
/// Returns the physical layout and keymap.
pub fn parse_zmk_config_dir(
    config_dir: &str,
) -> Result<(ZmkPhysicalLayout, ZmkKeymap), Box<dyn Error>> {
    let base = Path::new(config_dir);

    if !base.is_dir() {
        return Err(format!("ZMK config directory not found: {}", config_dir).into());
    }

    // Find .overlay file in boards/shields/*/
    let overlay_path = find_file_with_extension(base, &["boards", "shields"], "overlay")?;

    // Find .keymap file in config/
    let keymap_path = find_file_with_extension(base, &["config"], "keymap")?;

    let physical_layout = ZmkPhysicalLayout::parse_file(
        overlay_path
            .to_str()
            .ok_or("Invalid overlay path encoding")?,
    )?;

    let keymap =
        ZmkKeymap::parse_file(keymap_path.to_str().ok_or("Invalid keymap path encoding")?)?;

    Ok((physical_layout, keymap))
}

/// Find the first file with a given extension under a subdirectory path.
fn find_file_with_extension(
    base: &Path,
    subdirs: &[&str],
    extension: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let mut search_dir = base.to_path_buf();
    for sub in subdirs {
        search_dir = search_dir.join(sub);
    }

    if !search_dir.is_dir() {
        return Err(format!("Directory not found: {}", search_dir.display()).into());
    }

    // Search recursively for files with the given extension
    find_file_recursive(&search_dir, extension)?.ok_or_else(|| {
        format!(
            "No .{} file found under {}",
            extension,
            search_dir.display()
        )
        .into()
    })
}

fn find_file_recursive(dir: &Path, extension: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, extension)? {
                return Ok(Some(found));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// Registry of custom behaviors extracted from the keymap
#[derive(Debug, Default)]
struct BehaviorRegistry {
    /// Hold-tap behaviors: name -> (hold_behavior, tap_behavior)
    hold_taps: HashMap<String, (String, String)>,
    /// Mod-morph behaviors: name -> (default_binding, morphed_binding)
    mod_morphs: HashMap<String, (String, String)>,
    /// Sticky key behaviors: name -> base_behavior
    sticky_keys: HashMap<String, String>,
}

/// Extract custom behavior definitions from the devicetree
fn extract_behaviors(content: &str) -> BehaviorRegistry {
    let mut registry = BehaviorRegistry::default();

    // Built-in hold-tap behaviors
    registry
        .hold_taps
        .insert("mt".to_string(), ("&kp".to_string(), "&kp".to_string()));
    registry
        .hold_taps
        .insert("lt".to_string(), ("&mo".to_string(), "&kp".to_string()));

    // Built-in sticky behaviors
    registry
        .sticky_keys
        .insert("sk".to_string(), "&kp".to_string());
    registry
        .sticky_keys
        .insert("sl".to_string(), "&mo".to_string());

    // Find behavior nodes with compatible = "zmk,behavior-hold-tap"
    let hold_tap_re =
        Regex::new(r#"(?s)(\w+):\s*\w+\s*\{[^}]*compatible\s*=\s*"zmk,behavior-hold-tap"[^}]*\}"#)
            .unwrap();

    for cap in hold_tap_re.captures_iter(content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        // Default to kp for both if we can't determine
        registry
            .hold_taps
            .insert(name, ("&kp".to_string(), "&kp".to_string()));
    }

    // Find mod-morph behaviors
    let mod_morph_re = Regex::new(
        r#"(?s)(\w+):\s*\w+\s*\{[^}]*compatible\s*=\s*"zmk,behavior-mod-morph"[^}]*bindings\s*=\s*<([^>]*)>[^}]*\}"#
    ).unwrap();

    for cap in mod_morph_re.captures_iter(content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let bindings_str = cap.get(2).unwrap().as_str();
        let bindings: Vec<&str> = bindings_str.split(',').map(|s| s.trim()).collect();
        if bindings.len() >= 2 {
            registry
                .mod_morphs
                .insert(name, (bindings[0].to_string(), bindings[1].to_string()));
        }
    }

    registry
}

/// Parse a bindings string into LayoutKey list
fn parse_bindings(bindings_str: &str, behaviors: &BehaviorRegistry) -> Vec<Option<LayoutKey>> {
    // Split on & but keep the & prefix
    let mut keys = Vec::new();
    let mut current = String::new();

    for ch in bindings_str.chars() {
        if ch == '&' && !current.trim().is_empty() {
            if let Some(key) = parse_single_binding(current.trim(), behaviors) {
                keys.push(key);
            }
            current = String::from("&");
        } else {
            current.push(ch);
        }
    }

    // Don't forget the last binding
    if !current.trim().is_empty() {
        if let Some(key) = parse_single_binding(current.trim(), behaviors) {
            keys.push(key);
        }
    }

    keys
}

/// Parse a single ZMK binding into a LayoutKey
fn parse_single_binding(binding: &str, behaviors: &BehaviorRegistry) -> Option<Option<LayoutKey>> {
    let binding = binding.trim();
    if binding.is_empty() {
        return None;
    }

    // Split into behavior and params
    let parts: Vec<&str> = binding.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let behavior = parts[0];
    let params: Vec<&str> = parts[1..].to_vec();

    Some(binding_to_layout_key(behavior, &params, behaviors))
}

/// Convert a ZMK behavior + params to a LayoutKey
fn binding_to_layout_key(
    behavior: &str,
    params: &[&str],
    behaviors: &BehaviorRegistry,
) -> Option<LayoutKey> {
    // Strip the & prefix if present
    let behavior = behavior.strip_prefix('&').unwrap_or(behavior);

    match behavior {
        "none" => Some(LayoutKey {
            tap: Label::default(),
            kind: KeycodeKind::Basic,
            ..Default::default()
        }),

        "trans" => None, // Transparent = None

        "kp" => {
            // Key press: &kp A
            let key_name = params.first().copied().unwrap_or("");
            Some(LayoutKey {
                tap: Label::new(zmk_key_to_label(key_name)),
                kind: if is_modifier_key(key_name) {
                    KeycodeKind::Modifier
                } else {
                    KeycodeKind::Basic
                },
                symbol: zmk_key_to_symbol(key_name),
                ..Default::default()
            })
        }

        "kt" => {
            // Key toggle: &kt A
            let key_name = params.first().copied().unwrap_or("");
            Some(LayoutKey {
                tap: Label::new(format!("⇅{}", zmk_key_to_label(key_name))),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }

        "mo" => {
            // Momentary layer: &mo 1
            let layer = params.first().copied().unwrap_or("?");
            Some(LayoutKey {
                tap: Label::new(format!("L{}", layer)),
                kind: KeycodeKind::Modifier,
                layer_ref: layer.parse().ok(),
                ..Default::default()
            })
        }

        "to" => {
            // To layer: &to 1
            let layer = params.first().copied().unwrap_or("?");
            Some(LayoutKey {
                tap: Label::new(format!("→L{}", layer)),
                kind: KeycodeKind::Modifier,
                layer_ref: layer.parse().ok(),
                ..Default::default()
            })
        }

        "tog" => {
            // Toggle layer: &tog 1
            let layer = params.first().copied().unwrap_or("?");
            Some(LayoutKey {
                tap: Label::new(format!("⇅L{}", layer)),
                kind: KeycodeKind::Modifier,
                layer_ref: layer.parse().ok(),
                ..Default::default()
            })
        }

        "sl" => {
            // Sticky layer: &sl 1
            let layer = params.first().copied().unwrap_or("?");
            Some(LayoutKey {
                tap: Label::new(format!("L{}", layer)),
                hold: Some(Label::new("sticky")),
                kind: KeycodeKind::Modifier,
                layer_ref: layer.parse().ok(),
                ..Default::default()
            })
        }

        "sk" => {
            // Sticky key: &sk LSHIFT
            let key_name = params.first().copied().unwrap_or("");
            Some(LayoutKey {
                tap: Label::new(zmk_key_to_label(key_name)),
                hold: Some(Label::new("sticky")),
                kind: KeycodeKind::Modifier,
                ..Default::default()
            })
        }

        "mt" => {
            // Mod-tap: &mt LSHIFT A
            if params.len() >= 2 {
                let hold_key = params[0];
                let tap_key = params[1];
                Some(LayoutKey {
                    tap: Label::new(zmk_key_to_label(tap_key)),
                    hold: Some(Label::new(zmk_key_to_label(hold_key))),
                    kind: KeycodeKind::Modifier,
                    symbol: zmk_key_to_symbol(tap_key),
                    ..Default::default()
                })
            } else {
                Some(LayoutKey {
                    tap: Label::new("mt?"),
                    ..Default::default()
                })
            }
        }

        "lt" => {
            // Layer-tap: &lt 1 SPACE
            if params.len() >= 2 {
                let layer = params[0];
                let tap_key = params[1];
                Some(LayoutKey {
                    tap: Label::new(zmk_key_to_label(tap_key)),
                    hold: Some(Label::new(format!("L{}", layer))),
                    kind: KeycodeKind::Modifier,
                    layer_ref: layer.parse().ok(),
                    symbol: zmk_key_to_symbol(tap_key),
                    ..Default::default()
                })
            } else {
                Some(LayoutKey {
                    tap: Label::new("lt?"),
                    ..Default::default()
                })
            }
        }

        "bt" => {
            // Bluetooth: &bt BT_SEL 0
            let action = params.first().copied().unwrap_or("BT");
            let param = params.get(1).copied();
            let label = match action {
                "BT_CLR" => "BT Clr".to_string(),
                "BT_CLR_ALL" => "BT Clr All".to_string(),
                "BT_SEL" => format!("BT{}", param.unwrap_or("?")),
                "BT_NXT" => "BT→".to_string(),
                "BT_PRV" => "BT←".to_string(),
                _ => format!("BT {}", action),
            };
            Some(LayoutKey {
                tap: Label::new(label),
                kind: KeycodeKind::Special,
                symbol: Some(egui_phosphor::regular::BLUETOOTH.to_string()),
                ..Default::default()
            })
        }

        "out" => {
            // Output selection: &out OUT_USB
            let action = params.first().copied().unwrap_or("");
            let label = match action {
                "OUT_USB" => "USB",
                "OUT_BLE" => "BLE",
                "OUT_TOG" => "Out⇅",
                _ => action,
            };
            Some(LayoutKey {
                tap: Label::new(label),
                kind: KeycodeKind::Special,
                ..Default::default()
            })
        }

        "reset" | "sys_reset" => Some(LayoutKey {
            tap: Label::new("Reset"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),

        "bootloader" => Some(LayoutKey {
            tap: Label::new("Boot"),
            kind: KeycodeKind::Special,
            ..Default::default()
        }),

        "caps_word" => Some(LayoutKey {
            tap: Label::new("CapsW"),
            kind: KeycodeKind::Modifier,
            ..Default::default()
        }),

        "key_repeat" => Some(LayoutKey {
            tap: Label::new("Repeat"),
            kind: KeycodeKind::Special,
            symbol: Some(egui_phosphor::regular::REPEAT.to_string()),
            ..Default::default()
        }),

        // Check custom behaviors
        _ => {
            // Check if it's a custom hold-tap
            if let Some((_hold, _tap)) = behaviors.hold_taps.get(behavior) {
                if params.len() >= 2 {
                    let hold_key = params[0];
                    let tap_key = params[1];
                    return Some(LayoutKey {
                        tap: Label::new(zmk_key_to_label(tap_key)),
                        hold: Some(Label::new(zmk_key_to_label(hold_key))),
                        kind: KeycodeKind::Modifier,
                        symbol: zmk_key_to_symbol(tap_key),
                        ..Default::default()
                    });
                }
            }

            // Check if it's a mod-morph
            if let Some((default, _morphed)) = behaviors.mod_morphs.get(behavior) {
                // Just show the default binding
                return parse_single_binding(default, behaviors).flatten();
            }

            // Unknown behavior - show as-is
            let label = if params.is_empty() {
                behavior.to_string()
            } else {
                format!("{} {}", behavior, params.join(" "))
            };
            Some(LayoutKey {
                tap: Label::new(label),
                kind: KeycodeKind::Basic,
                ..Default::default()
            })
        }
    }
}

/// Check if a ZMK key name is a modifier
fn is_modifier_key(key: &str) -> bool {
    matches!(
        key.to_uppercase().as_str(),
        "LSHIFT"
            | "LSHFT"
            | "RSHIFT"
            | "RSHFT"
            | "LCTRL"
            | "LCTL"
            | "RCTRL"
            | "RCTL"
            | "LALT"
            | "RALT"
            | "LGUI"
            | "RGUI"
            | "LCMD"
            | "RCMD"
            | "LWIN"
            | "RWIN"
            | "LMETA"
            | "RMETA"
    )
}

// Lazy-initialized regex patterns for ZMK key conversion
static KEY_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Numbers: N1 -> 1
        (Regex::new(r"^N(\d)$").unwrap(), "$1"),
        // Function keys: F1 -> F1
        (Regex::new(r"^F(\d+)$").unwrap(), "F$1"),
    ]
});

/// Convert a ZMK key name to a display label
fn zmk_key_to_label(key: &str) -> String {
    let key = key.to_uppercase();

    // Apply regex patterns
    for (pattern, replacement) in KEY_PATTERNS.iter() {
        if pattern.is_match(&key) {
            return pattern.replace(&key, *replacement).to_string();
        }
    }

    // Direct mappings
    match key.as_str() {
        // Letters are already good
        k if k.len() == 1 && k.chars().next().unwrap().is_alphabetic() => k.to_string(),

        // Modifiers
        "LSHIFT" | "LSHFT" => "LShift".to_string(),
        "RSHIFT" | "RSHFT" => "RShift".to_string(),
        "LCTRL" | "LCTL" => "LCtrl".to_string(),
        "RCTRL" | "RCTL" => "RCtrl".to_string(),
        "LALT" => "LAlt".to_string(),
        "RALT" => "RAlt".to_string(),
        "LGUI" | "LCMD" | "LWIN" | "LMETA" => "LGui".to_string(),
        "RGUI" | "RCMD" | "RWIN" | "RMETA" => "RGui".to_string(),

        // Common keys
        "SPACE" | "SPC" => "Space".to_string(),
        "ENTER" | "RET" | "RETURN" => "Enter".to_string(),
        "TAB" => "Tab".to_string(),
        "ESCAPE" | "ESC" => "Esc".to_string(),
        "BACKSPACE" | "BSPC" => "Bksp".to_string(),
        "DELETE" | "DEL" => "Del".to_string(),
        "INSERT" | "INS" => "Ins".to_string(),
        "HOME" => "Home".to_string(),
        "END" => "End".to_string(),
        "PAGE_UP" | "PG_UP" => "PgUp".to_string(),
        "PAGE_DOWN" | "PG_DN" => "PgDn".to_string(),
        "CAPSLOCK" | "CAPS" => "Caps".to_string(),
        "PRINTSCREEN" | "PSCRN" => "PrtSc".to_string(),
        "SCROLLLOCK" | "SLCK" => "ScrLk".to_string(),
        "PAUSE_BREAK" => "Pause".to_string(),

        // Arrows
        "UP" => "↑".to_string(),
        "DOWN" => "↓".to_string(),
        "LEFT" => "←".to_string(),
        "RIGHT" => "→".to_string(),

        // Punctuation
        "MINUS" => "-".to_string(),
        "EQUAL" => "=".to_string(),
        "LBKT" | "LEFT_BRACKET" => "[".to_string(),
        "RBKT" | "RIGHT_BRACKET" => "]".to_string(),
        "BACKSLASH" | "BSLH" => "\\".to_string(),
        "SEMICOLON" | "SEMI" => ";".to_string(),
        "APOSTROPHE" | "APOS" | "SQT" => "'".to_string(),
        "GRAVE" => "`".to_string(),
        "COMMA" => ",".to_string(),
        "PERIOD" | "DOT" => ".".to_string(),
        "SLASH" | "FSLH" => "/".to_string(),

        // Numpad
        "KP_N0" | "KP_NUM_0" => "KP0".to_string(),
        "KP_N1" | "KP_NUM_1" => "KP1".to_string(),
        "KP_N2" | "KP_NUM_2" => "KP2".to_string(),
        "KP_N3" | "KP_NUM_3" => "KP3".to_string(),
        "KP_N4" | "KP_NUM_4" => "KP4".to_string(),
        "KP_N5" | "KP_NUM_5" => "KP5".to_string(),
        "KP_N6" | "KP_NUM_6" => "KP6".to_string(),
        "KP_N7" | "KP_NUM_7" => "KP7".to_string(),
        "KP_N8" | "KP_NUM_8" => "KP8".to_string(),
        "KP_N9" | "KP_NUM_9" => "KP9".to_string(),
        "KP_PLUS" => "KP+".to_string(),
        "KP_MINUS" => "KP-".to_string(),
        "KP_MULTIPLY" | "KP_ASTERISK" => "KP*".to_string(),
        "KP_DIVIDE" | "KP_SLASH" => "KP/".to_string(),
        "KP_DOT" => "KP.".to_string(),
        "KP_ENTER" => "KPEnt".to_string(),
        "KP_NUMLOCK" | "KP_NUM" => "NumLk".to_string(),

        // Media
        "C_VOL_UP" | "C_VOLUME_UP" => "Vol+".to_string(),
        "C_VOL_DN" | "C_VOLUME_DOWN" => "Vol-".to_string(),
        "C_MUTE" => "Mute".to_string(),
        "C_PLAY_PAUSE" | "C_PP" => "Play".to_string(),
        "C_NEXT" => "Next".to_string(),
        "C_PREV" | "C_PREVIOUS" => "Prev".to_string(),
        "C_STOP" => "Stop".to_string(),
        "C_BRI_UP" | "C_BRIGHTNESS_UP" => "Bri+".to_string(),
        "C_BRI_DN" | "C_BRIGHTNESS_DOWN" => "Bri-".to_string(),

        // Default: clean up underscores
        other => other.replace('_', " "),
    }
}

/// Get a symbol for a ZMK key if available
fn zmk_key_to_symbol(key: &str) -> Option<String> {
    let key = key.to_uppercase();
    match key.as_str() {
        "BACKSPACE" | "BSPC" => Some(egui_phosphor::regular::BACKSPACE.to_string()),
        "DELETE" | "DEL" => Some(egui_phosphor::regular::SELECTION_SLASH.to_string()),
        "ENTER" | "RET" | "RETURN" => Some(egui_phosphor::regular::ARROW_ELBOW_LEFT.to_string()),
        "TAB" => Some(egui_phosphor::regular::ARROW_LINE_RIGHT.to_string()),
        "SPACE" | "SPC" => Some(egui_phosphor::regular::ARROWS_OUT_LINE_HORIZONTAL.to_string()),
        "ESCAPE" | "ESC" => Some(egui_phosphor::regular::ARROW_SQUARE_OUT.to_string()),
        "CAPSLOCK" | "CAPS" => Some(egui_phosphor::regular::ARROW_FAT_UP.to_string()),
        "UP" => Some(egui_phosphor::regular::ARROW_UP.to_string()),
        "DOWN" => Some(egui_phosphor::regular::ARROW_DOWN.to_string()),
        "LEFT" => Some(egui_phosphor::regular::ARROW_LEFT.to_string()),
        "RIGHT" => Some(egui_phosphor::regular::ARROW_RIGHT.to_string()),
        "HOME" => Some(egui_phosphor::regular::HOUSE.to_string()),
        "END" => Some(egui_phosphor::regular::ARROW_LINE_DOWN_RIGHT.to_string()),
        "PAGE_UP" | "PG_UP" => Some(egui_phosphor::regular::CARET_DOUBLE_UP.to_string()),
        "PAGE_DOWN" | "PG_DN" => Some(egui_phosphor::regular::CARET_DOUBLE_DOWN.to_string()),
        "PRINTSCREEN" | "PSCRN" => Some(egui_phosphor::regular::CAMERA.to_string()),
        "C_VOL_UP" | "C_VOLUME_UP" => Some(egui_phosphor::regular::SPEAKER_HIGH.to_string()),
        "C_VOL_DN" | "C_VOLUME_DOWN" => Some(egui_phosphor::regular::SPEAKER_LOW.to_string()),
        "C_MUTE" => Some(egui_phosphor::regular::SPEAKER_X.to_string()),
        "C_PLAY_PAUSE" | "C_PP" => Some(egui_phosphor::regular::PLAY_PAUSE.to_string()),
        "C_NEXT" => Some(egui_phosphor::regular::SKIP_FORWARD.to_string()),
        "C_PREV" | "C_PREVIOUS" => Some(egui_phosphor::regular::SKIP_BACK.to_string()),
        "C_STOP" => Some(egui_phosphor::regular::STOP.to_string()),
        "C_BRI_UP" | "C_BRIGHTNESS_UP" => Some(egui_phosphor::regular::SUN.to_string()),
        "C_BRI_DN" | "C_BRIGHTNESS_DOWN" => Some(egui_phosphor::regular::SUN_DIM.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_keymap() {
        let content = r#"
            keymap {
                compatible = "zmk,keymap";
                
                default_layer {
                    bindings = <&kp A &kp B &kp C>;
                };
            };
        "#;

        let keymap = ZmkKeymap::parse(content).unwrap();
        assert_eq!(keymap.layer_order.len(), 1);
        assert_eq!(keymap.layer_order[0], "default_layer");

        let keys = keymap.layers.get("default_layer").unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].as_ref().unwrap().tap.full, "A");
        assert_eq!(keys[1].as_ref().unwrap().tap.full, "B");
        assert_eq!(keys[2].as_ref().unwrap().tap.full, "C");
    }

    #[test]
    fn test_parse_transparent() {
        let content = r#"
            keymap {
                compatible = "zmk,keymap";
                
                layer1 {
                    bindings = <&trans &kp A &trans>;
                };
            };
        "#;

        let keymap = ZmkKeymap::parse(content).unwrap();
        let keys = keymap.layers.get("layer1").unwrap();
        assert!(keys[0].is_none()); // transparent
        assert!(keys[1].is_some()); // A
        assert!(keys[2].is_none()); // transparent
    }

    #[test]
    fn test_parse_layer_tap() {
        let content = r#"
            keymap {
                compatible = "zmk,keymap";
                
                default {
                    bindings = <&lt 1 SPACE>;
                };
            };
        "#;

        let keymap = ZmkKeymap::parse(content).unwrap();
        let keys = keymap.layers.get("default").unwrap();
        let key = keys[0].as_ref().unwrap();
        assert_eq!(key.tap.full, "Space");
        assert_eq!(key.hold.as_ref().map(|l| l.full.as_str()), Some("L1"));
        assert_eq!(key.layer_ref, Some(1));
    }

    #[test]
    fn test_parse_mod_tap() {
        let content = r#"
            keymap {
                compatible = "zmk,keymap";
                
                default {
                    bindings = <&mt LSHIFT A>;
                };
            };
        "#;

        let keymap = ZmkKeymap::parse(content).unwrap();
        let keys = keymap.layers.get("default").unwrap();
        let key = keys[0].as_ref().unwrap();
        assert_eq!(key.tap.full, "A");
        assert_eq!(key.hold.as_ref().map(|l| l.full.as_str()), Some("LShift"));
    }

    #[test]
    fn test_zmk_key_to_label() {
        assert_eq!(zmk_key_to_label("A"), "A");
        assert_eq!(zmk_key_to_label("N1"), "1");
        assert_eq!(zmk_key_to_label("SPACE"), "Space");
        assert_eq!(zmk_key_to_label("LSHIFT"), "LShift");
        assert_eq!(zmk_key_to_label("BSPC"), "Bksp");
    }

    #[test]
    fn test_parse_physical_keys() {
        let content = r#"
            keys = <&key_physical_attrs 100 100    0    0 0 0 0>
                 , <&key_physical_attrs 100 100  100    0 0 0 1>
                 , <&key_physical_attrs 150 100  200  100 0 1 0>;
        "#;

        let keys = parse_physical_keys(content).unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].row, 0);
        assert_eq!(keys[0].col, 0);
        assert!((keys[0].x - 0.0).abs() < 0.01);
        assert!((keys[0].w - 1.0).abs() < 0.01);
        assert_eq!(keys[1].col, 1);
        assert!((keys[1].x - 1.0).abs() < 0.01);
        assert_eq!(keys[2].row, 1);
        assert!((keys[2].w - 1.5).abs() < 0.01);
        assert!((keys[2].y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_matrix_transform() {
        let content = r#"
            transform {
                compatible = "zmk,matrix-transform";
                columns = <4>;
                rows = <3>;
                map = <RC(0,0) RC(0,1)>;
            };
        "#;

        let (rows, cols) = parse_matrix_transform(content).unwrap();
        assert_eq!(rows, 3);
        assert_eq!(cols, 4);
    }

    #[test]
    fn test_to_matrix() {
        let content = r#"
            keymap {
                compatible = "zmk,keymap";

                default_layer {
                    bindings = <&kp A &kp B &kp C &kp D>;
                };
            };
        "#;

        let keymap = ZmkKeymap::parse(content).unwrap();

        let physical_keys = vec![
            Key {
                row: 0,
                col: 0,
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            Key {
                row: 0,
                col: 1,
                x: 1.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            Key {
                row: 1,
                col: 0,
                x: 0.0,
                y: 1.0,
                w: 1.0,
                h: 1.0,
            },
            Key {
                row: 1,
                col: 1,
                x: 1.0,
                y: 1.0,
                w: 1.0,
                h: 1.0,
            },
        ];

        let matrix = keymap.to_matrix(&physical_keys, 2, 2);
        assert_eq!(matrix.len(), 1); // 1 layer
        assert_eq!(matrix[0].len(), 2); // 2 rows
        assert_eq!(matrix[0][0].len(), 2); // 2 cols
        assert_eq!(matrix[0][0][0].as_ref().unwrap().tap.full, "A");
        assert_eq!(matrix[0][0][1].as_ref().unwrap().tap.full, "B");
        assert_eq!(matrix[0][1][0].as_ref().unwrap().tap.full, "C");
        assert_eq!(matrix[0][1][1].as_ref().unwrap().tap.full, "D");
    }
}
