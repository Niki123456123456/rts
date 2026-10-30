use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub industry_stocks: Vec<IndustryStockView>,
    #[serde(default)]
    pub horses: Vec<HorseView>,
    #[serde(default)]
    pub factory_orders: Vec<FactoryView>,

    #[serde(default)]
    pub policies: Vec<PolicyView>,
    #[serde(default)]
    pub gold: Option<u64>,
    #[serde(default)]
    pub prices: Vec<PriceView>,
    #[serde(default)]
    pub players: Vec<PlayerView>,
    #[serde(default)]
    pub requests: Vec<RequestView>,
    #[serde(default)]
    pub saves: Vec<SaveView>,
    pub connected: bool,
    pub ready: bool,
    pub error: Option<String>,
    #[serde(default, deserialize_with = "optional_id")]
    pub game_id: Option<u64>,
    pub games: Vec<Game>,
    pub units: Vec<Unit>,
    #[serde(default)]
    pub buildings: Vec<Building>,
    #[serde(default)]
    pub clearings: Vec<TerrainClearing>,
    #[serde(default)]
    pub keep_population: Vec<KeepPopulation>,
    #[serde(default)]
    pub settlement_needs: Vec<SettlementNeeds>,
    #[serde(default)]
    pub building_operations: Vec<BuildingOperation>,
    #[serde(default)]
    pub resource_nodes: Vec<ResourceNode>,
    #[serde(default)]
    pub gates: Vec<GateStatus>,
    #[serde(default)]
    pub moats: Vec<MoatView>,
    #[serde(default)]
    pub stairs: Vec<StairView>,
    #[serde(default)]
    pub wildlife: Vec<Wildlife>,
    #[serde(default)]
    pub ox_carts: Vec<OxCart>,
    #[serde(default)]
    pub fish_stocks: Vec<FishStock>,
    #[serde(default)]
    pub roof_occupants: Vec<RoofOccupant>,
    #[serde(default)]
    pub resources: Option<Resources>,
    #[serde(default)]
    pub warehouses: Vec<Warehouse>,
    #[serde(default)]
    pub granaries: Vec<Granary>,
    #[serde(default)]
    pub workers: Vec<WorkerState>,
    #[serde(default)]
    pub producers: Vec<ProducerState>,
    #[serde(default)]
    pub arrows: Vec<Arrow>,
}

