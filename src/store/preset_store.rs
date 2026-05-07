use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct PresetStore {
    #[serde(default)]
    pub intensity: HashMap<String, HashMap<String, f32>>,
    #[serde(default)]
    pub color: HashMap<String, HashMap<String, f32>>,
    #[serde(default)]
    pub position: HashMap<String, HashMap<String, f32>>,
    #[serde(default)]
    pub beam: HashMap<String, HashMap<String, f32>>,
    #[serde(default)]
    pub other: HashMap<String, HashMap<String, f32>>,
}

impl PresetStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_file(path: &str) -> Self {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(store) = serde_json::from_str::<PresetStore>(&contents) {
                return store;
            } else {
                log::warn!("Could not parse presets file");
            }
        }
        
        let mut store = Self::new();
        store.color.insert("Red".to_string(), HashMap::from([("Red".to_string(), 100.0), ("Green".to_string(), 0.0), ("Blue".to_string(), 0.0), ("White".to_string(), 0.0)]));
        store.color.insert("Green".to_string(), HashMap::from([("Red".to_string(), 0.0), ("Green".to_string(), 100.0), ("Blue".to_string(), 0.0), ("White".to_string(), 0.0)]));
        store.color.insert("Blue".to_string(), HashMap::from([("Red".to_string(), 0.0), ("Green".to_string(), 0.0), ("Blue".to_string(), 100.0), ("White".to_string(), 0.0)]));
        store.color.insert("Purple".to_string(), HashMap::from([("Red".to_string(), 50.0), ("Green".to_string(), 0.0), ("Blue".to_string(), 50.0), ("White".to_string(), 0.0)]));
        store.color.insert("White".to_string(), HashMap::from([("Red".to_string(), 100.0), ("Green".to_string(), 100.0), ("Blue".to_string(), 100.0), ("White".to_string(), 100.0)]));
        
        store.position.insert("Center".to_string(), HashMap::from([("Pan".to_string(), 50.0), ("Tilt".to_string(), 50.0)]));
        store.position.insert("Up".to_string(), HashMap::from([("Tilt".to_string(), 100.0)]));
        store.position.insert("Down".to_string(), HashMap::from([("Tilt".to_string(), 0.0)]));
        
        store.beam.insert("Wide".to_string(), HashMap::from([("Zoom".to_string(), 100.0)]));

        store.save_to_file(path);
        store
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
