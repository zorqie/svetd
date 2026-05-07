use std::collections::HashMap;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::engine::command::Cue;
use crate::input::parser::{parse_command, ParserState};

#[derive(Debug, Serialize, Deserialize)]
pub struct CueStore {
    pub cues: HashMap<String, Cue>,
}

impl CueStore {
    pub fn new() -> Self {
        Self {
            cues: HashMap::new(),
        }
    }

    pub fn load_from_file(path: &str) -> Self {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(mut store) = serde_json::from_str::<CueStore>(&contents) {
                let mut modified = false;
                for cue in store.cues.values_mut() {
                    if cue.commands.is_empty() && !cue.raw_commands.is_empty() {
                        let mut state = ParserState::default();
                        for raw in &cue.raw_commands {
                            if let Some(cmds) = parse_command(raw, &mut state) {
                                cue.commands.extend(cmds);
                            }
                        }
                        modified = true;
                    }
                }
                
                if modified {
                    store.save_to_file(path);
                }
                return store;
            } else {
                log::warn!("Could not parse cues file");
            }
        }
        
        Self::new()
    }

    pub fn save_to_file(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }
}