#[derive(Clone, Deserialize)]
pub struct PolicyView {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub rations: u8,
    pub taxes: u8,
    pub alarm: bool,
    pub alarm_started: u32,
}
#[derive(Clone, Deserialize)]
pub struct PriceView {
    pub resource: u8,
    pub pressure: i32,
}
#[derive(Clone, Deserialize)]
pub struct PlayerView {
    pub color: u32,
    pub owned: bool,
}
#[derive(Clone, Deserialize)]
pub struct RequestView {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub from_color: u32,
    pub to_color: u32,
    pub incoming: bool,
    pub outgoing: bool,
    pub resource: u8,
    pub amount: u32,
    pub offered_gold: u64,
    pub note: String,
    pub status: u8,
}
#[derive(Clone, Deserialize)]
pub struct SaveView {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub title: String,
    pub saved_at_micros: i64,
}
#[derive(Clone, Deserialize)]
pub struct GateStatus {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub open: bool,
}
#[derive(Clone, Deserialize)]
pub struct MoatView {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub owned: bool,
    pub x: f32,
    pub z: f32,
}
#[derive(Clone, Deserialize)]
pub struct StairView {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    #[serde(deserialize_with = "id")]
    pub wall_id: u64,
    pub owned: bool,
    pub base_x: f32,
    pub base_y: f32,
    pub base_z: f32,
    pub top_x: f32,
    pub top_y: f32,
    pub top_z: f32,
}
#[derive(Clone, Deserialize)]
pub struct ResourceNode {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub site: u32,
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub remaining: u32,
    pub capacity: u32,
    pub phase: u8,
    pub phase_ticks: u32,
}
#[derive(Clone, Deserialize)]
pub struct Wildlife {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub species: u8,
    pub group_id: u32,
    pub x: f32,
    pub z: f32,
    pub health: f32,
    pub state: u8,
    pub meat: u32,
}
#[derive(Clone, Deserialize)]
pub struct OxCart {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    #[serde(default, deserialize_with = "optional_id")]
    pub source_building: Option<u64>,
}
#[derive(Clone, Deserialize)]
pub struct FishStock {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub amount: u32,
}
#[derive(Clone, Deserialize)]
pub struct Arrow {
    #[serde(default)]
    pub kind: String,
    #[serde(deserialize_with = "id")]
    pub id: u64,
    #[serde(deserialize_with = "id")]
    pub target_id: u64,
    pub building: bool,
    pub start_x: f32,
    pub start_y: f32,
    pub start_z: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub end_z: f32,
    pub age: f32,
    pub flight_seconds: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub struct Resources {
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
    #[serde(default)]
    pub apples: u32,
    #[serde(default)]
    pub bread: u32,
    #[serde(default)]
    pub cheese: u32,
    #[serde(default)]
    pub meat: u32,
}

#[derive(Clone, Deserialize)]
pub struct Granary {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub apples: u32,
    pub bread: u32,
    pub cheese: u32,
    pub meat: u32,
}

#[derive(Clone, Deserialize)]
pub struct Warehouse {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
}
#[derive(Clone, Deserialize)]
pub struct WorkerState {
    #[serde(deserialize_with = "id")]
    pub unit_id: u64,
    #[serde(deserialize_with = "id")]
    pub source_building: u64,
    pub resource: u8,
    pub cargo: u32,
}
#[derive(Clone, Deserialize)]
pub struct ProducerState {
    #[serde(default)]
    pub progress: u32,
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub resource: u8,
    pub buffer: u32,
}

#[derive(Clone, Deserialize)]
pub struct Game {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub title: String,
}

#[derive(Clone, Deserialize)]
pub struct Unit {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub activity: u8,
    #[serde(default)]
    pub facing_x: f32,
    #[serde(default)]
    pub facing_z: f32,
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub number: usize,
    pub color: u32,
    pub health: f32,
    pub owned: bool,
    #[serde(default)]
    pub worker: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Deserialize)]
pub struct Building {
    pub kind: String,
    #[serde(default)]
    pub rotation: u8,
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub color: u32,
    pub health: f32,
    pub owned: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Deserialize)]
pub struct BuildingOperation {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub closed: bool,
}

#[derive(Clone, Deserialize)]
pub struct SettlementNeeds {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub happiness: u32,
    pub food: i32,
    pub variety: i32,
    pub housing: i32,
    pub water: i32,
    pub health: i32,
    pub leisure: i32,
    pub goods: i32,
    pub employment: i32,
    pub immigration_seconds: u32,
}

#[derive(Clone, Deserialize)]
pub struct KeepPopulation {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub arrived: u32,
    pub seconds: u32,
}

#[derive(Clone, Deserialize, PartialEq)]
pub struct TerrainClearing {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    pub x: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
}

