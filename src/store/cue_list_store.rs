use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use crate::engine::command::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CueListStep {
    pub fade_in: Option<Duration>,
    pub hold: Option<Duration>,
    pub fade_out: Option<Duration>,
    pub cue: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CueListStepConfig {
    Simple(String),
    Detailed(CueListStep),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CueList {
    pub name: String,
    pub cues: Vec<CueListStepConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CueListStore {
    pub lists: HashMap<String, CueList>,
}

impl CueListStore {
    pub fn load_from_file(path: &str) -> Option<Self> {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(store) = serde_json::from_str::<Self>(&contents) {
                return Some(store);
            }
        }
        None
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
