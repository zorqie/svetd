#![allow(unused)]
pub mod playback;

use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicIsize, AtomicU8, AtomicU32, Ordering},
    },
};

pub struct Param(Arc<AtomicIsize>);

impl Param {
    pub fn new() -> Self {
        Self(Arc::new(AtomicIsize::new(0)))
    }
    pub fn value(&self) -> isize {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct Params {
    map: HashMap<String, Arc<FloatParam>>,
}

impl Params {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn create_param(&mut self, name: &str) -> Arc<FloatParam> {
        let param = Arc::new(FloatParam::new(0.));
        self.map.insert(name.to_string(), param.clone());
        param
    }
    pub fn insert(&mut self, name: String, param: FloatParam) {
        self.map.insert(name, Arc::new(param));
    }
    pub fn get(&self, name: String) -> Option<Arc<FloatParam>> {
        self.map.get(&name).cloned()
    }
}

pub static PARAM_MAP: LazyLock<Mutex<Params>> = LazyLock::new(|| Mutex::new(Params::new()));

// pub static PARAMS: LazyLock<HashMap<String, Arc<FloatParam>>> = LazyLock::new(|| {
//     let mut m = HashMap::new();
//     for p in 1..30 {
//         m.insert(format!("PB{p}"), Arc::new(FloatParam::new(0.)));
//     }
//     m
// });

#[derive(Debug)]
pub struct FloatParam {
    val: AtomicU32,
    last_updated: std::sync::atomic::AtomicU64,
}

impl FloatParam {
    pub fn new(value: f32) -> Self {
        Self::new_with_time(value, Self::now_micros())
    }
    
    pub fn new_with_time(value: f32, time: u64) -> Self {
        let as_u32 = value.to_bits();
        Self { 
            val: AtomicU32::new(as_u32),
            last_updated: std::sync::atomic::AtomicU64::new(time),
        }
    }

    fn now_micros() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_micros() as u64
    }

    pub fn store(&self, value: f32) {
        let as_u32 = value.to_bits();
        self.val.store(as_u32, Ordering::SeqCst);
        self.last_updated.store(Self::now_micros(), Ordering::SeqCst);
    }

    pub fn load(&self) -> f32 {
        let as_u32 = self.val.load(Ordering::SeqCst);
        f32::from_bits(as_u32)
    }

    pub fn last_updated(&self) -> u64 {
        self.last_updated.load(Ordering::SeqCst)
    }

    pub fn load_u8(&self) -> u8 {
        self.load() as u8
    }
}
