use std::io::{self, BufRead};
use std::thread;
use crossbeam_channel::Sender;
use crate::engine::command::Cue;
use super::parser::parse_command_line;

pub fn start_cli_thread(tx: Sender<Cue>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(line) = line {
                if let Some(cue) = parse_command_line(&line) {
                    let _ = tx.send(cue);
                } else {
                    println!("Unrecognized command format.");
                }
            }
        }
    });
}
