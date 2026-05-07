use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layout {
    pub heads: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LayoutStore {
    pub layouts: HashMap<String, Layout>,
}

impl LayoutStore {
    pub fn load_from_file(path: &str) -> Option<Self> {
        let contents = fs::read_to_string(path).ok()?;
        let layouts: HashMap<String, Layout> = serde_json::from_str(&contents).ok()?;
        Some(Self { layouts })
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(&self.layouts) {
            let _ = fs::write(path, json);
        }
    }

    pub fn get_default_layout(&self) -> Option<&Layout> {
        if let Some(l) = self.layouts.get("plan1") {
            return Some(l);
        }
        self.layouts.values().next()
    }
}
