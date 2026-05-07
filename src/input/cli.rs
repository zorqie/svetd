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
                match parse_command_line(&line) {
                    Ok(Some(cue)) => {
                        let _ = tx.send(cue);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        println!("Unrecognized command format.");
                    }
                }
            }
        }
    });
}
