use super::{effects::Effect, target::Target};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Duration {
    Time(f32),  // in seconds
    Tempo(f32), // in beats
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Numeric(f32),
    Semantic(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    SetLevel { target: Target, value: Value },
    SetColor { target: Target, color: Value },
    SetPosition { target: Target, pos: Value }, 
    SetStrobe { target: Target, strobe: Value },
    SetTempo { bpm: f32 },
    StartEffect { target: Target, effect: Effect },
    StopEffect { target: Target },
    StartCueList { list: String },
    StopCueList { list: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Command>,
}

impl Cue {
    pub fn new(name: &str, commands: Vec<Command>) -> Self {
        Self {
            name: name.to_string(),
            raw_commands: Vec::new(),
            commands,
        }
    }
}
