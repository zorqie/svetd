use std::sync::{Arc, RwLock};


pub struct Cue {
    pub heads: Arc<RwLock<Vec<Head>>>,
}

impl Cue {
    pub fn new() -> Self {
        Self {
            heads: Arc::new(RwLock::new(Vec::new())),
        }
    }
    pub fn levels(&self) -> Vec<(usize, u8)> {
        let mut out = Vec::new();
        for h in self.heads.read().unwrap().iter() {
            out.append(&mut h.levels());
        }
        out
    }
}

#[derive(Default, Debug)]
pub struct Head {
    start_ch: usize,
}

impl Head {
    pub fn patch(mut self, ch: usize) {
        self.start_ch = ch;
    }
    pub fn levels(&self) -> Vec<(usize, u8)> {
        let v = vec![
            (self.start_ch + 0, 0u8), 
            (self.start_ch + 1, 64u8), 
            (self.start_ch + 2, 128u8)];
        v
    }
}