#[derive(Clone, Deserialize)]
pub struct RoofOccupant {
    #[serde(deserialize_with = "id")]
    pub unit_id: u64,
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub slot: u8,
    #[serde(default, deserialize_with = "optional_id")]
    pub next_building: Option<u64>,
    pub next_slot: u8,
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

#[derive(Clone, Deserialize)]
pub struct IndustryStockView {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub wheat: u32,
    pub flour: u32,
}
#[derive(Clone, Deserialize)]
pub struct HorseView {
    #[serde(deserialize_with = "id")]
    pub id: u64,
    #[serde(deserialize_with = "id")]
    pub stable_id: u64,
    pub age: u32,
}
#[derive(Clone, Deserialize)]
pub struct FactoryView {
    #[serde(deserialize_with = "id")]
    pub building_id: u64,
    pub kind: String,
    pub progress: u32,
    pub duration: u32,
}
#[derive(Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    QueueSiege {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
        kind: String,
    },
    SetCivicPolicy {
        rations: u8,
        taxes: u8,
    },
    PalaceAlarm {
        active: bool,
    },
    CloseAllGates,
    MarketExchange {
        sell: u8,
        amount: u32,
        buy: u8,
    },
    MarketTrade {
        resource: u8,
        amount: u32,
        buy: bool,
    },
    RequestGoods {
        target_color: u32,
        resource: u8,
        amount: u32,
        offered_gold: u64,
        note: String,
    },
    RespondRequest {
        #[serde(serialize_with = "serialize_id")]
        request_id: u64,
        accept: bool,
    },
    SaveGame {
        title: String,
    },
    LoadGame {
        #[serde(serialize_with = "serialize_id")]
        save_id: u64,
    },
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
    AssignPeasant {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    UnassignPeasant {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    AssignOxCart {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    ReleaseOxCart {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    HuntAnimal {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        #[serde(serialize_with = "serialize_id")]
        animal_id: u64,
    },
    SetGate {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
        open: bool,
    },
    DigMoat {
        start_x: f32,
        start_z: f32,
        end_x: f32,
        end_z: f32,
    },
    FillMoat {
        #[serde(serialize_with = "serialize_id")]
        moat_id: u64,
    },
    BuildWallStair {
        #[serde(serialize_with = "serialize_id")]
        wall_id: u64,
        base_x: f32,
        base_z: f32,
        top_x: f32,
        top_z: f32,
    },
    BuildWallKind {
        start_x: f32,
        start_z: f32,
        end_x: f32,
        end_z: f32,
        kind: String,
    },
    RecruitUnit {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
        kind: String,
    },
    Build {
        x: f32,
        z: f32,
        building: String,
        rotation: u8,
    },
    BuildWall {
        start_x: f32,
        start_z: f32,
        end_x: f32,
        end_z: f32,
    },
    RemoveBuilding {
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    AttackUnits {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        #[serde(serialize_with = "serialize_id")]
        target_unit_id: u64,
    },
    AttackBuilding {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
    },
    MoveToRoof {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        #[serde(serialize_with = "serialize_id")]
        building_id: u64,
        x: f32,
        z: f32,
    },
    FormationTargets {
        #[serde(serialize_with = "serialize_ids")]
        unit_ids: Vec<u64>,
        start_x: f32,
        start_z: f32,
        end_x: f32,
        end_z: f32,
    },
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
        let awaiting_game = matches!(
            &action,
            Action::Create { .. } | Action::Join { .. } | Action::LoadGame { .. }
        );
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
                            .subscribe([
                                "SELECT * FROM game",
                                "SELECT * FROM participant",
                                "SELECT * FROM save_catalog",
                            ]);
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
                    .on_queue_siege(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_set_civic_policy(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_palace_alarm(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_close_all_gates(move |ctx| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_market_trade(move |ctx, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_market_exchange(move |ctx, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_request_goods(move |ctx, _, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_respond_request(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_save_game(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_load_game(move |ctx, _| record_status(&errors, ctx));
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
                let errors = self.status.clone();
                conn.reducers
                    .on_set_formation_targets(move |ctx, _, _, _, _, _| {
                        record_status(&errors, ctx)
                    });
                let errors = self.status.clone();
                conn.reducers
                    .on_build(move |ctx, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_build_rotated(move |ctx, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_build_wall(move |ctx, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_move_to_roof(move |ctx, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_remove_building(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_attack_building(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_recruit_unit(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_attack_units(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_assign_peasant(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_unassign_peasant(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_assign_ox_cart(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_release_ox_cart(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_hunt_animal(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_set_gate(move |ctx, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_dig_moat(move |ctx, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_fill_moat(move |ctx, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_build_wall_stair(move |ctx, _, _, _, _, _| record_status(&errors, ctx));
                let errors = self.status.clone();
                conn.reducers
                    .on_build_wall_kind(move |ctx, _, _, _, _, _| record_status(&errors, ctx));
                self.connection = Some(conn);
                return Ok(());
            }
            self.status.lock().unwrap().error = None;
            let conn = self
                .connection
                .as_ref()
                .ok_or("Connect to a server first.")?;
            match action {
                Action::SetGate { building_id, open } => conn.reducers.set_gate(building_id, open),
                Action::DigMoat {
                    start_x,
                    start_z,
                    end_x,
                    end_z,
                } => conn.reducers.dig_moat(start_x, start_z, end_x, end_z),
                Action::FillMoat { moat_id } => conn.reducers.fill_moat(moat_id),
                Action::BuildWallStair {
                    wall_id,
                    base_x,
                    base_z,
                    top_x,
                    top_z,
                } => conn
                    .reducers
                    .build_wall_stair(wall_id, base_x, base_z, top_x, top_z),
                Action::BuildWallKind {
                    start_x,
                    start_z,
                    end_x,
                    end_z,
                    kind,
                } => conn
                    .reducers
                    .build_wall_kind(start_x, start_z, end_x, end_z, kind),
                Action::QueueSiege { building_id, kind } => {
                    conn.reducers.queue_siege(building_id, kind)
                }
                Action::SetCivicPolicy { rations, taxes } => {
                    conn.reducers.set_civic_policy(rations, taxes)
                }
                Action::PalaceAlarm { active } => conn.reducers.palace_alarm(active),
                Action::CloseAllGates => conn.reducers.close_all_gates(),
                Action::MarketExchange { sell, amount, buy } => {
                    conn.reducers.market_exchange(sell, amount, buy)
                }
                Action::MarketTrade {
                    resource,
                    amount,
                    buy,
                } => conn.reducers.market_trade(resource, amount, buy),
                Action::RequestGoods {
                    target_color,
                    resource,
                    amount,
                    offered_gold,
                    note,
                } => {
                    conn.reducers
                        .request_goods(target_color, resource, amount, offered_gold, note)
                }
                Action::RespondRequest { request_id, accept } => {
                    conn.reducers.respond_request(request_id, accept)
                }
                Action::SaveGame { title } => conn.reducers.save_game(title),
                Action::LoadGame { save_id } => conn.reducers.load_game(save_id),
                Action::Create { title } => conn.reducers.create_game(title),
                Action::Join { game_id } => conn.reducers.join_game(game_id),
                Action::Leave => conn.reducers.leave_game(),
                Action::AssignOxCart { building_id } => conn.reducers.assign_ox_cart(building_id),
                Action::ReleaseOxCart { building_id } => conn.reducers.release_ox_cart(building_id),
                Action::HuntAnimal {
                    unit_ids,
                    animal_id,
                } => conn.reducers.hunt_animal(unit_ids, animal_id),
                Action::AssignPeasant { building_id } => conn.reducers.assign_peasant(building_id),
                Action::UnassignPeasant { building_id } => {
                    conn.reducers.unassign_peasant(building_id)
                }
                Action::RecruitUnit { building_id, kind } => {
                    conn.reducers.recruit_unit(building_id, kind)
                }
                Action::Build {
                    x,
                    z,
                    building,
                    rotation,
                } => conn.reducers.build_rotated(x, z, building, rotation),
                Action::BuildWall {
                    start_x,
                    start_z,
                    end_x,
                    end_z,
                } => conn.reducers.build_wall(start_x, start_z, end_x, end_z),
                Action::MoveToRoof {
                    unit_ids,
                    building_id,
                    x,
                    z,
                } => conn.reducers.move_to_roof(unit_ids, building_id, x, z),
                Action::RemoveBuilding { building_id } => {
                    conn.reducers.remove_building(building_id)
                }
                Action::AttackUnits {
                    unit_ids,
                    target_unit_id,
                } => conn.reducers.attack_units(unit_ids, target_unit_id),
                Action::AttackBuilding {
                    unit_ids,
                    building_id,
                } => conn.reducers.attack_building(unit_ids, building_id),
                Action::Targets { unit_ids, x, z } => conn.reducers.set_targets(unit_ids, x, z),
                Action::FormationTargets {
                    unit_ids,
                    start_x,
                    start_z,
                    end_x,
                    end_z,
                } => conn
                    .reducers
                    .set_formation_targets(unit_ids, start_x, start_z, end_x, end_z),
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
            snapshot.saves = conn
                .db
                .save_catalog()
                .iter()
                .filter(|s| s.owner == conn.identity())
                .map(|s| super::SaveView {
                    id: s.id,
                    title: s.title,
                    saved_at_micros: s.saved_at_micros,
                })
                .collect();
            snapshot.game_id = conn
                .db
                .participant()
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
                            .subscribe([
                                format!("SELECT * FROM civic_policy WHERE game_id = {id}"),
                                format!("SELECT * FROM industry_stock WHERE game_id = {id}"),
                                format!("SELECT * FROM stable_horse WHERE game_id = {id}"),
                                format!("SELECT * FROM factory_order WHERE game_id = {id}"),
                                format!("SELECT * FROM projectile_style WHERE game_id = {id}"),
                                format!("SELECT * FROM treasury WHERE game_id = {id}"),
                                format!("SELECT * FROM market_pressure WHERE game_id = {id}"),
                                format!("SELECT * FROM diplomatic_request WHERE game_id = {id}"),
                                format!("SELECT * FROM army_unit WHERE game_id = {id}"),
                                format!("SELECT * FROM building WHERE game_id = {id}"),
                                format!("SELECT * FROM terrain_clearing WHERE game_id = {id}"),
                                format!("SELECT * FROM keep_population WHERE game_id = {id}"),
                                format!("SELECT * FROM settlement_needs WHERE game_id = {id}"),
                                format!("SELECT * FROM building_operation WHERE game_id = {id}"),
                                format!("SELECT * FROM resource_node WHERE game_id = {id}"),
                                format!("SELECT * FROM gate_state WHERE game_id = {id}"),
                                format!("SELECT * FROM moat WHERE game_id = {id}"),
                                format!("SELECT * FROM wall_stair WHERE game_id = {id}"),
                                format!("SELECT * FROM wildlife WHERE game_id = {id}"),
                                format!("SELECT * FROM ox_cart WHERE game_id = {id}"),
                                format!("SELECT * FROM fish_stock WHERE game_id = {id}"),
                                format!("SELECT * FROM building_rotation WHERE game_id = {id}"),
                                format!("SELECT * FROM tower_occupant WHERE game_id = {id}"),
                                format!("SELECT * FROM player_resources WHERE game_id = {id}"),
                                format!("SELECT * FROM warehouse_stock WHERE game_id = {id}"),
                                format!("SELECT * FROM granary_stock WHERE game_id = {id}"),
                                format!("SELECT * FROM worker WHERE game_id = {id}"),
                                format!("SELECT * FROM producer WHERE game_id = {id}"),
                                format!("SELECT * FROM unit_profile WHERE game_id = {id}"),
                                format!("SELECT * FROM arrow_projectile WHERE game_id = {id}"),
                            ]),
                    );
                }
            }
            snapshot.industry_stocks = conn
                .db
                .industry_stock()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::IndustryStockView {
                    building_id: r.building_id,
                    wheat: r.wheat,
                    flour: r.flour,
                })
                .collect();
            snapshot.horses = conn
                .db
                .stable_horse()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::HorseView {
                    id: r.id,
                    stable_id: r.stable_id,
                    age: r.age,
                })
                .collect();
            snapshot.factory_orders = conn
                .db
                .factory_order()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::FactoryView {
                    building_id: r.building_id,
                    kind: r.kind,
                    progress: r.progress,
                    duration: r.duration,
                })
                .collect();
            snapshot.policies = conn
                .db
                .civic_policy()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::PolicyView {
                    building_id: r.building_id,
                    rations: r.rations,
                    taxes: r.taxes,
                    alarm: r.alarm,
                    alarm_started: r.alarm_started,
                })
                .collect();
            snapshot.gold = conn
                .db
                .treasury()
                .connection_id()
                .find(&conn.connection_id())
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| r.gold);
            snapshot.prices = conn
                .db
                .market_pressure()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::PriceView {
                    resource: r.resource,
                    pressure: r.pressure,
                })
                .collect();
            snapshot.players = conn
                .db
                .participant()
                .iter()
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::PlayerView {
                    color: r.color,
                    owned: r.connection_id == conn.connection_id(),
                })
                .collect();
            snapshot.requests = conn
                .db
                .diplomatic_request()
                .iter()
                .filter(|r| {
                    Some(r.game_id) == snapshot.game_id
                        && (r.from == conn.connection_id() || r.to == conn.connection_id())
                })
                .map(|r| super::RequestView {
                    id: r.id,
                    from_color: r.from_color,
                    to_color: r.to_color,
                    incoming: r.to == conn.connection_id(),
                    outgoing: r.from == conn.connection_id(),
                    resource: r.resource,
                    amount: r.amount,
                    offered_gold: r.offered_gold,
                    note: r.note,
                    status: r.status,
                })
                .collect();
            let food = conn
                .db
                .granary_stock()
                .iter()
                .filter(|s| Some(s.game_id) == snapshot.game_id && s.owner == conn.connection_id())
                .fold([0u32; 4], |n, s| {
                    [
                        n[0].saturating_add(s.apples),
                        n[1].saturating_add(s.bread),
                        n[2].saturating_add(s.cheese),
                        n[3].saturating_add(s.meat),
                    ]
                });
            snapshot.granaries = conn
                .db
                .granary_stock()
                .iter()
                .filter(|s| Some(s.game_id) == snapshot.game_id)
                .map(|s| super::Granary {
                    building_id: s.building_id,
                    apples: s.apples,
                    bread: s.bread,
                    cheese: s.cheese,
                    meat: s.meat,
                })
                .collect();
            snapshot.resources = conn
                .db
                .player_resources()
                .connection_id()
                .find(&conn.connection_id())
                .filter(|r| Some(r.game_id) == snapshot.game_id)
                .map(|r| super::Resources {
                    wood: r.wood,
                    stone: r.stone,
                    iron: r.iron,
                    apples: food[0],
                    bread: food[1],
                    cheese: food[2],
                    meat: food[3],
                });
            snapshot.keep_population = conn
                .db
                .keep_population()
                .iter()
                .filter(|k| Some(k.game_id) == snapshot.game_id)
                .map(|k| super::KeepPopulation {
                    building_id: k.building_id,
                    arrived: k.arrived,
                    seconds: k.seconds,
                })
                .collect();
            snapshot.gates = conn
                .db
                .gate_state()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| GateStatus {
                    building_id: n.building_id,
                    open: n.open,
                })
                .collect();
            snapshot.moats = conn
                .db
                .moat()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| MoatView {
                    id: n.id,
                    owned: n.owner == conn.connection_id(),
                    x: n.x,
                    z: n.z,
                })
                .collect();
            snapshot.stairs = conn
                .db
                .wall_stair()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| StairView {
                    id: n.id,
                    wall_id: n.wall_id,
                    owned: n.owner == conn.connection_id(),
                    base_x: n.base_x,
                    base_y: n.base_y,
                    base_z: n.base_z,
                    top_x: n.top_x,
                    top_y: n.top_y,
                    top_z: n.top_z,
                })
                .collect();
            snapshot.resource_nodes = conn
                .db
                .resource_node()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::ResourceNode {
                    id: n.id,
                    site: n.site,
                    kind: n.kind,
                    x: n.x,
                    z: n.z,
                    remaining: n.remaining,
                    capacity: n.capacity,
                    phase: n.phase,
                    phase_ticks: n.phase_ticks,
                })
                .collect();
            snapshot.wildlife = conn
                .db
                .wildlife()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::Wildlife {
                    id: n.id,
                    species: n.species,
                    group_id: n.group_id,
                    x: n.x,
                    z: n.z,
                    health: n.health,
                    state: n.state,
                    meat: n.meat,
                })
                .collect();
            snapshot.ox_carts = conn
                .db
                .ox_cart()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::OxCart {
                    building_id: n.building_id,
                    source_building: n.source_building,
                })
                .collect();
            snapshot.fish_stocks = conn
                .db
                .fish_stock()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::FishStock {
                    building_id: n.building_id,
                    amount: n.amount,
                })
                .collect();
            snapshot.building_operations = conn
                .db
                .building_operation()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::BuildingOperation {
                    building_id: n.building_id,
                    closed: n.closed,
                })
                .collect();
            snapshot.settlement_needs = conn
                .db
                .settlement_needs()
                .iter()
                .filter(|n| Some(n.game_id) == snapshot.game_id)
                .map(|n| super::SettlementNeeds {
                    building_id: n.building_id,
                    happiness: n.happiness,
                    food: n.food,
                    variety: n.variety,
                    housing: n.housing,
                    water: n.water,
                    health: n.health,
                    leisure: n.leisure,
                    goods: n.goods,
                    employment: n.employment,
                    immigration_seconds: n.immigration_seconds,
                })
                .collect();
            snapshot.warehouses = conn
                .db
                .warehouse_stock()
                .iter()
                .filter(|s| Some(s.game_id) == snapshot.game_id)
                .map(|s| super::Warehouse {
                    building_id: s.building_id,
                    wood: s.wood,
                    stone: s.stone,
                    iron: s.iron,
                })
                .collect();
            snapshot.workers = conn
                .db
                .worker()
                .iter()
                .filter(|w| Some(w.game_id) == snapshot.game_id)
                .map(|w| super::WorkerState {
                    unit_id: w.unit_id,
                    source_building: w.source_building,
                    resource: w.resource,
                    cargo: w.cargo,
                })
                .collect();
            snapshot.producers = conn
                .db
                .producer()
                .iter()
                .filter(|p| Some(p.game_id) == snapshot.game_id)
                .map(|p| super::ProducerState {
                    progress: p.progress,
                    building_id: p.building_id,
                    resource: p.resource,
                    buffer: p.buffer,
                })
                .collect();
            snapshot.arrows = conn
                .db
                .arrow_projectile()
                .iter()
                .filter(|a| Some(a.game_id) == snapshot.game_id)
                .map(|a| super::Arrow {
                    kind: conn
                        .db
                        .projectile_style()
                        .projectile_id()
                        .find(&a.id)
                        .map_or_else(|| "archer".into(), |s| s.kind),
                    id: a.id,
                    target_id: a.target_id,
                    building: a.building,
                    start_x: a.start_x,
                    start_y: a.start_y,
                    start_z: a.start_z,
                    end_x: a.end_x,
                    end_y: a.end_y,
                    end_z: a.end_z,
                    age: a.age,
                    flight_seconds: a.flight_seconds,
                })
                .collect();
            snapshot.units = conn
                .db
                .army_unit()
                .iter()
                .filter(|u| Some(u.game_id) == snapshot.game_id)
                .map(|u| super::Unit {
                    id: u.id,
                    kind: conn
                        .db
                        .unit_profile()
                        .unit_id()
                        .find(&u.id)
                        .map(|p| p.kind)
                        .unwrap_or_default(),
                    activity: conn
                        .db
                        .unit_profile()
                        .unit_id()
                        .find(&u.id)
                        .map(|p| p.activity)
                        .unwrap_or(0),
                    facing_x: conn
                        .db
                        .unit_profile()
                        .unit_id()
                        .find(&u.id)
                        .map(|p| p.facing_x)
                        .unwrap_or(0.),
                    facing_z: conn
                        .db
                        .unit_profile()
                        .unit_id()
                        .find(&u.id)
                        .map(|p| p.facing_z)
                        .unwrap_or(-1.),
                    number: u.number as usize,
                    color: u.color,
                    health: u.health,
                    owned: u.owner == conn.connection_id()
                        && conn.db.worker().unit_id().find(&u.id).is_none(),
                    worker: conn.db.worker().unit_id().find(&u.id).is_some(),
                    x: u.x,
                    y: u.y,
                    z: u.z,
                })
                .collect();
            snapshot.roof_occupants = conn
                .db
                .tower_occupant()
                .iter()
                .filter(|o| Some(o.game_id) == snapshot.game_id)
                .map(|o| super::RoofOccupant {
                    unit_id: o.unit_id,
                    building_id: o.building_id,
                    slot: o.slot,
                    next_building: o.next_building,
                    next_slot: o.next_slot,
                })
                .collect();
            snapshot.buildings = conn
                .db
                .building()
                .iter()
                .filter(|b| Some(b.game_id) == snapshot.game_id)
                .map(|b| super::Building {
                    rotation: conn
                        .db
                        .building_rotation()
                        .building_id()
                        .find(&b.id)
                        .map_or(0, |row| row.quarter_turns),
                    kind: b.kind,
                    id: b.id,
                    color: b.color,
                    health: b.health,
                    owned: b.owner == conn.connection_id(),
                    x: b.x,
                    y: b.y,
                    z: b.z,
                })
                .collect();
            snapshot.clearings = conn
                .db
                .terrain_clearing()
                .iter()
                .filter(|c| Some(c.game_id) == snapshot.game_id)
                .map(|c| super::TerrainClearing {
                    id: c.id,
                    x: c.x,
                    z: c.z,
                    half_x: c.half_x,
                    half_z: c.half_z,
                })
                .collect();
            snapshot.clearings.sort_by_key(|c| c.id);
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
                    "Timed out: connected={}, ready={}, game={:?}, units={}",
                    network.snapshot.connected,
                    network.snapshot.ready,
                    network.snapshot.game_id,
                    network.snapshot.units.len()
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
            until(&mut network, |s| s.game_id.is_some() && s.units.len() == 1);
            // A perimeter unit has a route out; interior units are physically
            // enclosed while their neighbours remain stationary.
            let id = network
                .snapshot
                .units
                .iter()
                .find(|u| u.number == 1)
                .unwrap()
                .id;
            assert!(network.send(Action::Targets {
                unit_ids: vec![id],
                x: 3.0,
                z: 3.0
            }));
            until(&mut network, |s| {
                s.units
                    .iter()
                    .any(|u| u.id == id && u.x == 3.0 && u.z == 3.0)
            });
            assert!(network.send(Action::Build {
                x: -27.0,
                z: 15.0,
                building: "granary".into(),
                rotation: 0,
            }));
            until(&mut network, |s| s.buildings.len() == 1);
            let building = network.snapshot.buildings[0].clone();
            assert!(building.owned && building.health == 1.0);
            assert!(network.send(Action::RemoveBuilding {
                building_id: building.id
            }));
            until(&mut network, |s| s.buildings.is_empty());
            assert!(network.send(Action::Leave));
            until(&mut network, |s| s.game_id.is_none() && s.units.is_empty());
        }
    }
}
