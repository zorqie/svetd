use crate::engine::command::{Command, Value};
use crate::engine::profile::OflProfile;
use crate::param::FloatParam;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Fixture {
    pub name: String,
    pub address: u16,
    pub universe: u16,
    pub profile: Arc<OflProfile>,
    pub mode_index: usize,
    pub parameters: HashMap<String, Arc<FloatParam>>,
}

impl Fixture {
    pub fn new(name: &str, address: u16, profile: Arc<OflProfile>, mode_index: usize) -> Self {
        let mut parameters = HashMap::new();

        if let Some(mode) = profile.modes.get(mode_index) {
            for ch_name in &mode.channels {
                let mut default_val = 0.0;
                if let Some(ch_def) = profile.available_channels.get(ch_name) {
                    if ch_def.channel_type == "Single Color" {
                        default_val = 255.0;
                    }
                }
                parameters.insert(ch_name.clone(), Arc::new(FloatParam::new(default_val)));
            }

            let has_dimmer = mode.channels.iter().any(|c| {
                if let Some(ch) = profile.available_channels.get(c) {
                    ch.channel_type == "Intensity"
                } else {
                    false
                }
            });

            let has_color = mode.channels.iter().any(|c| {
                if let Some(ch) = profile.available_channels.get(c) {
                    ch.channel_type == "Single Color"
                } else {
                    false
                }
            });

            if !has_dimmer && has_color {
                parameters.insert("Dimmer".to_string(), Arc::new(FloatParam::new(0.0)));
            }
        }

        Self {
            name: name.to_string(),
            address,
            universe: 1,
            profile,
            mode_index,
            parameters,
        }
    }

    pub fn handle_command(&self, cmd: &Command) {
        match cmd {
            Command::SetLevel { value, .. } => {
                let level = match value {
                    Value::Numeric(n) => *n,
                    Value::Semantic(_) => 255.0,
                };
                if let Some(p) = self.parameters.get("Dimmer") {
                    p.store(level);
                }
            }
            Command::SetColor { color, .. } => {
                if let Value::Semantic(c) = color {
                    let (r, g, b) = match c.to_lowercase().as_str() {
                        "red" => (255.0, 0.0, 0.0),
                        "green" => (0.0, 255.0, 0.0),
                        "blue" => (0.0, 0.0, 255.0),
                        "purple" => (128.0, 0.0, 128.0),
                        "white" => (255.0, 255.0, 255.0),
                        _ => (0.0, 0.0, 0.0),
                    };
                    if let Some(p) = self.parameters.get("Red") {
                        p.store(r);
                    }
                    if let Some(p) = self.parameters.get("Green") {
                        p.store(g);
                    }
                    if let Some(p) = self.parameters.get("Blue") {
                        p.store(b);
                    }
                    if let Some(p) = self.parameters.get("White") {
                        if c.to_lowercase() == "white" {
                            p.store(255.0);
                        } else {
                            p.store(0.0);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn render(&self, dmx_buffer: &mut [u8], times_buffer: &mut [u64]) {
        if let Some(mode) = self.profile.modes.get(self.mode_index) {
            let has_physical_dimmer = mode.channels.iter().any(|c| {
                if let Some(ch) = self.profile.available_channels.get(c) {
                    ch.channel_type == "Intensity"
                } else {
                    false
                }
            });
            
            let dimmer_param = self.parameters.get("Dimmer");
            let dimmer_val = if has_physical_dimmer {
                1.0 
            } else {
                dimmer_param.map(|p| p.load() / 255.0).unwrap_or(1.0)
            };
            let dimmer_time = dimmer_param.map(|p| p.last_updated()).unwrap_or(0);

            for (i, ch_name) in mode.channels.iter().enumerate() {
                let offset = i as u16;
                let addr = (self.address - 1 + offset) as usize;
                
                if addr < dmx_buffer.len() {
                    let param = self.parameters.get(ch_name);
                    let mut val = param.map(|p| p.load()).unwrap_or(0.0);
                    let mut time = param.map(|p| p.last_updated()).unwrap_or(0);
                    
                    if let Some(ch_def) = self.profile.available_channels.get(ch_name) {
                        if !has_physical_dimmer && ch_def.channel_type == "Single Color" {
                            val *= dimmer_val;
                            time = time.max(dimmer_time);
                        }
                    }
                    
                    dmx_buffer[addr] = val as u8;
                    times_buffer[addr] = time;
                }
            }
        }
    }
}
