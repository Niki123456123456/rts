use crate::simulation::{
    can_skip_corner, formation_assignment, move_separated, plan_attack, terrain_height,
};
use spacetimedb::{Identity, ReducerContext, ScheduleAt, Table};
use std::time::Duration;

const TICK_SECONDS: f32 = 0.05;
const UNIT_SPEED: f32 = 3.0;
use stronghold_map::{self as map, Obstacle, MAP_LIMIT};

// Successful construction permanently clears trees for this game, including
// after demolition or reconnect. Clients cannot request arbitrary clearings.
#[spacetimedb::table(name = terrain_clearing, public)]
pub struct TerrainClearing {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub x: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
}

#[spacetimedb::table(name = game, public)]
pub struct Game {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub title: String,
}

#[spacetimedb::table(name = unit, public)]
pub struct Unit {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub number: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub target_x: f32,
    pub target_z: f32,
}

// Membership is keyed by connection, so two tabs can independently join games.
#[spacetimedb::table(name = player, public)]
pub struct Player {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
    pub identity: Identity,
    pub game_id: u64,
}

// Keep the original tables above so the hosted database can upgrade without
// deleting existing games. New gameplay uses these versioned army records.
#[derive(Clone)]
#[spacetimedb::table(name = army_unit, public)]
pub struct ArmyUnit {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    #[index(btree)]
    pub owner: spacetimedb::ConnectionId,
    pub color: u32,
    pub number: u32,
    pub health: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub target_x: f32,
    pub target_z: f32,
}

#[spacetimedb::table(name = participant, public)]
pub struct Participant {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
    pub identity: Identity,
    #[index(btree)]
    pub game_id: u64,
    pub color: u32,
}

#[derive(Clone)]
#[spacetimedb::table(name = player_resources, public)]
pub struct PlayerResources {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
    #[index(btree)]
    pub game_id: u64,
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
}

// One-time supplies migrate legacy players who had no stockpile or starting balance.
#[spacetimedb::table(name=starting_supplies)]
pub struct StartingSupplies {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
}
fn ensure_resources(ctx: &ReducerContext, player: &Participant) -> PlayerResources {
    let mut balance = ctx
        .db
        .player_resources()
        .connection_id()
        .find(player.connection_id)
        .unwrap_or_else(|| {
            ctx.db.player_resources().insert(PlayerResources {
                connection_id: player.connection_id,
                game_id: player.game_id,
                wood: map::STARTING_RESOURCE_AMOUNT,
                stone: map::STARTING_RESOURCE_AMOUNT,
                iron: map::STARTING_RESOURCE_AMOUNT,
            })
        });
    if ctx
        .db
        .starting_supplies()
        .connection_id()
        .find(player.connection_id)
        .is_none()
    {
        ctx.db.starting_supplies().insert(StartingSupplies {
            connection_id: player.connection_id,
        });
        if balance.wood == 0
            && balance.stone == 0
            && balance.iron == 0
            && ctx
                .db
                .logistics_player()
                .connection_id()
                .find(player.connection_id)
                .is_none()
        {
            balance.wood = map::STARTING_RESOURCE_AMOUNT;
            balance.stone = map::STARTING_RESOURCE_AMOUNT;
            balance.iron = map::STARTING_RESOURCE_AMOUNT;
            ctx.db
                .player_resources()
                .connection_id()
                .update(balance.clone());
        }
    }
    balance
}

#[derive(Clone)]
#[spacetimedb::table(name = building, public)]
pub struct Building {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    #[index(btree)]
    pub owner: spacetimedb::ConnectionId,
    pub color: u32,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
}

// Additive orientation metadata preserves the existing building table and saved games.
#[spacetimedb::table(name = building_rotation, public)]
pub struct BuildingRotation {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub quarter_turns: u8,
}

#[derive(Clone)]
#[spacetimedb::table(name = tower_occupant, public)]
pub struct TowerOccupant {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    #[index(btree)]
    pub building_id: u64,
    pub slot: u8,
    pub door: u8,
    pub next_building: Option<u64>,
    pub next_slot: u8,
}

// Entry tower is separate metadata, so existing hosted table schemas stay intact.
#[spacetimedb::table(name = wall_entry)]
pub struct WallEntry {
    #[primary_key]
    pub unit_id: u64,
    pub tower_id: u64,
}

#[spacetimedb::table(name = tower_travel)]
pub struct TowerTravel {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub building_id: u64,
    pub path: Vec<f32>,
    // Waypoints: 0 roof walk, 3 door/hatch teleport, 4 wall walk, 5 internal transfer; 1/2 legacy.
    pub modes: Vec<u8>,
    pub next: u32,
    pub leaving: bool,
    pub active: bool,
}

// Paths and attack orders stay authoritative and private. Existing army schema
// remains unchanged, allowing a non-destructive upgrade of hosted games.
#[spacetimedb::table(name = unit_order)]
pub struct UnitOrder {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub attack_building: Option<u64>,
    pub waypoints: Vec<f32>,
}

