use serde::Deserialize;
use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub web: WebSettings,
    pub osc: OscSettings,
    pub engine: EngineSettings,
    pub artnet: ArtnetSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebSettings {
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OscSettings {
    pub bind_address: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EngineSettings {
    pub patch_file: String,
    pub profiles_dir: String,
    pub cues_file: String,
    pub presets_file: String,
    pub groups_file: String,
    pub layout_file: String,
    pub effects_file: String,
    pub cue_lists_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ArtnetSettings {
    pub target_ip: String,
    pub universe: u16,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start off by merging in the "default" configuration file if it exists
            .add_source(File::with_name("config/default").required(false))
            // Add in a local configuration file
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from the environment (with a prefix of SVETD)
            // Eg.. `SVETD_WEB__BIND_ADDRESS=0.0.0.0:80` would set the `web.bind_address` key
            .add_source(Environment::with_prefix("SVETD").separator("__"))
            // Programmatically fallback to default settings if neither files nor env are present
            .set_default("web.bind_address", "0.0.0.0:8080")?
            .set_default("osc.bind_address", "127.0.0.1")?
            .set_default("osc.port", 8001)?
            .set_default("engine.patch_file", "config/patch.json")?
            .set_default("engine.profiles_dir", "profiles")?
            .set_default("engine.cues_file", "config/cues.json")?
            .set_default("engine.presets_file", "config/presets.json")?
            .set_default("engine.groups_file", "config/groups.json")?
            .set_default("engine.layout_file", "config/layout.json")?
            .set_default("engine.effects_file", "config/effects.json")?
            .set_default("engine.cue_lists_file", "config/cue_lists.json")?
            .set_default("artnet.target_ip", "2.0.0.1")?
            .set_default("artnet.universe", 0)?
            .build()?;

        s.try_deserialize()
    }
}
