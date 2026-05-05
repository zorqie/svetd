use std::{
    net::UdpSocket,
    sync::{Arc, LazyLock},
    thread,
};

use crossbeam_channel::Receiver;
use log::debug;
use rosc::{OscPacket, OscType};

use crate::param::playback::PlayCommand;

static OSC_PORT: u16 = 8001;
pub static OSC_SOCKET: LazyLock<Arc<UdpSocket>> = LazyLock::new(|| {
    Arc::new(UdpSocket::bind(("127.0.0.1", OSC_PORT)).expect("Could not bind on OSC port"))
});

#[derive(Debug, PartialEq, Clone)]
pub enum OscCommand {
    At { level: f32, target: String },
    // PressCmd {
    //     target: String,
    // },
    On { target: String },
    Off { target: String },
}

pub fn start_osc_thread() -> Receiver<Option<OscCommand>> {
    debug!("Starting OSC listener: {:?}", OSC_SOCKET.local_addr());

    let (osc_tx, osc_rx) = crossbeam_channel::unbounded::<Option<OscCommand>>();
    // let rx_clone = osc_rx.clone(); //NEVER clone Receivers, it's useless. Messages are consumed!!!
    // thread::spawn(move || {
    //     loop {
    //         if let Ok(Some(cmd)) = rx_clone.recv() {
    //             debug!("OSC: {cmd:?}");
    //             handle_cmd(cmd);
    //         }
    //     }
    // });

    let mut buf = [0u8; rosc::decoder::MTU];

    thread::spawn(move || {
        loop {
            match OSC_SOCKET.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                    let cmd = handle_packet(packet);
                    let _ = osc_tx.send(cmd); //TODO deal with error
                }
                Err(e) => {
                    println!("Error receiving from socket: {}", e);
                    break;
                }
            }
        }
    });

    osc_rx
}

fn handle_packet(packet: OscPacket) -> Option<OscCommand> {
    match packet {
        OscPacket::Message(msg) => {
            // println!("OSC: {}", msg);
            let addr: Vec<&str> = msg.addr.split("/").collect();
            if addr[1] == "LS" {
                let cmd: Option<OscCommand> = match (addr.len(), addr[2]) {
                    (3, "DBO") => {
                        let OscType::Float(on) = msg.args[0] else {
                            panic!("Unexpected.")
                        };
                        match on {
                            1. => Some(OscCommand::On {
                                target: format!("{}", addr[2]),
                            }),
                            0. => Some(OscCommand::Off {
                                target: format!("{}", addr[2]),
                            }),
                            _ => None,
                        }
                    }
                    (4, "Level") => {
                        let level = match msg.args[0] {
                            rosc::OscType::Float(f) => f,
                            _ => 0.,
                        };
                        debug!("{}@{}", addr[3], level);
                        Some(OscCommand::At {
                            level,
                            target: addr[3].to_string(),
                        })
                    }
                    (5, "Level") => {
                        let OscType::Float(level) = msg.args[0] else {
                            panic!("What are we doing here?")
                        };
                        debug!("PB{}@{:?}", addr[4], level);
                        Some(OscCommand::At {
                            level,
                            target: format!("{}{}", addr[3], addr[4]),
                        })
                    }
                    (5, "Flash") => {
                        let OscType::Float(on) = msg.args[0] else {
                            panic!("Unexpected.")
                        };
                        match on {
                            1. => Some(OscCommand::On {
                                target: format!("{}{}", addr[3], addr[4]),
                            }),
                            0. => Some(OscCommand::Off {
                                target: format!("{}{}", addr[3], addr[4]),
                            }),
                            _ => None,
                        }
                    }
                    (6, "Executor") => {
                        let OscType::Float(on) = msg.args[0] else {
                            panic!("Unexpected.")
                        };
                        match on {
                            1. => Some(OscCommand::On {
                                target: format!("Exec {}.{}.{}", addr[3], addr[4], addr[5]),
                            }),
                            0. => Some(OscCommand::Off {
                                target: format!("Exec {}.{}.{}", addr[3], addr[4], addr[5]),
                            }),
                            _ => None,
                        }
                    }
                    (_, _) => {
                        debug!("->{addr:?}");
                        None
                    }
                };
                // if let Some(osc_cmd) = cmd {
                //     debug!("Commanded: {osc_cmd:?}");
                // }
                cmd
            } else {
                None
            }
            // println!("OSC: {} --> {:?}", msg.addr, msg.args);
        }
        OscPacket::Bundle(bundle) => {
            println!("OSC Bundle: {:?}", bundle);
            None
        }
    }
}

// Deleted obsolete handle_osc_cmd