#[spacetimedb::table(name = movement_tick, scheduled(move_units))]
pub struct MovementTick {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

#[spacetimedb::reducer]
pub fn create_game(ctx: &ReducerContext, title: String) -> Result<(), String> {
    let title = title.trim().to_owned();
    if title.is_empty() || title.chars().count() > 60 || title.chars().any(char::is_control) {
        return Err("Enter a game title between 1 and 60 characters.".into());
    }
    if ctx.db.game().title().find(&title).is_some() {
        return Err("A game with this title already exists.".into());
    }
    let game = ctx.db.game().insert(Game { id: 0, title });
    join_game(ctx, game.id)
}

#[spacetimedb::reducer]
pub fn join_game(ctx: &ReducerContext, game_id: u64) -> Result<(), String> {
    if ctx.db.game().id().find(game_id).is_none() {
        return Err("This game no longer exists.".into());
    }
    let connection_id = ctx
        .connection_id
        .ok_or("A client connection is required.")?;
    if let Some(player) = ctx.db.participant().connection_id().find(connection_id) {
        if player.identity != ctx.sender {
            return Err("This connection belongs to another player.".into());
        }
        if player.game_id == game_id {
            ensure_resources(ctx, &player);
            ensure_player_keep(ctx, &player)?;
            return Ok(());
        }
    }
    ensure_ecology(ctx, game_id);
    leave_game(ctx);
    if claim_restored_player(ctx, game_id, connection_id)? {
        start_tick(ctx);
        start_migration(ctx);
        start_production(ctx);
        start_ecology(ctx);
        start_combat_scan(ctx);
        return Ok(());
    }
    let used_colors: std::collections::HashSet<_> = ctx
        .db
        .participant()
        .game_id()
        .filter(game_id)
        .map(|player| player.color)
        .chain(
            ctx.db
                .restored_player()
                .game_id()
                .filter(game_id)
                .map(|p| p.color),
        )
        .collect();
    let color = (0..u32::MAX)
        .find(|color| !used_colors.contains(color))
        .ok_or("No player colors available.")?;
    ctx.db.participant().insert(Participant {
        connection_id,
        identity: ctx.sender,
        game_id,
        color,
    });
    ensure_resources(
        ctx,
        &ctx.db
            .participant()
            .connection_id()
            .find(connection_id)
            .unwrap(),
    );
    // Each lord starts near a separate player home inside the map.
    let axis = |slot: u32| {
        if slot == 0 {
            0.0
        } else {
            slot.div_ceil(2) as f32 * if slot % 2 == 1 { 9.0 } else { -9.0 }
        }
    };
    let origin_x = axis(color % 7) - 1.4;
    let origin_z = axis((color / 7) % 7) - 1.05;
    let mut x = origin_x;
    let mut z = origin_z;
    let obstacles = obstacles(ctx, game_id);
    if !map::free((x, z), &obstacles)
        || ctx
            .db
            .army_unit()
            .game_id()
            .filter(game_id)
            .any(|u| map::distance((x, z), (u.x, u.z)) < map::UNIT_SPACING)
    {
        let p = (-55..=55)
            .flat_map(|i| (-55..=55).map(move |j| (i as f32 * 0.7, j as f32 * 0.7)))
            .filter(|p| map::free(*p, &obstacles))
            .filter(|p| {
                !ctx.db
                    .army_unit()
                    .game_id()
                    .filter(game_id)
                    .any(|u| map::distance(*p, (u.x, u.z)) < map::UNIT_SPACING)
            })
            .min_by(|a, b| map::distance(*a, (x, z)).total_cmp(&map::distance(*b, (x, z))))
            .ok_or("No room for this army.")?;
        x = p.0;
        z = p.1;
    }
    let unit = ctx.db.army_unit().insert(ArmyUnit {
        id: 0,
        game_id,
        owner: connection_id,
        color,
        number: 1,
        health: map::units::UnitKind::Lord.max_health(),
        x,
        y: terrain_height(x, z) + 0.75,
        z,
        target_x: x,
        target_z: z,
    });
    register_combat_unit(ctx, &unit, map::units::UnitKind::Lord);
    ensure_player_keep(ctx, &participant(ctx)?)?;
    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_game(ctx: &ReducerContext) {
    if let Some(connection_id) = ctx.connection_id {
        if let Some(player) = ctx.db.participant().connection_id().find(connection_id) {
            if player.identity != ctx.sender {
                return;
            }
            for arrow in ctx
                .db
                .arrow_projectile()
                .game_id()
                .filter(player.game_id)
                .filter(|a| a.owner == connection_id)
                .collect::<Vec<_>>()
            {
                ctx.db.arrow_projectile().id().delete(arrow.id);
            }
            let units: Vec<_> = ctx.db.army_unit().owner().filter(connection_id).collect();
            for unit in units {
                ctx.db.lord_vitality().unit_id().delete(unit.id);
                ctx.db.hunt_order().unit_id().delete(unit.id);
                ctx.db.archer_cooldown().unit_id().delete(unit.id);
                ctx.db.ranged_building_order().unit_id().delete(unit.id);
                ctx.db.worker().unit_id().delete(unit.id);
                ctx.db.unit_profile().unit_id().delete(unit.id);
                ctx.db.unit_engagement().unit_id().delete(unit.id);
                ctx.db.combat_surface_home().unit_id().delete(unit.id);
                clear_tower(ctx, unit.id);
                ctx.db.unit_order().unit_id().delete(unit.id);
                ctx.db.army_unit().id().delete(unit.id);
            }
            for building in ctx
                .db
                .building()
                .owner()
                .filter(connection_id)
                .collect::<Vec<_>>()
            {
                delete_building(ctx, building.id);
            }
            replan(ctx, player.game_id);
            ctx.db.participant().connection_id().delete(connection_id);
        }
        ctx.db.treasury().connection_id().delete(connection_id);
        ctx.db
            .player_resources()
            .connection_id()
            .delete(connection_id);
        ctx.db
            .logistics_player()
            .connection_id()
            .delete(connection_id);
        ctx.db
            .granary_player()
            .connection_id()
            .delete(connection_id);
        ctx.db
            .starting_supplies()
            .connection_id()
            .delete(connection_id);
        ctx.db.player().connection_id().delete(connection_id);
        ctx.db
            .player_keep_spawn()
            .connection_id()
            .delete(connection_id);
    }
}

#[spacetimedb::reducer(client_connected)]
pub fn connected(ctx: &ReducerContext) {
    for player in ctx.db.participant().iter() {
        ensure_resources(ctx, &player);
    }
    for b in ctx.db.building().iter().collect::<Vec<_>>() {
        if let Err(error) = register_logistics_building(ctx, &b) {
            spacetimedb::log::warn!("{error}");
        }
    }
    for unit in ctx.db.army_unit().iter() {
        if ctx.db.worker().unit_id().find(unit.id).is_none()
            && ctx.db.unit_profile().unit_id().find(unit.id).is_none()
        {
            register_combat_unit(
                ctx,
                &unit,
                if unit.number == 1 {
                    map::units::UnitKind::Lord
                } else {
                    map::units::UnitKind::ArmedPeasant
                },
            );
        }
    }
    start_production(ctx);
    start_combat_scan(ctx);
    normalize_roof_slots(ctx);
    for player in ctx.db.participant().iter().collect::<Vec<_>>() {
        if let Err(error) = ensure_player_keep(ctx, &player) {
            spacetimedb::log::warn!("Cannot create starting keep: {error}");
        }
    }
    if ctx.db.tower_travel().iter().next().is_some() {
        start_tick(ctx);
    }
}

#[spacetimedb::reducer(client_disconnected)]
pub fn disconnected(ctx: &ReducerContext) {
    leave_game(ctx);
}

#[spacetimedb::reducer]
pub fn set_targets(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    target_x: f32,
    target_z: f32,
) -> Result<(), String> {
    if !target_x.is_finite() || !target_z.is_finite() {
        return Err("Target must be finite.".into());
    }
    if target_x.abs() > MAP_LIMIT || target_z.abs() > MAP_LIMIT {
        return Err("Target is outside the map.".into());
    }
    let player = participant(ctx)?;
    let units = owned_units(ctx, &player, unit_ids)?;
    let obstacles = obstacles(ctx, player.game_id);
    let selected: std::collections::HashSet<_> = units.iter().map(|u| u.id).collect();
    let mut reserved = ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .filter(|u| !selected.contains(&u.id))
        .flat_map(|u| ground_reservations(ctx, &u))
        .collect::<Vec<_>>();
    let center_x = units.iter().map(|u| u.x).sum::<f32>() / units.len() as f32;
    let center_z = units.iter().map(|u| u.z).sum::<f32>() / units.len() as f32;
    for mut unit in units {
        // Formation offsets use authoritative positions, never client positions.
        unit.target_x = (target_x + unit.x - center_x).clamp(-MAP_LIMIT, MAP_LIMIT);
        unit.target_z = (target_z + unit.z - center_z).clamp(-MAP_LIMIT, MAP_LIMIT);
        if !map::free((unit.target_x, unit.target_z), &obstacles) {
            return Err("Target is blocked or unreachable. Choose open ground.".into());
        }
        let target = free_destination((unit.target_x, unit.target_z), &reserved, &obstacles)
            .ok_or("There is no free space at this destination.")?;
        unit.target_x = target.0;
        unit.target_z = target.1;
        reserved.push(target);
        let start = depart_tower(ctx, &unit)?;
        let path = map::shortest_path(start, (unit.target_x, unit.target_z), &obstacles)
            .ok_or("Target is blocked or unreachable. Choose open ground.")?;
        save_order(ctx, &unit, None, path);
        ctx.db.army_unit().id().update(unit);
    }
    start_tick(ctx);
    Ok(())
}

/// The server derives all slots from the same endpoints used by the preview.
#[spacetimedb::reducer]
pub fn set_formation_targets(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    start_x: f32,
    start_z: f32,
    end_x: f32,
    end_z: f32,
) -> Result<(), String> {
    for value in [start_x, start_z, end_x, end_z] {
        if !value.is_finite() || value.abs() > MAP_LIMIT {
            return Err("Formation must be inside the map.".into());
        }
    }
    let player = participant(ctx)?;
    let mut units = owned_units(ctx, &player, unit_ids)?;
    units.sort_by_key(|unit| unit.id);
    let obstacles = obstacles(ctx, player.game_id);
    let selected: std::collections::HashSet<_> = units.iter().map(|u| u.id).collect();
    let mut reserved = ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .filter(|u| !selected.contains(&u.id))
        .flat_map(|u| ground_reservations(ctx, &u))
        .collect::<Vec<_>>();
    let slots = map::formation_positions((start_x, start_z), (end_x, end_z), units.len());
    let assignment = formation_assignment(
        &units.iter().map(|u| (u.x, u.z)).collect::<Vec<_>>(),
        &slots,
    );
    for (mut unit, slot) in units.into_iter().zip(assignment) {
        let target = slots[slot];
        // Keep exact preview positions; reject blocked formations instead of distorting rows.
        if !map::free(target, &obstacles)
            || reserved
                .iter()
                .any(|p| map::distance(*p, target) < map::UNIT_SPACING - 0.001)
        {
            return Err("Formation is blocked. Choose open ground.".into());
        }
        let start = depart_tower(ctx, &unit)?;
        let path = map::shortest_path(start, target, &obstacles)
            .ok_or("Formation is unreachable. Choose open ground.")?;
        unit.target_x = target.0;
        unit.target_z = target.1;
        reserved.push(target);
        save_order(ctx, &unit, None, path);
        ctx.db.army_unit().id().update(unit);
    }
    start_tick(ctx);
    Ok(())
}

fn start_tick(ctx: &ReducerContext) {
    // No background calls while idle: important for usage-based cloud hosting.
    if ctx.db.movement_tick().iter().next().is_none() {
        ctx.db.movement_tick().insert(MovementTick {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(50).into(),
        });
    }
}

#[spacetimedb::reducer]
pub fn move_units(ctx: &ReducerContext, tick: MovementTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Movement may only be run by the server scheduler.".into());
    }
    normalize_roof_slots(ctx);
    // Upgrade any in-flight orders from the earlier movement implementation.
    for unit in ctx.db.army_unit().iter() {
        if (unit.x != unit.target_x || unit.z != unit.target_z)
            && ctx.db.unit_order().unit_id().find(unit.id).is_none()
            && ctx.db.tower_occupant().unit_id().find(unit.id).is_none()
        {
            if let Some(path) = map::shortest_path(
                (unit.x, unit.z),
                (unit.target_x, unit.target_z),
                &obstacles(ctx, unit.game_id),
            ) {
                save_order(ctx, &unit, None, path);
            } else {
                stop_unit(ctx, unit);
            }
        }
    }
    let previous_positions = ctx
        .db
        .army_unit()
        .iter()
        .map(|u| (u.id, (u.x, u.z)))
        .collect();
    let tower_moved = step_tower_travel(ctx);
    let mut changed_games = std::collections::HashSet::new();
    for mut order in ctx.db.unit_order().iter().collect::<Vec<_>>() {
        let Some(mut unit) = ctx.db.army_unit().id().find(order.unit_id) else {
            ctx.db.unit_order().unit_id().delete(order.unit_id);
            continue;
        };
        if ctx
            .db
            .participant()
            .connection_id()
            .find(unit.owner)
            .is_none()
        {
            continue;
        }
        if tower_moved.contains(&unit.id) {
            continue;
        }
        if ctx
            .db
            .tower_travel()
            .unit_id()
            .find(unit.id)
            .is_some_and(|t| t.leaving)
        {
            continue;
        }
        if let Some(id) = order.attack_building {
            let Some(mut building) = ctx.db.building().id().find(id) else {
                stop_unit(ctx, unit);
                continue;
            };
            let shape = building_shape(ctx, &building);
            if order.waypoints.is_empty()
                && map::distance((unit.x, unit.z), (unit.target_x, unit.target_z)) < 0.05
                && map::distance((unit.x, unit.z), shape.closest((unit.x, unit.z)))
                    <= map::ATTACK_RANGE
            {
                building.health = (building.health - 0.05 * TICK_SECONDS).max(0.0);
                if building.health <= 0.0 {
                    delete_building(ctx, id);
                    changed_games.insert(building.game_id);
                    // Replanning is done after this tick to avoid updating a stale
                    // copy of another unit's route while iterating orders.
                } else {
                    ctx.db.building().id().update(building);
                }
                continue;
            }
        }
        if is_archer(ctx, &unit) && unit_combat_facing(ctx, &unit).is_some() {
            stop_unit(ctx, unit);
            continue;
        }
        let peers = ctx
            .db
            .army_unit()
            .game_id()
            .filter(unit.game_id)
            .filter(|u| u.id != unit.id && on_ground(ctx, u))
            .map(|u| (u.x, u.z))
            .collect::<Vec<_>>();
        let obstacles = worker_obstacles(ctx, &unit);
        let mut budget = unit_speed(ctx, &unit) * TICK_SECONDS;
        while order.waypoints.len() >= 2 && budget > 0.00001 {
            while order.waypoints.len() >= 4
                && can_skip_corner(
                    (unit.x, unit.z),
                    (order.waypoints[0], order.waypoints[1]),
                    (order.waypoints[2], order.waypoints[3]),
                    &obstacles,
                )
            {
                order.waypoints.drain(..2);
            }

            let goal = (order.waypoints[0], order.waypoints[1]);
            let start = (unit.x, unit.z);
            let (x, z) = move_separated(start, goal, budget, &peers, &obstacles);
            unit.x = x;
            unit.z = z;
            budget -= map::distance(start, (x, z));
            if map::distance((x, z), goal) < 0.00001 {
                order.waypoints.drain(..2);
            } else {
                break;
            }
        }
        unit.y = ground_height(ctx, unit.game_id, unit.x, unit.z) + 0.75;
        if order.waypoints.is_empty() && order.attack_building.is_none() {
            stop_unit(ctx, unit);
        } else {
            ctx.db.army_unit().id().update(unit);
            ctx.db.unit_order().unit_id().update(order);
        }
    }
    update_unit_activity(ctx, &previous_positions);
    for game_id in changed_games {
        replan(ctx, game_id);
    }
    if ctx.db.unit_order().iter().next().is_none() && ctx.db.tower_travel().iter().next().is_none()
    {
        ctx.db
            .movement_tick()
            .scheduled_id()
            .delete(tick.scheduled_id);
    }
    Ok(())
}

fn participant(ctx: &ReducerContext) -> Result<Participant, String> {
    let connection = ctx
        .connection_id
        .ok_or("A client connection is required.")?;
    let player = ctx
        .db
        .participant()
        .connection_id()
        .find(connection)
        .ok_or("Join a game first.")?;
    if player.identity != ctx.sender {
        return Err("This connection belongs to another player.".into());
    }
    Ok(player)
}
fn owned_units(
    ctx: &ReducerContext,
    player: &Participant,
    mut ids: Vec<u64>,
) -> Result<Vec<ArmyUnit>, String> {
    if ids.is_empty() || ids.len() > map::units::MAX_COMBAT_UNITS {
        return Err("Select between 1 and 200 units.".into());
    }
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|id| {
            let u = ctx.db.army_unit().id().find(id).ok_or("Unknown unit.")?;
            if u.game_id != player.game_id || u.owner != player.connection_id {
                return Err("You can only command your own units in your game.".into());
            }
            if ctx.db.worker().unit_id().find(id).is_some() {
                return Err("Transportbauern sind nicht steuerbar.".into());
            }
            ctx.db.unit_engagement().unit_id().delete(u.id);
            ctx.db.ranged_building_order().unit_id().delete(u.id);
            ctx.db.combat_surface_home().unit_id().delete(u.id);
            Ok(u)
        })
        .collect()
}
fn rotation(ctx: &ReducerContext, building_id: u64) -> u8 {
    ctx.db
        .building_rotation()
        .building_id()
        .find(building_id)
        .map_or(0, |row| row.quarter_turns)
}
fn delete_building(ctx: &ReducerContext, building_id: u64) {
    ctx.db.civic_policy().building_id().delete(building_id);
    ctx.db.close_gate_order().building_id().delete(building_id);
    ctx.db.keep_population().building_id().delete(building_id);
    ctx.db.settlement_needs().building_id().delete(building_id);
    ctx.db
        .settlement_progress()
        .building_id()
        .delete(building_id);
    ctx.db
        .building_operation()
        .building_id()
        .delete(building_id);
    remove_fort_building(ctx, building_id);
    remove_ecology_building(ctx, building_id);
    remove_logistics_building(ctx, building_id);
    cancel_surface_routes_through(ctx, building_id);
    ctx.db.building_rotation().building_id().delete(building_id);
    ctx.db.building().id().delete(building_id);
    evacuate_tower(ctx, building_id);
}
fn building_shape(ctx: &ReducerContext, building: &Building) -> Obstacle {
    map::BuildingKind::from_name(&building.kind)
        .unwrap_or_default()
        .rotated_obstacle(building.x, building.z, rotation(ctx, building.id))
}
fn obstacles(ctx: &ReducerContext, game_id: u64) -> Vec<Obstacle> {
    fort_obstacles(ctx, game_id)
}
fn save_order(ctx: &ReducerContext, u: &ArmyUnit, attack: Option<u64>, path: Vec<map::Point>) {
    ctx.db.unit_order().unit_id().delete(u.id);
    ctx.db.unit_order().insert(UnitOrder {
        unit_id: u.id,
        game_id: u.game_id,
        attack_building: attack,
        waypoints: path.into_iter().flat_map(|p| [p.0, p.1]).collect(),
    });
}
fn stop_unit(ctx: &ReducerContext, mut u: ArmyUnit) {
    ctx.db.unit_order().unit_id().delete(u.id);
    u.target_x = u.x;
    u.target_z = u.z;
    ctx.db.army_unit().id().update(u);
}
fn free_destination(
    desired: map::Point,
    reserved: &[map::Point],
    obstacles: &[Obstacle],
) -> Option<map::Point> {
    let available = |p: map::Point| {
        map::free(p, obstacles)
            && reserved
                .iter()
                .all(|q| map::distance(p, *q) >= map::UNIT_SPACING)
    };
    if available(desired) {
        return Some(desired);
    }
    for ring in 1..=12 {
        let radius = ring as f32 * 0.6;
        for i in 0..32 {
            let angle = i as f32 * std::f32::consts::TAU / 32.0;
            let p = (
                desired.0 + angle.cos() * radius,
                desired.1 + angle.sin() * radius,
            );
            if available(p) {
                return Some(p);
            }
        }
    }
    None
}

