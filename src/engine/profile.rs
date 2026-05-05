use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OflProfile {
    pub name: String,
    #[serde(rename = "availableChannels")]
    pub available_channels: HashMap<String, OflChannel>,
    pub modes: Vec<OflMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OflChannel {
    #[serde(rename = "type")]
    pub channel_type: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OflMode {
    pub name: String,
    pub channels: Vec<String>,
}

impl OflProfile {
    pub fn load(path: &str) -> Option<Self> {
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }
}
