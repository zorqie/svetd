use std::{collections::HashMap, ops::{Deref, DerefMut}, sync::{Arc, LazyLock}};

use log::{debug, trace};

use crate::param::{FloatParam, PARAM_MAP, Param};

pub static PLAYBACKS: LazyLock<HashMap<String, Arc<Playback>>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for p in 1..30 {
        m.insert(format!("PB{p}"), Arc::new(Playback::new(format!("PB{p}"))));
    }
    m
});



#[derive(Debug, PartialEq, Clone)]
pub enum PlayCommand {
    At( f32),
    FlashOn, FlashOff,
    Go, Stop, Pause,
    Next, Prev,
}

#[derive(Debug, Clone)]
pub struct Playback {
    name: String,
    min: f32,
    max: f32,
    last: f32,
    pub param: Arc<FloatParam>,
    at_min: Option<PlayCommand>, // what to do when level goes down to zero, usually Stop
    above_min: Option<PlayCommand>, // what to do when level goes up above zero, usually Go
    at_max: Option<PlayCommand>, // what to do when level goes up to max, usually nothing
    below_max: Option<PlayCommand>, // what to do when level goes down below max, usually nothing
    priority: usize,
}


impl Playback {
    pub fn new(name: String) -> Self {
        let param = PARAM_MAP.lock().unwrap().create_param(&name);
        Self { 
            name: name.clone(), 
            min: 0.,
            max: 255.,
            last: 0.,
            priority: 0,
            at_min: Some(PlayCommand::Stop),
            above_min: Some(PlayCommand::Go),
            at_max: None,
            below_max: None,
            param,
        }
    }

    pub fn set_level(&mut self, level: f32) {
        &self.param.store(level);
    }

    pub fn level(&self) -> f32 {
        self.param.load()
    }

    pub fn levels(&self) -> Vec<(usize, u8)> {
        let mut vals = Vec::new();
        let v = self.level();
        for i in 0..3 {
            vals.push((i, v as u8));
        }
        vals
    }

    pub fn handle_cmd(&mut self, cmd: PlayCommand) {
        trace!("Got cmd {cmd:?}");
        match cmd {
            PlayCommand::At (level) => {
                self.param.store(level);
            },
            PlayCommand::FlashOn => {
                self.last = self.param.load();
                self.param.store(255.);
            },
            PlayCommand::FlashOff => {
                self.param.store(self.last);
            },
            _ => {},
        }
    }
}