use crate::engine::command::Command;
use crate::engine::effects::Effect;
use crate::input::parser::{PARSER_STATE, ParserState, parse_command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedEffect {
    pub name: String,
    pub raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Effect>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EffectStore {
    pub effects: HashMap<String, SavedEffect>,
}

impl EffectStore {
    pub fn load_from_file(path: &str) -> Self {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(mut store) = serde_json::from_str::<EffectStore>(&contents) {
                let mut modified = false;

                for (_id, effect) in store.effects.iter_mut() {
                    if effect.params.is_none() && !effect.raw.is_empty() {
                        let mut state = ParserState::default();
                        let dummy_cmd = format!("1@{}", effect.raw);
                        if let Some(cmds) = parse_command(&dummy_cmd, &mut state) {
                            for cmd in cmds {
                                if let Command::StartEffect {
                                    effect: parsed_fx, ..
                                } = cmd
                                {
                                    effect.params = Some(parsed_fx);
                                }
                            }
                        }
                        modified = true;
                    }
                }

                if modified {
                    let _ = fs::write(path, serde_json::to_string_pretty(&store).unwrap());
                }

                let mut parser_state = PARSER_STATE.lock().unwrap();
                for (id, fx) in &store.effects {
                    if let Some(interp) = &fx.params {
                        parser_state
                            .saved_effects
                            .insert(id.clone(), interp.clone());
                    }
                }

                return store;
            }
        }
        Self::default()
    }
}
