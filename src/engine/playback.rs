use crate::store::cue_list_store::{CueListStore, CueListStepConfig};
use crate::store::cue_store::CueStore;
use crate::engine::command::{Cue, Duration};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Instant;
use crossbeam_channel::Sender;

pub struct ActiveCueList {
    pub list_name: String,
    pub step_index: usize,
    pub start_time: Instant,
    pub start_beat: f32,
    pub hold_duration: Option<Duration>,
}

pub struct PlaybackManager {
    active_lists: HashMap<String, ActiveCueList>,
    cue_list_store: Arc<RwLock<CueListStore>>,
    cue_store: Arc<RwLock<CueStore>>,
    cue_tx: Sender<Cue>,
}

impl PlaybackManager {
    pub fn new(
        cue_list_store: Arc<RwLock<CueListStore>>,
        cue_store: Arc<RwLock<CueStore>>,
        cue_tx: Sender<Cue>,
    ) -> Self {
        Self {
            active_lists: HashMap::new(),
            cue_list_store,
            cue_store,
            cue_tx,
        }
    }

    pub fn start_list(&mut self, list_name: &str, current_beat: f32) {
        let has_cues = {
            let store = self.cue_list_store.read().unwrap();
            store.lists.get(list_name).map_or(false, |l| !l.cues.is_empty())
        };
        if has_cues {
            self.trigger_step(list_name, 0, current_beat);
        }
    }

    pub fn stop_list(&mut self, list_name: &str) {
        if list_name == "all" {
            self.active_lists.clear();
        } else {
            self.active_lists.remove(list_name);
        }
    }

    fn trigger_step(&mut self, list_name: &str, step_index: usize, current_beat: f32) {
        let store = self.cue_list_store.read().unwrap();
        let list = store.lists.get(list_name).unwrap(); // safe because checked in caller
        
        let step = &list.cues[step_index];
        let (cue_id, hold_duration) = match step {
            CueListStepConfig::Simple(c) => (c.clone(), Some(Duration::Tempo(1.0))),
            CueListStepConfig::Detailed(d) => (d.cue.clone(), d.hold.clone()),
        };

        if let Some(cue) = self.cue_store.read().unwrap().cues.get(&cue_id) {
            let _ = self.cue_tx.send(cue.clone());
        }

        self.active_lists.insert(list_name.to_string(), ActiveCueList {
            list_name: list_name.to_string(),
            step_index,
            start_time: Instant::now(),
            start_beat: current_beat,
            hold_duration,
        });
    }

    pub fn tick(&mut self, current_beat: f32) {
        let now = Instant::now();
        let mut lists_to_advance = Vec::new();

        for (name, active) in &self.active_lists {
            if let Some(dur) = &active.hold_duration {
                let is_done = match dur {
                    Duration::Time(secs) => {
                        let elapsed = now.duration_since(active.start_time).as_secs_f32();
                        elapsed >= *secs
                    }
                    Duration::Tempo(beats) => {
                        let elapsed = current_beat - active.start_beat;
                        elapsed >= *beats
                    }
                };
                if is_done {
                    lists_to_advance.push((name.clone(), active.step_index + 1));
                }
            }
        }

        for (name, next_index) in lists_to_advance {
            let store = self.cue_list_store.read().unwrap();
            if let Some(list) = store.lists.get(&name) {
                let actual_index = if next_index >= list.cues.len() {
                    0 // loop by default for now
                } else {
                    next_index
                };
                drop(store);
                self.trigger_step(&name, actual_index, current_beat);
            }
        }
    }
}
