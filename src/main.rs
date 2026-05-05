mod engine;
mod input;
mod param;
mod timing;
mod output;

use std::thread;
use std::time::Duration;

use crate::engine::Engine;
use crate::timing::Timer;

fn main() {
    env_logger::init_from_env(env_logger::Env::new().filter_or("LOG_LEVEL", "debug"));

    let e = Engine::default();
    e.start();
   
    loop {
        thread::sleep(Duration::from_millis(200));
    }
}
