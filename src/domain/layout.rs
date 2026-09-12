//! Keyboard physical layout and geometry definitions.

pub mod geometry;

pub type Row = usize;
pub type Column = usize;

/// Physical key placement, dimensions, matrix coordinates, and rotation angle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub row: Row,
    pub col: Column,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Rotation angle in degrees, clockwise around the key's center.
    #[serde(default)]
    pub r: f32,
}

/// Named physical layout option for a keyboard (e.g. "Default", "ISO", "Split Backspace").
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardLayout {
    pub name: String,
    pub keys: Vec<Key>,
}

impl KeyboardLayout {
    pub fn get_dimensions(&self) -> (f32, f32) {
        let max_x = self.keys.iter().map(|k| k.x + k.w).fold(0.0, f32::max);
        let max_y = self.keys.iter().map(|k| k.y + k.h).fold(0.0, f32::max);
        (max_x, max_y)
    }
}

/// Complete physical definition of a keyboard, including row/col matrix bounds
/// and all supported layout geometry variants.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardDefinition {
    pub vid: u16,
    pub pid: u16,
    pub rows: usize,
    pub cols: usize,
    pub layouts: Vec<KeyboardLayout>,
}

impl KeyboardDefinition {
    pub fn get_layout_names(&self) -> Vec<String> {
        self.layouts.iter().map(|l| l.name.clone()).collect()
    }

    pub fn get_layout(&self, layout_name: &str) -> Result<KeyboardLayout, String> {
        self.layouts
            .iter()
            .find(|l| l.name == layout_name)
            .cloned()
            .ok_or_else(|| format!("Layout '{}' not found.", layout_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_dimensions() {
        let layout = KeyboardLayout {
            name: "test".to_string(),
            keys: vec![
                Key { row: 0, col: 0, x: 0.0, y: 0.0, w: 1.0, h: 1.0, r: 0.0 },
                Key { row: 0, col: 1, x: 1.0, y: 0.0, w: 2.0, h: 1.0, r: 0.0 },
                Key { row: 1, col: 0, x: 0.0, y: 1.0, w: 1.0, h: 2.0, r: 0.0 },
            ],
        };
        assert_eq!(layout.get_dimensions(), (3.0, 3.0));
    }

    #[test]
    fn test_definition_layout_lookup() {
        let def = KeyboardDefinition {
            vid: 0x1234,
            pid: 0x5678,
            rows: 2,
            cols: 2,
            layouts: vec![
                KeyboardLayout { name: "ANSI".to_string(), keys: vec![] },
                KeyboardLayout { name: "ISO".to_string(), keys: vec![] },
            ],
        };
        assert_eq!(def.get_layout_names(), vec!["ANSI", "ISO"]);
        assert!(def.get_layout("ISO").is_ok());
        assert!(def.get_layout("Dvorak").is_err());
    }
}
