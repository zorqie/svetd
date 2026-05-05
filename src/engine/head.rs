use std::{collections::HashMap, sync::Arc};

use crate::param::FloatParam;

pub enum FixtureTraitType {
    Dimmer,
    Color,
    Pan,
    Tilt,

    Strobe,
    Gobo,
    Iris,
    Zoom,
    Prism,
    Control,
}

pub enum FixtureColor {
    None,
    Named(String),
    Rgb(f32, f32, f32),
    Wheel(u8),
    Cto(f32),
}
pub enum FixtureStrobe {
    None,
    Open,
    Speed(f32),
    Random(f32),
}

pub struct FixturePositon {
    x: f32,
    y: f32,
    z: f32,
}

pub enum FixtureCommand {
    Dimmer(f32),
    Color(FixtureColor),
    Pan(f32),
    Tilt(f32),
    Position(FixturePositon),
    Strobe(FixtureStrobe),
}

struct Head {
    // profile: Profile,
    patch: usize, // starting channel if patched. u16 gives us 128 DMX universes,
    // should be enough but indexing
    intensity: FloatParam,
    traits: HashMap<FixtureTraitType, Arc<dyn Trait>>,
}

pub trait Trait {
    fn at(self, level: f32);
    fn value(&self) -> f32;
}

pub trait Dimmer: Trait {
    fn dimmer(&self) -> f32;
}

impl Trait for Head {
    fn at(self, level: f32) {
        self.intensity.store(level);
    }

    fn value(&self) -> f32 {
        self.intensity.load()
    }
}

impl Dimmer for Head {
    fn dimmer(&self) -> f32 {
        self.intensity.load()
    }
}
