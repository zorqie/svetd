use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub name: String,
    pub heads: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GroupStore {
    pub groups: HashMap<String, Group>,
}

impl GroupStore {
    pub fn load_from_file(path: &str) -> Option<Self> {
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
