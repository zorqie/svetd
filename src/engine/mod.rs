pub mod command;
pub mod effects;
pub mod fixture;
pub mod patch;
pub mod playback;
pub mod profile;
pub mod target;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use log::debug;

use crate::store::cue_store::CueStore;
use crate::engine::effects::EffectManager;
use crate::engine::fixture::Fixture;
use crate::engine::profile::OflProfile;
use crate::{
    input,
    output::artnet::ArtNetSender,
    param::{FloatParam, Params},
    timing::Timer,
};

pub struct Engine {
    pub timing: Arc<RwLock<Timer>>,
    pub params: Params,
    pub fixtures: Arc<HashMap<String, Arc<Fixture>>>,
    pub effect_manager: Arc<RwLock<EffectManager>>,
    pub cue_store: Arc<RwLock<CueStore>>,
    pub cue_list_store: Arc<RwLock<crate::store::cue_list_store::CueListStore>>,
    pub preset_store: Arc<RwLock<crate::store::preset_store::PresetStore>>,
    pub group_store: Arc<RwLock<crate::store::group_store::GroupStore>>,
    pub layout_store: Arc<RwLock<crate::store::layout_store::LayoutStore>>,
    pub raw_channels: Arc<Vec<Arc<FloatParam>>>,
    pub settings: crate::settings::Settings,
}

impl Engine {
    pub fn new(settings: crate::settings::Settings) -> Self {
        let mut fixtures: HashMap<String, Arc<Fixture>> = HashMap::new();
        let mut profiles: HashMap<String, Arc<OflProfile>> = HashMap::new();

        if let Some(patch_store) =
            crate::engine::patch::PatchStore::load_from_file(&settings.engine.patch_file)
        {
            for (id, entry) in patch_store.fixtures {
                let profile_path =
                    format!("{}/{}.json", settings.engine.profiles_dir, entry.profile);

                let profile = profiles
                    .entry(entry.profile.clone())
                    .or_insert_with(|| {
                        Arc::new(
                            OflProfile::load(&profile_path)
                                .expect(&format!("Failed to load profile: {}", profile_path)),
                        )
                    })
                    .clone();

                fixtures.insert(
                    id,
                    Arc::new(Fixture::new(
                        &entry.name,
                        entry.address,
                        profile,
                        entry.mode,
                    )),
                );
            }
        } else {
            log::warn!("Could not load {}", settings.engine.patch_file);
        }

        let mut raw_channels = Vec::with_capacity(512);
        for _ in 0..512 {
            raw_channels.push(Arc::new(crate::param::FloatParam::new_with_time(0.0, 0)));
        }

        Self {
            timing: Arc::new(RwLock::new(Timer::default())),
            params: Params::new(),
            fixtures: Arc::new(fixtures),
            effect_manager: Arc::new(RwLock::new(EffectManager::new())),
            // Load EffectStore first so parser state gets populated
            // with interpreted effects for CueStore to use.
            // We don't necessarily need to store it in Engine since it's in ParserState.
            // But we keep it in the Engine if needed later.
            
            cue_store: {
                let _ = crate::store::effect_store::EffectStore::load_from_file(&settings.engine.effects_file);
                Arc::new(RwLock::new(CueStore::load_from_file(
                    &settings.engine.cues_file,
                )))
            },
            cue_list_store: Arc::new(RwLock::new(
                crate::store::cue_list_store::CueListStore::load_from_file(&settings.engine.cue_lists_file)
                    .unwrap_or_default()
            )),
            preset_store: Arc::new(RwLock::new(crate::store::preset_store::PresetStore::load_from_file(
                &settings.engine.presets_file,
            ))),
            group_store: Arc::new(RwLock::new(
                crate::store::group_store::GroupStore::load_from_file(&settings.engine.groups_file)
                    .unwrap_or_default()
            )),
            layout_store: Arc::new(RwLock::new(
                crate::store::layout_store::LayoutStore::load_from_file(&settings.engine.layout_file)
                    .unwrap_or_default()
            )),
            raw_channels: Arc::new(raw_channels),
            settings,
        }
    }

