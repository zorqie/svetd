use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchStore {
    pub fixtures: HashMap<String, PatchEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchEntry {
    pub name: String,
    pub address: u16,
    pub profile: String,
    pub mode: usize,
}

impl PatchStore {
    pub fn load_from_file(path: &str) -> Option<Self> {
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }
}
