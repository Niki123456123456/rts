use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize)]
pub struct Snapshot {
    pub connected: bool,
    pub ready: bool,
    pub error: Option<String>,
    #[serde(default, deserialize_with = "optional_id")]
    pub game_id: Option<u64>,
    pub games: Vec<Game>,
    pub units: Vec<Unit>,
}

#[derive(Clone, Deserialize)]
pub struct Game {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub title: String,
}

#[derive(Clone, Deserialize)]
pub struct Unit {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub number: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// IDs cross the JavaScript bridge as strings to preserve all 64 bits.
fn id<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    String::deserialize(deserializer)?
        .parse()
        .map_err(serde::de::Error::custom)
}

fn optional_id<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|value| value.parse().map_err(serde::de::Error::custom))
        .transpose()
}

fn serialize_id<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

fn serialize_ids<S: serde::Serializer>(values: &[u64], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(values.iter().map(u64::to_string))
}

#[derive(Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Connect {
        uri: String,
        database: String,
    },
    Create {
        title: String,
    },
    Join {
        #[serde(serialize_with = "serialize_id")]
        game_id: u64,
    },
    Leave,
    Targets {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        x: f32,
        z: f32,
    },
}

#[derive(Resource)]
pub struct Multiplayer {
    pub snapshot: Snapshot,
    pub uri: String,
    pub database: String,
    pub title: String,
    pub pending: bool,
    backend: backend::Backend,
    local_error: Option<String>,
}

impl Default for Multiplayer {
    fn default() -> Self {
        Self {
            snapshot: Snapshot::default(),
            uri: option_env!("SPACETIMEDB_URI")
                .unwrap_or("https://maincloud.spacetimedb.com")
                .into(),
            database: option_env!("SPACETIMEDB_DATABASE")
                .unwrap_or("stronghold-niki123456123456")
                .into(),
            title: String::new(),
            pending: false,
            backend: backend::Backend::default(),
            local_error: None,
        }
    }
}

impl Multiplayer {
    pub fn send(&mut self, action: Action) -> bool {
        self.snapshot.error = None;
        self.local_error = None;
        let awaiting_game = matches!(&action, Action::Create { .. } | Action::Join { .. });
        match self.backend.send(action) {
            Ok(()) => {
                self.pending = awaiting_game;
                true
            }
            Err(error) => {
                self.local_error = Some(error.clone());
                self.snapshot.error = Some(error);
                self.pending = false;
                false
            }
        }
    }