    pub fn start(&self) {
        let data = Arc::new(RwLock::new(vec![0u8; 512]));

        let data_clone = Arc::clone(&data);
        let a = ArtNetSender::new(
            &self.settings.artnet.target_ip,
            self.settings.artnet.universe.into(),
        );
        a.start(data_clone);

        let data_clone = Arc::clone(&data);
        let fixtures = self.fixtures.clone();
        let effect_manager = self.effect_manager.clone();
        let timing = self.timing.clone();
        let raw_channels = self.raw_channels.clone();

        let (cue_tx, cue_rx) = crossbeam_channel::unbounded::<crate::engine::command::Cue>();

        let playback_manager = Arc::new(RwLock::new(playback::PlaybackManager::new(
            self.cue_list_store.clone(),
            self.cue_store.clone(),
            cue_tx.clone(),
        )));

        let playback_manager_exec = playback_manager.clone();

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            loop {
                let beat_progression = {
                    let mut t = timing.write().unwrap();
                    t.tick();
                    t.beat_progression()
                };

                playback_manager_exec.write().unwrap().tick(beat_progression);
                effect_manager.write().unwrap().tick(beat_progression);

                {
                    let mut writable_data = data_clone.write().unwrap();
                    let mut fixture_data = [0u8; 512];
                    let mut fixture_times = [0u64; 512];

                    for f in fixtures.values() {
                        f.render(&mut fixture_data, &mut fixture_times);
                    }

                    for i in 0..512 {
                        let raw_time = raw_channels[i].last_updated();
                        if raw_time > fixture_times[i] {
                            writable_data[i] = raw_channels[i].load() as u8;
                        } else {
                            writable_data[i] = fixture_data[i];
                        }
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
        });

        crate::input::cli::start_cli_thread(cue_tx.clone());
        crate::input::web::start_web_server(
            self.settings.web.clone(),
            self.fixtures.clone(),
            self.cue_store.clone(),
            self.group_store.clone(),
            self.layout_store.clone(),
            cue_tx.clone(),
            Arc::clone(&data),
        );

        let osc_rx = input::osc::start_osc_thread(self.settings.osc.clone());

        thread::spawn(move || {
            loop {
                if let Ok(Some(cmd)) = osc_rx.recv() {
                    let target_str = match &cmd {
                        input::osc::OscCommand::At { target, .. } => target.clone(),
                        input::osc::OscCommand::On { target } => target.clone(),
                        input::osc::OscCommand::Off { target } => target.clone(),
                    };

                    if target_str.starts_with("PB") {
                        if let Ok(num) = target_str[2..].parse::<u16>() {
                            let target =
                                crate::engine::target::Target::Fixtures(vec![format!("F{}", num)]);
                            let internal_cmd = match cmd {
                                input::osc::OscCommand::At { level, .. } => {
                                    crate::engine::command::Command::SetLevel {
                                        target,
                                        value: crate::engine::command::Value::Numeric(level),
                                    }
                                }
                                input::osc::OscCommand::On { .. } => {
                                    crate::engine::command::Command::SetLevel {
                                        target,
                                        value: crate::engine::command::Value::Numeric(255.0),
                                    }
                                }
                                input::osc::OscCommand::Off { .. } => {
                                    crate::engine::command::Command::SetLevel {
                                        target,
                                        value: crate::engine::command::Value::Numeric(0.0),
                                    }
                                }
                            };
                            let _ = cue_tx
                                .send(crate::engine::command::Cue::new("OSC", vec![internal_cmd]));
                        }
                    }
                }
            }
        });

        let fixtures_exec = self.fixtures.clone();
        let effect_manager_exec = self.effect_manager.clone();
        let timing_exec = self.timing.clone();
        let raw_channels_exec = self.raw_channels.clone();
        let preset_store_exec = self.preset_store.clone();
        let group_store_exec = self.group_store.clone();
        let playback_manager_cmd = playback_manager.clone();
        let timing_exec2 = self.timing.clone();

        thread::spawn(move || {
            loop {
                if let Ok(cue) = cue_rx.recv() {
                    debug!("Got cue: {:?}", cue);
                    let presets = preset_store_exec.read().unwrap();
                    for cmd in cue.commands {
                        match &cmd {
                            crate::engine::command::Command::StartCueList { list } => {
                                let beat = timing_exec2.read().unwrap().beat_progression();
                                playback_manager_cmd.write().unwrap().start_list(list, beat);
                                continue;
                            }
                            crate::engine::command::Command::StopCueList { list } => {
                                playback_manager_cmd.write().unwrap().stop_list(list);
                                continue;
                            }
                            _ => {}
                        }

                        let targets = match &cmd {
                            crate::engine::command::Command::SetLevel { target, .. } => target,
                            crate::engine::command::Command::SetColor { target, .. } => target,
                            crate::engine::command::Command::SetPosition { target, .. } => target,
                            crate::engine::command::Command::SetStrobe { target, .. } => target,
                            crate::engine::command::Command::StartEffect { target, .. } => target,
                            crate::engine::command::Command::StopEffect { target } => target,
                            _ => continue,
                        };

                        let mut params = Vec::new();
                        let mut targeted_fixtures = Vec::new();
                        let mut raw_params_to_set = Vec::new();

                        fn resolve_target(
                            t: &crate::engine::target::Target,
                            params: &mut Vec<Arc<crate::param::FloatParam>>,
                            targeted_fixtures: &mut Vec<Arc<crate::engine::fixture::Fixture>>,
                            raw_params_to_set: &mut Vec<Arc<crate::param::FloatParam>>,
                            fixtures_exec: &Arc<std::collections::HashMap<String, Arc<crate::engine::fixture::Fixture>>>,
                            raw_channels_exec: &Arc<Vec<Arc<crate::param::FloatParam>>>,
                            group_store_exec: &Arc<RwLock<crate::store::group_store::GroupStore>>,
                        ) {
                            match t {
                                crate::engine::target::Target::Channels(chs) => {
                                    for ch in &chs.0 {
                                        if *ch > 0 && *ch <= 512 {
                                            let p = raw_channels_exec[(*ch - 1) as usize].clone();
                                            raw_params_to_set.push(p.clone());
                                            params.push(p);
                                        }
                                    }
                                }
                                crate::engine::target::Target::Fixtures(fxs) => {
                                    for f in fxs {
                                        if let Some(fix) = fixtures_exec.get(f) {
                                            targeted_fixtures.push(fix.clone());
                                            if let Some(p) = fix.parameters.get("Dimmer") {
                                                params.push(p.clone());
                                            }
                                        }
                                    }
                                }
                                crate::engine::target::Target::Groups(groups) => {
                                    let store = group_store_exec.read().unwrap();
                                    for g_name in groups {
                                        let mut found_heads = Vec::new();
                                        for (id, group) in &store.groups {
                                            if id.eq_ignore_ascii_case(g_name) || group.name.eq_ignore_ascii_case(g_name) {
                                                found_heads.extend(group.heads.clone());
                                                break;
                                            }
                                        }
                                        for fx_name in found_heads {
                                            if let Some(fix) = fixtures_exec.get(&fx_name) {
                                                targeted_fixtures.push(fix.clone());
                                                if let Some(p) = fix.parameters.get("Dimmer") {
                                                    params.push(p.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                                crate::engine::target::Target::Mixed(mixed) => {
                                    for inner in mixed {
                                        resolve_target(inner, params, targeted_fixtures, raw_params_to_set, fixtures_exec, raw_channels_exec, group_store_exec);
                                    }
                                }
                                _ => {}
                            }
                        }

                        resolve_target(targets, &mut params, &mut targeted_fixtures, &mut raw_params_to_set, &fixtures_exec, &raw_channels_exec, &group_store_exec);

                        match cmd {
                            crate::engine::command::Command::SetLevel { ref value, .. } => {
                                let level = match value {
                                    crate::engine::command::Value::Numeric(n) => *n,
                                    crate::engine::command::Value::Semantic(_) => 255.0,
                                };
                                let mut em = effect_manager_exec.write().unwrap();
                                for fix in &targeted_fixtures {
                                    if let Some(p) = fix.parameters.get("Dimmer") {
                                        em.clear_effects_for(p);
                                    }
                                    fix.handle_command(&cmd, &presets);
                                }
                                for p in &raw_params_to_set {
                                    em.clear_effects_for(p);
                                    p.store(level);
                                }
                            }
                            crate::engine::command::Command::SetColor { .. } => {
                                let mut em = effect_manager_exec.write().unwrap();
                                for fix in &targeted_fixtures {
                                    if let Some(p) = fix.parameters.get("Red") {
                                        em.clear_effects_for(p);
                                    }
                                    if let Some(p) = fix.parameters.get("Green") {
                                        em.clear_effects_for(p);
                                    }
                                    if let Some(p) = fix.parameters.get("Blue") {
                                        em.clear_effects_for(p);
                                    }
                                    if let Some(p) = fix.parameters.get("White") {
                                        em.clear_effects_for(p);
                                    }
                                    fix.handle_command(&cmd, &presets);
                                }
                            }
                            crate::engine::command::Command::SetPosition { .. } => {
                                let mut em = effect_manager_exec.write().unwrap();
                                for fix in &targeted_fixtures {
                                    if let Some(p) = fix.parameters.get("Pan") {
                                        em.clear_effects_for(p);
                                    }
                                    if let Some(p) = fix.parameters.get("Tilt") {
                                        em.clear_effects_for(p);
                                    }
                                    fix.handle_command(&cmd, &presets);
                                }
                            }
                            crate::engine::command::Command::SetStrobe { .. } => {
                                for fix in &targeted_fixtures {
                                    fix.handle_command(&cmd, &presets);
                                }
                            }
                            crate::engine::command::Command::StartEffect { effect, .. } => {
                                let start_beat = timing_exec.read().unwrap().beat_progression();
                                for param in &params {
                                    effect_manager_exec.write().unwrap().add_effect(
                                        param.clone(),
                                        effect.clone(),
                                        start_beat,
                                    );
                                }
                            }
                            crate::engine::command::Command::StopEffect { .. } => {
                                for param in &params {
                                    effect_manager_exec
                                        .write()
                                        .unwrap()
                                        .clear_effects_for(param);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}
