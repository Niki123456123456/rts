// Immutable, private world snapshots; loading always creates a separate world.
#[derive(spacetimedb::SpacetimeType)]
pub struct SavedWorld {
    pub owners: Vec<Participant>,
    pub predator_peace: Vec<PredatorPeace>,
    pub building: Vec<Building>,
    pub army_unit: Vec<ArmyUnit>,
    pub wildlife: Vec<Wildlife>,
    pub building_rotation: Vec<BuildingRotation>,
    pub terrain_clearing: Vec<TerrainClearing>,
    pub unit_profile: Vec<UnitProfile>,
    pub keep_population: Vec<KeepPopulation>,
    pub settlement_needs: Vec<SettlementNeeds>,
    pub settlement_progress: Vec<SettlementProgress>,
    pub building_operation: Vec<BuildingOperation>,
    pub warehouse_stock: Vec<WarehouseStock>,
    pub granary_stock: Vec<GranaryStock>,
    pub fish_stock: Vec<FishStock>,
    pub producer: Vec<Producer>,
    pub worker: Vec<Worker>,
    pub tower_occupant: Vec<TowerOccupant>,
    pub tower_travel: Vec<TowerTravel>,
    pub wall_entry: Vec<WallEntry>,
    pub gate_state: Vec<GateState>,
    pub moat: Vec<Moat>,
    pub wall_stair: Vec<WallStair>,
    pub resource_node: Vec<ResourceNode>,
    pub ox_cart: Vec<OxCart>,
    pub ecology_state: Vec<EcologyState>,
    pub player_resources: Vec<PlayerResources>,
    pub unit_order: Vec<UnitOrder>,
    pub hunt_order: Vec<HuntOrder>,
    pub wildlife_order: Vec<WildlifeOrder>,
    pub player_keep_spawn: Vec<PlayerKeepSpawn>,
    pub logistics_player: Vec<LogisticsPlayer>,
    pub granary_player: Vec<GranaryPlayer>,
    pub starting_supplies: Vec<StartingSupplies>,
}
#[spacetimedb::table(name=save_slot)]
pub struct SaveSlot {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub owner: Identity,
    pub title: String,
    pub data: SavedWorld,
}
#[spacetimedb::table(name=save_catalog,public)]
pub struct SaveCatalog {
    #[primary_key]
    pub id: u64,
    pub owner: Identity,
    pub title: String,
    pub saved_at_micros: i64,
}
#[spacetimedb::table(name=restored_player)]
pub struct RestoredPlayer {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub identity: Identity,
    pub connection_id: spacetimedb::ConnectionId,
    pub color: u32,
}
#[spacetimedb::reducer]
pub fn save_game(ctx: &ReducerContext, title: String) -> Result<(), String> {
    let p = participant(ctx)?;
    let title = title.trim().chars().take(60).collect::<String>();
    if title.is_empty() {
        return Err("Gib dem Spielstand einen Namen.".into());
    }
    let bids = ctx
        .db
        .building()
        .game_id()
        .filter(p.game_id)
        .map(|b| b.id)
        .collect::<std::collections::HashSet<_>>();
    let uids = ctx
        .db
        .army_unit()
        .game_id()
        .filter(p.game_id)
        .map(|u| u.id)
        .collect::<std::collections::HashSet<_>>();
    let aids = ctx
        .db
        .wildlife()
        .game_id()
        .filter(p.game_id)
        .map(|u| u.id)
        .collect::<std::collections::HashSet<_>>();
    let owners = ctx
        .db
        .participant()
        .game_id()
        .filter(p.game_id)
        .chain(
            ctx.db
                .restored_player()
                .game_id()
                .filter(p.game_id)
                .map(|r| Participant {
                    connection_id: r.connection_id,
                    identity: r.identity,
                    game_id: r.game_id,
                    color: r.color,
                }),
        )
        .collect::<Vec<_>>();
    let conns = owners
        .iter()
        .map(|p| p.connection_id)
        .collect::<std::collections::HashSet<_>>();
    let data = SavedWorld {
        owners,
        predator_peace: ctx
            .db
            .predator_peace()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        building: ctx
            .db
            .building()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        army_unit: ctx
            .db
            .army_unit()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        wildlife: ctx
            .db
            .wildlife()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        building_rotation: ctx
            .db
            .building_rotation()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        terrain_clearing: ctx
            .db
            .terrain_clearing()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        unit_profile: ctx
            .db
            .unit_profile()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        keep_population: ctx
            .db
            .keep_population()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        settlement_needs: ctx
            .db
            .settlement_needs()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        settlement_progress: ctx
            .db
            .settlement_progress()
            .iter()
            .filter(|r| bids.contains(&r.building_id))
            .collect(),
        building_operation: ctx
            .db
            .building_operation()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        warehouse_stock: ctx
            .db
            .warehouse_stock()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        granary_stock: ctx
            .db
            .granary_stock()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        fish_stock: ctx
            .db
            .fish_stock()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        producer: ctx
            .db
            .producer()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        worker: ctx
            .db
            .worker()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        tower_occupant: ctx
            .db
            .tower_occupant()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        tower_travel: ctx
            .db
            .tower_travel()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        wall_entry: ctx
            .db
            .wall_entry()
            .iter()
            .filter(|r| uids.contains(&r.unit_id))
            .collect(),
        gate_state: ctx
            .db
            .gate_state()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        moat: ctx
            .db
            .moat()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        wall_stair: ctx
            .db
            .wall_stair()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        resource_node: ctx
            .db
            .resource_node()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        ox_cart: ctx
            .db
            .ox_cart()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        ecology_state: ctx
            .db
            .ecology_state()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        player_resources: ctx
            .db
            .player_resources()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        unit_order: ctx
            .db
            .unit_order()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        hunt_order: ctx
            .db
            .hunt_order()
            .iter()
            .filter(|r| uids.contains(&r.unit_id))
            .collect(),
        wildlife_order: ctx
            .db
            .wildlife_order()
            .iter()
            .filter(|r| aids.contains(&r.animal_id))
            .collect(),
        player_keep_spawn: ctx
            .db
            .player_keep_spawn()
            .iter()
            .filter(|r| r.game_id == p.game_id)
            .collect(),
        logistics_player: ctx
            .db
            .logistics_player()
            .iter()
            .filter(|r| conns.contains(&r.connection_id))
            .collect(),
        granary_player: ctx
            .db
            .granary_player()
            .iter()
            .filter(|r| conns.contains(&r.connection_id))
            .collect(),
        starting_supplies: ctx
            .db
            .starting_supplies()
            .iter()
            .filter(|r| conns.contains(&r.connection_id))
            .collect(),
    };
    let previous = ctx
        .db
        .save_slot()
        .iter()
        .find(|s| s.owner == ctx.sender && s.title == title);
    let id = if let Some(s) = previous {
        let id = s.id;
        ctx.db.save_slot().id().update(SaveSlot {
            id,
            owner: ctx.sender,
            title: title.clone(),
            data,
        });
        id
    } else {
        if ctx
            .db
            .save_slot()
            .iter()
            .filter(|s| s.owner == ctx.sender)
            .count()
            >= 12
        {
            return Err("Bis zu 12 Spielstände möglich. Verwende einen bestehenden Namen zum Überschreiben.".into());
        }
        ctx.db
            .save_slot()
            .insert(SaveSlot {
                id: 0,
                owner: ctx.sender,
                title: title.clone(),
                data,
            })
            .id
    };
    let catalog = SaveCatalog {
        id,
        owner: ctx.sender,
        title,
        saved_at_micros: ctx.timestamp.to_micros_since_unix_epoch(),
    };
    if ctx.db.save_catalog().id().find(id).is_some() {
        ctx.db.save_catalog().id().update(catalog);
    } else {
        ctx.db.save_catalog().insert(catalog);
    }
    Ok(())
}
#[spacetimedb::reducer]
pub fn load_game(ctx: &ReducerContext, save_id: u64) -> Result<(), String> {
    let saved = ctx
        .db
        .save_slot()
        .id()
        .find(save_id)
        .filter(|s| s.owner == ctx.sender)
        .ok_or("Dieser Spielstand gehört dir nicht oder existiert nicht mehr.")?;
    let connection = ctx.connection_id.ok_or("Verbindung erforderlich.")?;
    leave_game(ctx);
    let game = ctx.db.game().insert(Game {
        id: 0,
        title: format!(
            "{} · {}",
            saved.title,
            ctx.timestamp.to_micros_since_unix_epoch()
        ),
    });
    let data = saved.data;
    let mut owners = std::collections::HashMap::new();
    for old in data.owners {
        let placeholder = spacetimedb::ConnectionId::from_u128(
            (u128::from(game.id) << 64) | u128::from(old.color + 1),
        );
        owners.insert(old.connection_id, placeholder);
        ctx.db.restored_player().insert(RestoredPlayer {
            id: 0,
            game_id: game.id,
            identity: old.identity,
            connection_id: placeholder,
            color: old.color,
        });
    }
    let mut bids = std::collections::HashMap::new();
    let mut uids = std::collections::HashMap::new();
    let mut aids = std::collections::HashMap::new();
    for mut r in data.building {
        let old = r.id;
        r.id = 0;
        r.game_id = game.id;
        r.owner = *owners
            .get(&r.owner)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        let row = ctx.db.building().insert(r);
        bids.insert(old, row.id);
    }
    for mut r in data.army_unit {
        let old = r.id;
        r.id = 0;
        r.game_id = game.id;
        r.owner = *owners
            .get(&r.owner)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        let row = ctx.db.army_unit().insert(r);
        uids.insert(old, row.id);
    }
    for mut r in data.wildlife {
        let old = r.id;
        r.id = 0;
        r.game_id = game.id;
        let row = ctx.db.wildlife().insert(r);
        aids.insert(old, row.id);
    }
    for mut r in data.building_rotation {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.building_rotation().insert(r);
    }
    for mut r in data.terrain_clearing {
        r.id = 0;
        r.game_id = game.id;
        ctx.db.terrain_clearing().insert(r);
    }
    for mut r in data.unit_profile {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.unit_profile().insert(r);
    }
    for mut r in data.keep_population {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.keep_population().insert(r);
    }
    for mut r in data.settlement_needs {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.settlement_needs().insert(r);
    }
    for mut r in data.settlement_progress {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        ctx.db.settlement_progress().insert(r);
    }
    for mut r in data.building_operation {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.building_operation().insert(r);
    }
    for mut r in data.warehouse_stock {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.warehouse_stock().insert(r);
    }
    for mut r in data.granary_stock {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.owner = *owners
            .get(&r.owner)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        ctx.db.granary_stock().insert(r);
    }
    for mut r in data.fish_stock {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.fish_stock().insert(r);
    }
    for mut r in data.producer {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.producer().insert(r);
    }
    for mut r in data.worker {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.source_building = *bids
            .get(&r.source_building)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.warehouse = r.warehouse.and_then(|id| bids.get(&id).copied());
        ctx.db.worker().insert(r);
    }
    for mut r in data.tower_occupant {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.next_building = r.next_building.and_then(|id| bids.get(&id).copied());
        ctx.db.tower_occupant().insert(r);
    }
    for mut r in data.tower_travel {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        ctx.db.tower_travel().insert(r);
    }
    for mut r in data.wall_entry {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.tower_id = *bids
            .get(&r.tower_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        ctx.db.wall_entry().insert(r);
    }
    for mut r in data.gate_state {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        ctx.db.gate_state().insert(r);
    }
    for mut r in data.moat {
        r.id = 0;
        r.game_id = game.id;
        r.owner = *owners
            .get(&r.owner)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        ctx.db.moat().insert(r);
    }
    for mut r in data.wall_stair {
        r.id = 0;
        r.game_id = game.id;
        r.owner = *owners
            .get(&r.owner)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        r.wall_id = *bids
            .get(&r.wall_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        ctx.db.wall_stair().insert(r);
    }
    for mut r in data.resource_node {
        r.id = 0;
        r.game_id = game.id;
        ctx.db.resource_node().insert(r);
    }
    for mut r in data.ox_cart {
        r.building_id = *bids
            .get(&r.building_id)
            .ok_or("Gebäudeverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.source_building = r.source_building.and_then(|id| bids.get(&id).copied());
        ctx.db.ox_cart().insert(r);
    }
    for mut r in data.ecology_state {
        r.game_id = game.id;
        ctx.db.ecology_state().insert(r);
    }
    for mut r in data.player_resources {
        r.connection_id = *owners
            .get(&r.connection_id)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        r.game_id = game.id;
        ctx.db.player_resources().insert(r);
    }
    for mut r in data.unit_order {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.game_id = game.id;
        r.attack_building = r.attack_building.and_then(|id| bids.get(&id).copied());
        ctx.db.unit_order().insert(r);
    }
    for mut r in data.hunt_order {
        r.unit_id = *uids
            .get(&r.unit_id)
            .ok_or("Einheitenverweis im Spielstand fehlt.")?;
        r.animal_id = *aids
            .get(&r.animal_id)
            .ok_or("Tierverweis im Spielstand fehlt.")?;
        ctx.db.hunt_order().insert(r);
    }
    for mut r in data.wildlife_order {
        r.animal_id = *aids
            .get(&r.animal_id)
            .ok_or("Tierverweis im Spielstand fehlt.")?;
        ctx.db.wildlife_order().insert(r);
    }
    for mut r in data.player_keep_spawn {
        r.connection_id = *owners
            .get(&r.connection_id)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        r.game_id = game.id;
        ctx.db.player_keep_spawn().insert(r);
    }
    for mut r in data.logistics_player {
        r.connection_id = *owners
            .get(&r.connection_id)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        ctx.db.logistics_player().insert(r);
    }
    for mut r in data.granary_player {
        r.connection_id = *owners
            .get(&r.connection_id)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        ctx.db.granary_player().insert(r);
    }
    for mut r in data.starting_supplies {
        r.connection_id = *owners
            .get(&r.connection_id)
            .ok_or("Spielstand enthält einen unbekannten Besitzer.")?;
        ctx.db.starting_supplies().insert(r);
    }

    for mut r in data.predator_peace {
        r.game_id = game.id;
        ctx.db.predator_peace().insert(r);
    }
    ctx.db
        .ecology_balance()
        .insert(EcologyBalance { game_id: game.id });
    ctx.db
        .predator_revision()
        .insert(PredatorRevision { game_id: game.id });
    claim_restored_player(ctx, game.id, connection)?;
    for u in ctx
        .db
        .army_unit()
        .game_id()
        .filter(game.id)
        .collect::<Vec<_>>()
    {
        if ctx
            .db
            .unit_profile()
            .unit_id()
            .find(u.id)
            .is_some_and(|p| p.kind == "lord")
        {
            ctx.db
                .lord_vitality()
                .insert(LordVitality { unit_id: u.id });
        }
    }
    start_tick(ctx);
    start_migration(ctx);
    start_production(ctx);
    start_ecology(ctx);
    start_combat_scan(ctx);
    Ok(())
}
fn claim_restored_player(
    ctx: &ReducerContext,
    game: u64,
    connection: spacetimedb::ConnectionId,
) -> Result<bool, String> {
    let Some(old) = ctx
        .db
        .restored_player()
        .game_id()
        .filter(game)
        .find(|p| p.identity == ctx.sender)
    else {
        return Ok(false);
    };
    for mut b in ctx
        .db
        .building()
        .owner()
        .filter(old.connection_id)
        .collect::<Vec<_>>()
    {
        b.owner = connection;
        ctx.db.building().id().update(b);
    }
    for mut u in ctx
        .db
        .army_unit()
        .owner()
        .filter(old.connection_id)
        .collect::<Vec<_>>()
    {
        u.owner = connection;
        ctx.db.army_unit().id().update(u);
    }
    for mut g in ctx
        .db
        .granary_stock()
        .game_id()
        .filter(game)
        .filter(|g| g.owner == old.connection_id)
        .collect::<Vec<_>>()
    {
        g.owner = connection;
        ctx.db.granary_stock().building_id().update(g);
    }
    if let Some(mut r) = ctx
        .db
        .player_resources()
        .connection_id()
        .find(old.connection_id)
    {
        ctx.db
            .player_resources()
            .connection_id()
            .delete(old.connection_id);
        r.connection_id = connection;
        ctx.db.player_resources().insert(r);
    }
    if ctx
        .db
        .player_keep_spawn()
        .connection_id()
        .delete(old.connection_id)
    {
        ctx.db.player_keep_spawn().insert(PlayerKeepSpawn {
            connection_id: connection,
            game_id: game,
        });
    }
    if ctx
        .db
        .logistics_player()
        .connection_id()
        .delete(old.connection_id)
    {
        ctx.db.logistics_player().insert(LogisticsPlayer {
            connection_id: connection,
        });
    }
    if ctx
        .db
        .granary_player()
        .connection_id()
        .delete(old.connection_id)
    {
        ctx.db.granary_player().insert(GranaryPlayer {
            connection_id: connection,
        });
    }
    if ctx
        .db
        .starting_supplies()
        .connection_id()
        .delete(old.connection_id)
    {
        ctx.db.starting_supplies().insert(StartingSupplies {
            connection_id: connection,
        });
    }
    ctx.db.participant().insert(Participant {
        connection_id: connection,
        identity: ctx.sender,
        game_id: game,
        color: old.color,
    });
    ctx.db.player().insert(Player {
        connection_id: connection,
        identity: ctx.sender,
        game_id: game,
    });
    ctx.db.restored_player().id().delete(old.id);
    Ok(true)
}