    pub fn poll(&mut self) {
        self.snapshot = self.backend.poll();
        if let Some(error) = &self.local_error {
            self.snapshot.error = Some(error.clone());
        }
        if self.snapshot.error.is_some()
            || self.snapshot.game_id.is_some()
            || !self.snapshot.connected
        {
            self.pending = false;
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = strongholdNetwork, js_name = snapshot)]
        fn snapshot() -> String;
        #[wasm_bindgen(catch, js_namespace = strongholdNetwork, js_name = send)]
        fn send_action(action: &str) -> Result<(), JsValue>;
    }

    #[derive(Default)]
    pub struct Backend;

    impl Backend {
        pub fn send(&mut self, action: Action) -> Result<(), String> {
            send_action(&serde_json::to_string(&action).map_err(|e| e.to_string())?)
                .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
        }

        pub fn poll(&mut self) -> Snapshot {
            serde_json::from_str(&snapshot()).unwrap_or_else(|e| Snapshot {
                error: Some(format!("Invalid network state: {e}")),
                ..default()
            })
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use crate::module_bindings::*;
    use spacetimedb_sdk::{DbContext, SubscriptionHandle as _, Table};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Status {
        connected: bool,
        ready: bool,
        error: Option<String>,
    }

    #[derive(Default)]
    pub struct Backend {
        connection: Option<DbConnection>,
        status: Arc<Mutex<Status>>,
        game_id: Option<u64>,
        units: Option<SubscriptionHandle>,
    }

    impl Backend {
        pub fn send(&mut self, action: Action) -> Result<(), String> {
            if let Action::Connect { uri, database } = action {
                if let Some(conn) = self.connection.take() {
                    let _ = conn.disconnect();
                    let _ = conn.frame_tick();
                }
                self.units = None;
                self.game_id = None;
                self.status = Arc::default();
                let connected = self.status.clone();
                let applied = self.status.clone();
                let sub_error = self.status.clone();
                let disconnected = self.status.clone();
                let connect_error = self.status.clone();
                let conn = DbConnection::builder()
                    .with_uri(uri)
                    .with_module_name(database)
                    .on_connect(move |conn, _, _| {
                        connected.lock().unwrap().connected = true;
                        conn.subscription_builder()
                            .on_applied(move |_| applied.lock().unwrap().ready = true)
                            .on_error(move |_, error| {
                                sub_error.lock().unwrap().error = Some(error.to_string())
                            })
                            .subscribe(["SELECT * FROM game", "SELECT * FROM player"]);
                    })
                    .on_connect_error(move |_, error| {
                        connect_error.lock().unwrap().error = Some(error.to_string());
                    })
                    .on_disconnect(move |_, error| {
                        let mut status = disconnected.lock().unwrap();
                        status.connected = false;
                        status.ready = false;
                        status.error = Some(
                            error
                                .map(|e| e.to_string())
                                .unwrap_or("Disconnected from server.".into()),
                        );
                    })
                    .build()
                    .map_err(|e| e.to_string())?;
                let errors = self.status.clone();
                conn.reducers
                    .on_create_game(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_join_game(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_leave_game(move |ctx| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_set_targets(move |ctx, _, _, _| record_status(&errors, ctx));
                self.connection = Some(conn);
                return Ok(());
            }
            self.status.lock().unwrap().error = None;
            let conn = self
                .connection
                .as_ref()
                .ok_or("Connect to a server first.")?;
            match action {
                Action::Create { title } => conn.reducers.create_game(title),
                Action::Join { game_id } => conn.reducers.join_game(game_id),
                Action::Leave => conn.reducers.leave_game(),
                Action::Targets { unit_ids, x, z } => conn.reducers.set_targets(unit_ids, x, z),
                Action::Connect { .. } => unreachable!(),
            }
            .map_err(|e| e.to_string())
        }

        pub fn poll(&mut self) -> Snapshot {
            let Some(conn) = &self.connection else {
                return Snapshot::default();
            };
            if let Err(error) = conn.frame_tick() {
                let mut status = self.status.lock().unwrap();
                status.error = Some(error.to_string());
                status.connected = false;
                status.ready = false;
            }
            let status = self.status.lock().unwrap();
            let mut snapshot = Snapshot {
                connected: status.connected,
                ready: status.ready,
                error: status.error.clone(),
                ..default()
            };
            if !status.ready || !status.connected {
                return snapshot;
            }
            drop(status);
            snapshot.games = conn
                .db
                .game()
                .iter()
                .map(|g| super::Game {
                    id: g.id,
                    title: g.title,
                })
                .collect();
            snapshot.games.sort_by_key(|g| g.id);
            snapshot.game_id = conn
                .db
                .player()
                .connection_id()
                .find(&conn.connection_id())
                .map(|p| p.game_id);
            if snapshot.game_id != self.game_id {
                if let Some(handle) = self.units.take() {
                    let _ = handle.unsubscribe();
                }
                self.game_id = snapshot.game_id;
                if let Some(id) = self.game_id {
                    let errors = self.status.clone();
                    self.units = Some(
                        conn.subscription_builder()
                            .on_error(move |_, error| {
                                errors.lock().unwrap().error = Some(error.to_string())
                            })
                            .subscribe(format!("SELECT * FROM unit WHERE game_id = {id}")),
                    );
                }
            }
            snapshot.units = conn
                .db
                .unit()
                .iter()
                .filter(|u| Some(u.game_id) == snapshot.game_id)
                .map(|u| super::Unit {
                    id: u.id,
                    number: u.number as usize,
                    x: u.x,
                    y: u.y,
                    z: u.z,
                })
                .collect();
            snapshot
        }
    }

    impl Drop for Backend {
        fn drop(&mut self) {
            if let Some(conn) = self.connection.take() {
                let _ = conn.disconnect();
                let _ = conn.frame_tick();
            }
        }
    }

    fn record_status(status: &Mutex<Status>, ctx: &ReducerEventContext) {
        if ctx.event.caller_connection_id != Some(ctx.connection_id()) {
            return;
        }
        status.lock().unwrap().error = match &ctx.event.status {
            spacetimedb_sdk::Status::Committed => None,
            spacetimedb_sdk::Status::Failed(error) => Some(error.to_string()),
            spacetimedb_sdk::Status::OutOfEnergy => Some("Server is out of energy.".into()),
        };
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

        fn until(network: &mut Multiplayer, predicate: impl Fn(&Snapshot) -> bool) {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                network.poll();
                assert!(
                    network.snapshot.error.is_none(),
                    "{:?}",
                    network.snapshot.error
                );
                if predicate(&network.snapshot) {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "Timed out waiting for native network state"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        #[test]
        #[ignore = "Requires the disposable SpacetimeDB integration-test database"]
        fn native_client_receives_server_movement() {
            let mut network = Multiplayer::default();
            assert!(network.send(Action::Connect {
                uri: std::env::var("SPACETIMEDB_URI").unwrap_or("http://127.0.0.1:3300".into()),
                database: std::env::var("SPACETIMEDB_DATABASE").unwrap_or("stronghold-test".into()),
            }));
            until(&mut network, |s| s.ready);
            assert!(network.send(Action::Create {
                title: format!(
                    "Native test {}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ),
            }));
            until(&mut network, |s| {
                s.game_id.is_some() && s.units.len() == 100
            });
            let id = network.snapshot.units[0].id;
            assert!(network.send(Action::Targets {
                unit_ids: vec![id],
                x: 1.0,
                z: 1.0
            }));
            until(&mut network, |s| {
                s.units
                    .iter()
                    .any(|u| u.id == id && u.x == 1.0 && u.z == 1.0)
            });
            assert!(network.send(Action::Leave));
            until(&mut network, |s| s.game_id.is_none() && s.units.is_empty());
        }
    }
}