fn attack_plans(
    ctx: &ReducerContext,
    units: &[ArmyUnit],
    b: &Building,
    keep: bool,
) -> Option<Vec<(u64, Vec<map::Point>)>> {
    let ids: std::collections::HashSet<_> = units.iter().map(|u| u.id).collect();
    let occupied = ctx
        .db
        .army_unit()
        .game_id()
        .filter(b.game_id)
        .filter(|u| !ids.contains(&u.id) && on_ground(ctx, u))
        .map(|u| (u.x, u.z))
        .collect::<Vec<_>>();
    let reserved = ctx
        .db
        .unit_order()
        .game_id()
        .filter(b.game_id)
        .filter(|o| o.attack_building == Some(b.id) && !ids.contains(&o.unit_id))
        .filter_map(|o| ctx.db.army_unit().id().find(o.unit_id))
        .map(|u| (u.target_x, u.target_z))
        .collect::<Vec<_>>();
    let starts = units
        .iter()
        .map(|u| {
            (
                u.id,
                ground_start(ctx, u),
                keep.then_some((u.target_x, u.target_z)),
            )
        })
        .collect::<Vec<_>>();
    plan_attack(
        &starts,
        building_shape(ctx, &b),
        &obstacles(ctx, b.game_id),
        &reserved,
        &occupied,
    )
}
fn apply_attack_plans(ctx: &ReducerContext, building_id: u64, plans: Vec<(u64, Vec<map::Point>)>) {
    for (id, path) in plans {
        if let Some(mut unit) = ctx.db.army_unit().id().find(id) {
            if let Some(goal) = path.last() {
                unit.target_x = goal.0;
                unit.target_z = goal.1;
            }
            save_order(ctx, &unit, Some(building_id), path);
            ctx.db.army_unit().id().update(unit);
        }
    }
}
fn replan(ctx: &ReducerContext, game_id: u64) {
    let obstacles = obstacles(ctx, game_id);
    let mut attacks = std::collections::BTreeMap::<u64, Vec<ArmyUnit>>::new();
    for order in ctx
        .db
        .unit_order()
        .game_id()
        .filter(game_id)
        .collect::<Vec<_>>()
    {
        let Some(u) = ctx.db.army_unit().id().find(order.unit_id) else {
            ctx.db.unit_order().unit_id().delete(order.unit_id);
            continue;
        };
        if let Some(id) = order.attack_building {
            attacks.entry(id).or_default().push(u);
            continue;
        }
        if ctx.db.tower_occupant().unit_id().find(u.id).is_some()
            && !ctx
                .db
                .tower_travel()
                .unit_id()
                .find(u.id)
                .is_some_and(|t| t.leaving)
        {
            replan_tower_approach(ctx, u);
            continue;
        }
        if let Some(path) =
            map::shortest_path(ground_start(ctx, &u), (u.target_x, u.target_z), &obstacles)
        {
            save_order(ctx, &u, None, path);
        } else {
            stop_unit(ctx, u);
        }
    }
    for (id, mut units) in attacks {
        units.sort_by_key(|u| u.id);
        let plans = ctx
            .db
            .building()
            .id()
            .find(id)
            .and_then(|b| attack_plans(ctx, &units, &b, true));
        if let Some(plans) = plans {
            apply_attack_plans(ctx, id, plans);
        } else {
            for unit in units {
                stop_unit(ctx, unit);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn build_granary(ctx: &ReducerContext, x: f32, z: f32) -> Result<(), String> {
    build(ctx, x, z, "granary".into())
}

#[spacetimedb::reducer]
pub fn build_barracks(ctx: &ReducerContext, x: f32, z: f32) -> Result<(), String> {
    build(ctx, x, z, "barracks".into())
}

#[spacetimedb::reducer]
pub fn build_stockpile(ctx: &ReducerContext, x: f32, z: f32) -> Result<(), String> {
    build(ctx, x, z, "stockpile".into())
}

// Legacy reducers above remain compatible with already published clients.
#[spacetimedb::reducer]
pub fn build(ctx: &ReducerContext, x: f32, z: f32, building: String) -> Result<(), String> {
    construct(ctx, x, z, building, 0)
}

#[spacetimedb::reducer]
pub fn build_rotated(
    ctx: &ReducerContext,
    x: f32,
    z: f32,
    building: String,
    quarter_turns: u8,
) -> Result<(), String> {
    construct(ctx, x, z, building, quarter_turns)
}

fn construct(
    ctx: &ReducerContext,
    x: f32,
    z: f32,
    building: String,
    quarter_turns: u8,
) -> Result<(), String> {
    ensure_resources(ctx, &participant(ctx)?);
    place_building(ctx, x, z, building, quarter_turns)?;
    replan(ctx, participant(ctx)?.game_id);
    Ok(())
}

/// One atomic reducer places the complete line, or leaves the map unchanged.
#[spacetimedb::reducer]
pub fn build_wall(
    ctx: &ReducerContext,
    start_x: f32,
    start_z: f32,
    end_x: f32,
    end_z: f32,
) -> Result<(), String> {
    build_wall_kind(ctx, start_x, start_z, end_x, end_z, "stone_wall".into())
}

fn place_building(
    ctx: &ReducerContext,
    x: f32,
    z: f32,
    building: String,
    quarter_turns: u8,
) -> Result<(), String> {
    place_building_for(ctx, &participant(ctx)?, x, z, building, quarter_turns)
}

fn place_building_for(
    ctx: &ReducerContext,
    player: &Participant,
    x: f32,
    z: f32,
    building: String,
    quarter_turns: u8,
) -> Result<(), String> {
    if quarter_turns > 3 {
        return Err("Rotation must be 0, 1, 2 or 3 quarter turns.".into());
    }
    let kind = map::BuildingKind::from_name(&building).ok_or("Unknown building type.")?;
    if kind == map::BuildingKind::Keep
        && ctx
            .db
            .player_keep_spawn()
            .connection_id()
            .find(player.connection_id)
            .is_some()
    {
        return Err(
            "Der Palast ist das einmalige Startgebäude und kann nicht gebaut werden.".into(),
        );
    }
    if kind == map::BuildingKind::Keep
        && ctx
            .db
            .building()
            .owner()
            .filter(player.connection_id)
            .any(|b| b.kind == "keep")
    {
        return Err("Jeder Spieler erhält genau einen Startpalast.".into());
    }
    let neighbours = ctx
        .db
        .building()
        .game_id()
        .filter(player.game_id)
        .filter_map(|b| {
            Some((
                map::BuildingKind::from_name(&b.kind)?,
                b.x,
                b.z,
                rotation(ctx, b.id),
            ))
        })
        .collect::<Vec<_>>();
    let (x, z) = map::placement::snap(kind, (x, z), quarter_turns, &neighbours);
    let shape = kind.construction_bounds(x, z, quarter_turns);
    let candidate = fort::Structure {
        id: 0,
        kind,
        x,
        y: 0.,
        z,
        rotation: quarter_turns,
        owner: player.color,
    };
    let existing = ctx
        .db
        .building()
        .game_id()
        .filter(player.game_id)
        .collect::<Vec<_>>();
    if kind.is_wall()
        && existing.iter().any(|b| {
            b.owner == player.connection_id && fort::covered_wall((x, z), fort_piece(ctx, b))
        })
    {
        return Ok(());
    }
    let replaced = existing
        .iter()
        .filter(|b| {
            b.owner == player.connection_id && fort::replaced_wall(candidate, fort_piece(ctx, b))
        })
        .collect::<Vec<_>>();

    ensure_ecology(ctx, player.game_id);
    let required = match kind {
        map::BuildingKind::Quarry => Some(1),
        map::BuildingKind::IronMine => Some(2),
        map::BuildingKind::Fishpond => Some(7),
        _ => None,
    };
    if let Some(resource) = required {
        if !ctx
            .db
            .resource_node()
            .game_id()
            .filter(player.game_id)
            .any(|n| {
                n.kind == resource
                    && n.remaining > 0
                    && map::distance((x, z), (n.x, n.z)) < if resource == 7 { 34. } else { 17. }
            })
        {
            return Err("Hier gibt es keinen verfügbaren Rohstoffbestand bzw. Fischgrund.".into());
        }
    }

    if !shape.in_build_area() {
        return Err("Place the building inside the map.".into());
    }
    let wall = kind.is_wall();
    let limit = if wall {
        map::walls::MAX_WALL_SEGMENTS
    } else {
        128
    };
    if ctx
        .db
        .building()
        .game_id()
        .filter(player.game_id)
        .filter(|b| map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_wall()) == wall)
        .count()
        >= limit
    {
        return Err(format!(
            "This game has reached the {limit}-{} limit.",
            if wall { "wall-segment" } else { "building" }
        ));
    }
    if existing.iter().any(|b| {
        shape.overlaps(
            map::BuildingKind::from_name(&b.kind)
                .unwrap_or_default()
                .construction_bounds(b.x, b.z, rotation(ctx, b.id)),
        ) && !(b.owner == player.connection_id
            && (fort::replaced_wall(candidate, fort_piece(ctx, b))
                || fort::may_join(candidate, fort_piece(ctx, b))))
    }) {
        return Err("Gebäude dürfen sich nicht überschneiden.".into());
    }

    let mut planned = existing
        .iter()
        .map(|b| fort_piece(ctx, b))
        .collect::<Vec<_>>();
    planned.push(candidate);
    let extensions = fort::joins(&planned)
        .into_iter()
        .filter(|j| j.a == 0 || j.b == 0)
        .collect::<Vec<_>>();
    if ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .any(|u| {
            (on_ground(ctx, &u)
                && (kind.rotated_blocks_construction_unit(x, z, quarter_turns, (u.x, u.z))
                    || extensions
                        .iter()
                        .any(|j| j.shape.contains((u.x, u.z), map::CLEARANCE))))
                || ctx
                    .db
                    .tower_occupant()
                    .unit_id()
                    .find(u.id)
                    .is_some_and(|o| replaced.iter().any(|b| b.id == o.building_id))
        })
    {
        return Err("Units are standing in the construction area.".into());
    }
    if ctx
        .db
        .moat()
        .game_id()
        .filter(player.game_id)
        .any(|m| moat_shape(&m).overlaps(shape))
        && kind != map::BuildingKind::DrawbridgeGate
    {
        return Err("Hier liegt ein Graben.".into());
    }
    if ctx
        .db
        .wall_stair()
        .game_id()
        .filter(player.game_id)
        .any(|s| stair_shape(&s).overlaps(shape))
    {
        return Err("Hier liegt eine Mauertreppe.".into());
    }
    let y = map::terrain::construction_height(kind, x, z, quarter_turns).ok_or(
        "Unpassendes Gelände: Holzfäller brauchen Wald, Steinbrüche Stein und Eisenminen Erz. Normale Gebäude dürfen auf Land, aber nicht auf Fels oder Wasser stehen.")?;
    let refund = replaced.iter().fold(map::Cost::new(0, 0, 0), |sum, old| {
        let cost = map::BuildingKind::from_name(&old.kind).unwrap().cost();
        map::Cost::new(
            sum.wood + cost.wood,
            sum.stone + cost.stone,
            sum.iron + cost.iron,
        )
    });
    // Settle only the net cost, so replacement also works when a warehouse is full.
    // Only the automatic one-time starting palace is free.
    let cost = if kind == map::BuildingKind::Keep
        && ctx
            .db
            .player_keep_spawn()
            .connection_id()
            .find(player.connection_id)
            .is_none()
    {
        map::Cost::new(0, 0, 0)
    } else {
        kind.cost()
    };
    debit_cost(
        ctx,
        player,
        map::Cost::new(
            cost.wood.saturating_sub(refund.wood),
            cost.stone.saturating_sub(refund.stone),
            cost.iron.saturating_sub(refund.iron),
        ),
    )?;
    exchange_goods(ctx, player, 0, refund.wood.saturating_sub(cost.wood), true)?;
    exchange_goods(
        ctx,
        player,
        1,
        refund.stone.saturating_sub(cost.stone),
        true,
    )?;
    exchange_goods(ctx, player, 2, refund.iron.saturating_sub(cost.iron), true)?;
    for old in &replaced {
        delete_building(ctx, old.id);
    }
    let placed = ctx.db.building().insert(Building {
        id: 0,
        game_id: player.game_id,
        owner: player.connection_id,
        color: player.color,
        kind: kind.name().into(),
        x,
        y,
        z,
        health: 1.0,
    });
    ctx.db.building_rotation().insert(BuildingRotation {
        building_id: placed.id,
        game_id: placed.game_id,
        quarter_turns,
    });
    register_logistics_building(ctx, &placed)?;
    if kind.is_gate() {
        ctx.db.gate_state().insert(GateState {
            building_id: placed.id,
            game_id: placed.game_id,
            open: false,
        });
    }
    if kind == map::BuildingKind::Keep {
        register_keep(ctx, &placed);
    }
    let cleared = map::terrain::clearing_shape(kind, x, z, quarter_turns);
    ctx.db.terrain_clearing().insert(TerrainClearing {
        id: 0,
        game_id: player.game_id,
        x,
        z,
        half_x: cleared.half_x,
        half_z: cleared.half_z,
    });
    clear_resource_trees(ctx, player.game_id, cleared);
    auto_assign_peasants(ctx, player.connection_id);
    Ok(())
}

#[spacetimedb::reducer]
pub fn remove_building(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let player = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .ok_or("Unknown building.")?;
    if b.owner != player.connection_id || b.game_id != player.game_id {
        return Err("You can only remove your own buildings.".into());
    }
    delete_building(ctx, building_id);
    replan(ctx, player.game_id);
    Ok(())
}

#[spacetimedb::reducer]
pub fn attack_building(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    building_id: u64,
) -> Result<(), String> {
    let player = participant(ctx)?;
    let units = owned_units(ctx, &player, unit_ids)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .ok_or("Unknown building.")?;
    if b.game_id != player.game_id || b.owner == player.connection_id {
        return Err("Choose an enemy building in your game.".into());
    }
    let mut melee_units = Vec::new();
    for mut unit in units {
        if is_archer(ctx, &unit) {
            if !approach_bow_target(ctx, &mut unit, &BowTarget::Building(b.clone())) {
                return Err("No reachable firing position.".into());
            }
            ctx.db.ranged_building_order().insert(RangedBuildingOrder {
                unit_id: unit.id,
                building_id: b.id,
            });
        } else {
            depart_tower(ctx, &unit)?;
            melee_units.push(unit);
        }
    }
    if !melee_units.is_empty() {
        let plans = attack_plans(ctx, &melee_units, &b, false)
            .ok_or("No reachable space around this building.")?;
        apply_attack_plans(ctx, building_id, plans);
    }
    start_combat_scan(ctx);
    start_tick(ctx);
    Ok(())
}

include!("walls.rs");
include!("tower.rs");

include!("logistics.rs");

include!("units.rs");

include!("combat.rs");

include!("combat_surfaces.rs");

include!("ranged.rs");
include!("keep.rs");
include!("settlement.rs");
include!("ecology.rs");

include!("fortifications.rs");

#[spacetimedb::reducer]
pub fn build_wall_kind(
    ctx: &ReducerContext,
    start_x: f32,
    start_z: f32,
    end_x: f32,
    end_z: f32,
    kind: String,
) -> Result<(), String> {
    let p = participant(ctx)?;
    let k = map::BuildingKind::from_name(&kind)
        .filter(|k| k.is_wall())
        .ok_or("Ungültige Mauerart.")?;
    let line = map::walls::wall_line((start_x, start_z), (end_x, end_z));
    if line.is_empty() || line.len() > 512 {
        return Err("Ungültige Mauerlinie.".into());
    }
    for (x, z) in line {
        if ctx
            .db
            .building()
            .game_id()
            .filter(p.game_id)
            .any(|b| b.kind == k.name() && b.owner == p.connection_id && b.x == x && b.z == z)
        {
            continue;
        }
        place_building(ctx, x, z, kind.clone(), 0)?;
    }
    replan(ctx, p.game_id);
    Ok(())
}

include!("saves.rs");

include!("civic.rs");
