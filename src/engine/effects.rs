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
}

pub struct ActiveEffect {
    pub param: Arc<FloatParam>,
    pub effect: Effect,
    pub start_time: Instant,
    pub start_beat: f32,
    pub start_val: f32,
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
        
        self.active_effects.push(ActiveEffect {
            start_val: param.load(),
            param,
            effect,
            start_time: Instant::now(),
            start_beat,
        });
    }
    
    pub fn clear_effects_for(&mut self, param: &Arc<FloatParam>) {
        self.active_effects.retain(|e| !Arc::ptr_eq(&e.param, param));
    }

    pub fn tick(&mut self, current_beat: f32) {
        let now = Instant::now();
        let mut completed = Vec::new();

        for (i, active) in self.active_effects.iter().enumerate() {
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
            }
        }

        for i in completed.into_iter().rev() {
            self.active_effects.remove(i);
        }
    }
}
