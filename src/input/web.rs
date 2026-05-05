use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use socketioxide::{SocketIo, extract::SocketRef};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use serde_json::json;
use std::collections::HashMap;

use crate::engine::command::Cue;
use crate::engine::fixture::Fixture;

#[derive(Deserialize)]
struct ConsoleReqForm {
    command: String,
}

#[derive(Deserialize)]
struct ConsoleReq {
    form: ConsoleReqForm,
}

pub fn start_web_server(
    fixtures: Arc<HashMap<String, Arc<Fixture>>>,
    cue_tx: crossbeam_channel::Sender<Cue>,
    dmx_data: Arc<RwLock<Vec<u8>>>,
) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (layer, io) = SocketIo::new_layer();
            
            let fixtures_clone = fixtures.clone();
            let dmx_data_clone = dmx_data.clone();
            
            io.ns("/", move |socket: SocketRef| {
                let mut heads = Vec::new();
                let mut profiles_map = HashMap::new();
                
                for (id, fix) in fixtures_clone.iter() {
                    heads.push(json!({
                        "id": id,
                        "address": fix.address,
                        "type": fix.profile.name
                    }));
                    
                    if !profiles_map.contains_key(&fix.profile.name) {
                        let mut channels = Vec::new();
                        if let Some(mode) = fix.profile.modes.get(fix.mode_index) {
                            channels = mode.channels.clone();
                        }
                        profiles_map.insert(fix.profile.name.clone(), json!({
                            "channels": channels
                        }));
                    }
                }
                
                let mut dmx_obj = serde_json::Map::new();
                for (i, &val) in dmx_data_clone.read().unwrap().iter().enumerate() {
                    dmx_obj.insert((i + 1).to_string(), json!(val));
                }

                let init_msg = json!({
                    "setup": {
                        "heads": heads,
                        "groups": [
                            {"name": "Top", "heads": ["F1", "F2", "F3", "F4"]},
                            {"name": "Bottom", "heads": ["F5", "F6", "F7", "F8"]}
                        ]
                    },
                    "profiles": profiles_map,
                    "dmx": dmx_obj
                });
                
                let _ = socket.emit("init", init_msg);
                
                let socket_clone = socket.clone();
                let dmx_data_task = dmx_data_clone.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        let mut update_obj = serde_json::Map::new();
                        for (i, &val) in dmx_data_task.read().unwrap().iter().enumerate() {
                            update_obj.insert((i + 1).to_string(), json!(val));
                        }
                        let _ = socket_clone.emit("update", json!(update_obj));
                    }
                });
            });

            let app = Router::new()
                .route("/console", post(move |Json(payload): Json<ConsoleReq>| {
                    let cmd = payload.form.command;
                    if let Some(cue) = crate::input::parser::parse_command_line(&cmd) {
                        let _ = cue_tx.send(cue);
                    }
                    async { "OK" }
                }))
                .layer(layer)
                .layer(CorsLayer::permissive());

            if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:8080").await {
                let _ = axum::serve(listener, app).await;
            }
        });
    });
}
