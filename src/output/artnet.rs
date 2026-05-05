use std::{net::UdpSocket, sync::{Arc, LazyLock, RwLock}, thread, time::Duration};

use log::{debug, warn};


static ARTNET_PORT: u16 = 6454;
pub static ARTNET_SOCKET: LazyLock<Arc<UdpSocket>> = LazyLock::new(|| {
    Arc::new(
        UdpSocket::bind(("2.0.0.12", ARTNET_PORT))
        .expect("Could not bind on artnet port")
    )
});

// const ROWS : usize = 1;
// const COLS : usize = 32;

// fn f(x: usize, _y: usize, t: usize) -> u8 {
//     (((1+x*2) + t  % (ROWS * COLS))) as u8
// }

pub struct ArtNetSender {
    send_addr: String, 
    universe: i32,
}

impl ArtNetSender {
    pub fn new(send_addr: &str, universe: i32) -> Self {
        Self { 
            send_addr: send_addr.to_string(),
            universe 
        }
    }


    // TODO use self.socket!!!
    pub fn start(&self, data: Arc<RwLock<Vec<u8>>>) {
        match ARTNET_SOCKET.set_broadcast(true) {
            Ok(_) => debug!("Activated sending to broadcast"),
            Err(e) => debug!("Could not activate sending to broadcast: {e}"),
        }
        match ARTNET_SOCKET.set_nonblocking(true) {
            Ok(_) => debug!("Activated non-blocking mode"),
            Err(e) => debug!("Could not activate non-blocking mode: {e}"),
        };
        
        let addr = format!("{}:{}", self.send_addr, ARTNET_PORT);
        let universe = self.universe;
        thread::spawn(move || {
            let mut count = 0;

            loop {
                count += 1;
                
                let Ok(port_address) = artnet_protocol::PortAddress::try_from(universe) else {
                    warn!("Could not convert universe {} to port address", universe);
                    continue;
                };
                {
                    let readable_data = data.read().unwrap(); // Acquire a read lock
                    let output = artnet_protocol::Output {
                        data: artnet_protocol::PaddedData::from(readable_data.to_vec()),
                        port_address,
                        ..Default::default()
                    };
                    
                    let Ok(data) = artnet_protocol::ArtCommand::Output(output).write_to_buffer() else {
                        warn!("Could not create artnet output command for universe {}", universe);
                        continue;
                    };
                    
                    match ARTNET_SOCKET.send_to(&data, &addr) {
                        Err(err) if count == 10 => {
                            warn!("Could not send data after {count} tries, giving up - {err:?}");
                            break;
                        }
                        Err(err) => {
                            warn!("Could not send data on try {count} - {err:?}");
                            thread::sleep(Duration::from_nanos(1)); 
                        }
                        Ok(count) if count == 100 => {
                            debug!("Sent to {addr}: {:?}", &data);
                            // break;
                        }
                        _ => {}
                    };
                    // Lock is automatically released when readable_data goes out of scope
                }
                thread::sleep(Duration::from_millis(24)); //24ms approx= 40Hz
                
            }
        });
    }
}

/*
pub fn art_send() {
    let mut data = vec![0u8; 512];
    let addr ="2.0.0.1:6454";
    let universe = 0;

    thread::spawn(move || {
        let mut count = 0;
        match ARTNET_SOCKET.set_broadcast(true) {
            Ok(_) => debug!("Activated sending to broadcast"),
            Err(e) => debug!("Could not activate sending to broadcast: {e}"),
        }
        match ARTNET_SOCKET.set_nonblocking(true) {
            Ok(_) => debug!("Activated non-blocking mode"),
            Err(e) => debug!("Could not activate non-blocking mode: {e}"),
        };

        loop {
            for x in 0..COLS {
                for y in 0..ROWS {
                    let v = if let Some(p) = PARAMS.get(&format!("PB{}", 1+x%8)) {
                        p.load_u8()
                    } else {
                        0u8
                    };
                    data[(y * COLS + x) * 3] = v;
                }
            }
            // data[96 + count % 16] = data[96 + count % 16].wrapping_add((count * 16) as u8);
            count += 1;


            let Ok(port_address) = artnet_protocol::PortAddress::try_from(universe) else {
                warn!("Could not convert universe {universe} to port address");
                continue;
            };

            let output = artnet_protocol::Output {
                data: artnet_protocol::PaddedData::from(data.to_vec()),
                port_address,
                ..Default::default()
            };
            
            let Ok(data) = artnet_protocol::ArtCommand::Output(output).write_to_buffer() else {
                warn!("Could not create artnet output command for universe {universe}");
                continue;
            };
            
            match ARTNET_SOCKET.send_to(&data, addr) {
                Err(err) if count == 10 => {
                    warn!("Could not send data after {count} tries, giving up - {err:?}");
                    break;
                }
                Err(err) => {
                    warn!("Could not send data on try {count} - {err:?}");
                    thread::sleep(Duration::from_nanos(1)); 
                }
                Ok(count) => {
                    trace!("Sent {count} bytes to {addr}");
                    // break;
                }
            };
            thread::sleep(Duration::from_millis(24));
        }
    });
}

*/