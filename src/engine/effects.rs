use std::time::Instant;
use std::sync::Arc;
use crate::param::FloatParam;
use super::command::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    Sine,
    Step,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MathMethod {
    Absolute,
    Add,
    Subtract,
    Multiply,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Fade {
        target_val: f32,
        duration: Duration,
    },
    Cycle {
        min_val: f32,
        max_val: f32,
        duration: Duration,
        easing: Easing,
        method: MathMethod,
    },
    Random {
        min_val: f32,
        max_val: f32,
        duration: Duration,
        easing: Easing,
        method: MathMethod,
    },
    Random0 {
        min_val: f32,
        max_val: f32,
        duration: Duration,
        easing: Easing,
        method: MathMethod,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RandomState {
    pub prev_val: f32,
    pub next_val: f32,
    pub last_period: u64,
    pub rng_state: u64,
}

impl RandomState {
    pub fn new(start_val: f32, min_val: f32, max_val: f32) -> Self {
        let mut rng_state = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
        if rng_state == 0 { rng_state = 1; }
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let frac = (rng_state as u32 as f32) / (std::u32::MAX as f32);
        let next_val = min_val + frac * (max_val - min_val);

        Self {
            prev_val: start_val,
            next_val,
            last_period: 0,
            rng_state,
        }
    }
    
    pub fn gen_next(&mut self, min_val: f32, max_val: f32) {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let frac = (self.rng_state as u32 as f32) / (std::u32::MAX as f32);
        self.next_val = min_val + frac * (max_val - min_val);
    }
}

pub struct ActiveEffect {
    pub param: Arc<FloatParam>,
    pub effect: Effect,
    pub start_time: Instant,
    pub start_beat: f32,
    pub start_val: f32,
    pub random_state: Option<RandomState>,
}

pub struct EffectManager {
    active_effects: Vec<ActiveEffect>,
}

impl EffectManager {
    pub fn new() -> Self {
        Self {
            active_effects: Vec::new(),
        }
    }

    pub fn add_effect(&mut self, param: Arc<FloatParam>, effect: Effect, start_beat: f32) {
        // Remove existing effects on this param
        self.active_effects.retain(|e| !Arc::ptr_eq(&e.param, &param));
        
        let start_val = param.load();
        let random_state = match &effect {
            Effect::Random { min_val, max_val, .. } => {
                Some(RandomState::new(start_val, *min_val, *max_val))
            }
            Effect::Random0 { min_val, max_val, .. } => {
                let mut state = RandomState::new(0.0, *min_val, *max_val);
                state.prev_val = state.next_val;
                state.next_val = 0.0;
                Some(state)
            }
            _ => None,
        };
        
        self.active_effects.push(ActiveEffect {
            start_val,
            param,
            effect,
            start_time: Instant::now(),
            start_beat,
            random_state,
        });
    }
    
    pub fn clear_effects_for(&mut self, param: &Arc<FloatParam>) {
        self.active_effects.retain(|e| !Arc::ptr_eq(&e.param, param));
    }

    pub fn tick(&mut self, current_beat: f32) {
        let now = Instant::now();
        let mut completed = Vec::new();

        for (i, active) in self.active_effects.iter_mut().enumerate() {
            match &active.effect {
                Effect::Fade { target_val, duration } => {
                    let progress = match duration {
                        Duration::Time(secs) => {
                            let elapsed = now.duration_since(active.start_time).as_secs_f32();
                            if *secs > 0.0 { elapsed / secs } else { 1.0 }
                        }
                        Duration::Tempo(beats) => {
                            let elapsed = current_beat - active.start_beat;
                            if *beats > 0.0 { elapsed / beats } else { 1.0 }
                        }
                    };

                    if progress >= 1.0 {
                        active.param.store(*target_val);
                        completed.push(i);
                    } else {
                        let new_val = active.start_val + (*target_val - active.start_val) * progress;
                        active.param.store(new_val);
                    }
                }
                Effect::Cycle { min_val, max_val, duration, easing, method } => {
                    let progress = match duration {
                        Duration::Time(secs) => {
                            let elapsed = now.duration_since(active.start_time).as_secs_f32();
                            if *secs > 0.0 { (elapsed / secs) % 1.0 } else { 0.0 }
                        }
                        Duration::Tempo(beats) => {
                            let elapsed = current_beat - active.start_beat;
                            if *beats > 0.0 { (elapsed / beats) % 1.0 } else { 0.0 }
                        }
                    };
                    
                    let phase = match easing {
                        Easing::Linear => {
                            if progress < 0.5 {
                                progress * 2.0
                            } else {
                                1.0 - (progress - 0.5) * 2.0
                            }
                        },
                        Easing::Sine => {
                            (std::f32::consts::PI * 2.0 * progress - std::f32::consts::PI / 2.0).sin() * 0.5 + 0.5
                        },
                        Easing::Step => {
                            if progress < 0.5 { 0.0 } else { 1.0 }
                        }
                    };
                    
                    let value = min_val + (max_val - min_val) * phase;
                    
                    let new_val = match method {
                        MathMethod::Absolute => value,
                        MathMethod::Add => active.start_val + value,
                        MathMethod::Subtract => active.start_val - value,
                        MathMethod::Multiply => active.start_val * (value / 255.0),
                    };
                    
                    active.param.store(new_val);
                }
                Effect::Random { min_val, max_val, duration, easing, method } 
                | Effect::Random0 { min_val, max_val, duration, easing, method } => {
                    let is_rnd0 = matches!(active.effect, Effect::Random0 { .. });
                    
                    let (progress, period_idx) = match duration {
                        Duration::Time(secs) => {
                            let elapsed = now.duration_since(active.start_time).as_secs_f32();
                            if *secs > 0.0 { ((elapsed / secs) % 1.0, (elapsed / secs).floor() as u64) } else { (0.0, 0) }
                        }
                        Duration::Tempo(beats) => {
                            let elapsed = current_beat - active.start_beat;
                            if *beats > 0.0 { ((elapsed / beats) % 1.0, (elapsed / beats).floor() as u64) } else { (0.0, 0) }
                        }
                    };
                    
                    if let Some(state) = &mut active.random_state {
                        if period_idx > state.last_period {
                            state.last_period = period_idx;
                            if is_rnd0 {
                                state.gen_next(*min_val, *max_val);
                                state.prev_val = state.next_val;
                                state.next_val = 0.0;
                            } else {
                                state.prev_val = state.next_val;
                                state.gen_next(*min_val, *max_val);
                            }
                        }
                        
                        let phase = match easing {
                            Easing::Linear => progress,
                            Easing::Sine => (std::f32::consts::PI * progress - std::f32::consts::PI / 2.0).sin() * 0.5 + 0.5,
                            Easing::Step => if progress < 0.5 { 0.0 } else { 1.0 },
                        };
                        
                        let value = state.prev_val + (state.next_val - state.prev_val) * phase;
                        
                        let new_val = match method {
                            MathMethod::Absolute => value,
                            MathMethod::Add => active.start_val + value,
                            MathMethod::Subtract => active.start_val - value,
                            MathMethod::Multiply => active.start_val * (value / 255.0),
                        };
                        
                        active.param.store(new_val);
                    }
                }
            }
        }

        for i in completed.into_iter().rev() {
            self.active_effects.remove(i);
        }
    }
}
