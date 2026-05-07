use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use socketioxide::{SocketIo, extract::SocketRef};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tower_http::cors::CorsLayer;

use crate::engine::command::Cue;
use crate::store::cue_store::CueStore;
use crate::store::group_store::GroupStore;
use crate::store::layout_store::LayoutStore;
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
    settings: crate::settings::WebSettings,
    fixtures: Arc<HashMap<String, Arc<Fixture>>>,
    cue_store: Arc<RwLock<CueStore>>,
    group_store: Arc<RwLock<GroupStore>>,
    layout_store: Arc<RwLock<LayoutStore>>,
    cue_tx: crossbeam_channel::Sender<Cue>,
    dmx_data: Arc<RwLock<Vec<u8>>>,
) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (layer, io) = SocketIo::new_layer();

            let fixtures_clone = fixtures.clone();
            let dmx_data_clone = dmx_data.clone();
            let cue_store_clone = cue_store.clone();
            let group_store_clone = group_store.clone();
            let layout_store_clone = layout_store.clone();

            io.ns("/", move |socket: SocketRef| {
                let mut heads = Vec::new();
                let mut cues = Vec::new();
                let mut profiles_map = HashMap::new();

                let mut ordered_ids = Vec::new();
                {
                    let layout_lock = layout_store_clone.read().unwrap();
                    if let Some(layout) = layout_lock.get_default_layout() {
                        for row in &layout.heads {
                            for id in row {
                                if !id.is_empty() && !ordered_ids.contains(id) {
                                    ordered_ids.push(id.clone());
                                }
                            }
                        }
                    }
                }

                let mut unlisted: Vec<String> = fixtures_clone.keys()
                    .filter(|id| !ordered_ids.contains(id))
                    .cloned()
                    .collect();
                unlisted.sort();
                ordered_ids.extend(unlisted);

                for id in &ordered_ids {
                    if let Some(fix) = fixtures_clone.get(id) {
                        heads.push(json!({
                            "id": id,
                            "name": fix.name,
                            "address": fix.address,
                            "type": fix.profile.name
                        }));

                        if !profiles_map.contains_key(&fix.profile.name) {
                            let mut channels = Vec::new();
                            if let Some(mode) = fix.profile.modes.get(fix.mode_index) {
                                channels = mode.channels.clone();
                            }
                            profiles_map.insert(
                                fix.profile.name.clone(),
                                json!({
                                    "channels": channels
                                }),
                            );
                        }
                    }
                }

                for (id, cue) in cue_store_clone.read().unwrap().cues.iter() {
                    cues.push(json!({
                        "id": id,
                        "name": cue.name,
                    }));
                }

                let mut dmx_obj = serde_json::Map::new();
                for (i, &val) in dmx_data_clone.read().unwrap().iter().enumerate() {
                    dmx_obj.insert((i + 1).to_string(), json!(val));
                }

                let init_msg = json!({
                    "setup": {
                        "heads": heads,
                        "groups": group_store_clone.read().unwrap().groups.iter().map(|(id, g)| {
                            json!({"id": id, "name": g.name, "heads": g.heads})
                        }).collect::<Vec<_>>(),
                        "cues": cues
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
                .route(
                    "/console",
                    post(move |Json(payload): Json<ConsoleReq>| {
                        let cmd = payload.form.command;
                        if let Ok(Some(cue)) = crate::input::parser::parse_command_line(&cmd) {
                            let _ = cue_tx.send(cue);
                        }
                        async { "OK" }
                    }),
                )
                .layer(layer)
                .layer(CorsLayer::permissive());

            if let Ok(listener) = tokio::net::TcpListener::bind(&settings.bind_address).await {
                log::info!("Web server listening on {}", settings.bind_address);
                let _ = axum::serve(listener, app).await;
            } else {
                log::error!("Failed to bind web server to {}", settings.bind_address);
            }
        });
    });
}
