pub mod kle_parser;
pub mod qmk_json_parser;
pub mod via;
pub mod vial;

use std::error::Error;

pub type Row = usize;
pub type Column = usize;

#[derive(Debug, Clone)]
pub struct Key {
    pub row: Row,
    pub col: Column,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone)]
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

#[derive(Clone)]
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

pub trait KeyboardProtocol: Send {
    fn get_layout_definition(&self) -> &KeyboardDefinition;

    fn get_layer_count(&self) -> Result<usize, Box<dyn Error>>;

    fn read_all_keycodes(&self, layers: usize, rows: usize, cols: usize) -> Vec<Vec<Vec<u16>>>;

    fn hid_read(&self) -> Result<Vec<u8>, Box<dyn Error>>;
}
