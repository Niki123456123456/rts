use crate::simulation::{step_towards, terrain_height};
use spacetimedb::{Identity, ReducerContext, ScheduleAt, Table};
use std::time::Duration;

const TICK_SECONDS: f32 = 0.05;
const UNIT_SPEED: f32 = 3.0;
use stronghold_map::{self as map, Obstacle, MAP_LIMIT};

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
            return Ok(());
        }
    }
    leave_game(ctx);
    let used_colors: std::collections::HashSet<_> = ctx
        .db
        .participant()
        .game_id()
        .filter(game_id)
        .map(|player| player.color)
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
    // Each army gets a separate formation inside the map.
    let axis = |slot: u32| {
        if slot == 0 {
            0.0
        } else {
            slot.div_ceil(2) as f32 * if slot % 2 == 1 { 9.0 } else { -9.0 }
        }
    };
    let origin_x = axis(color % 7) - 1.4;
    let origin_z = axis((color / 7) % 7) - 1.05;
    for number in 1..=20 {
        let mut x = origin_x + ((number - 1) % 5) as f32 * 0.7;
        let mut z = origin_z + ((number - 1) / 5) as f32 * 0.7;
        let obstacles = obstacles(ctx, game_id);
        if !map::free((x, z), &obstacles) {
            let p = (-55..=55)
                .flat_map(|i| (-55..=55).map(move |j| (i as f32 * 0.7, j as f32 * 0.7)))
                .filter(|p| map::free(*p, &obstacles))
                .filter(|p| {
                    !ctx.db
                        .army_unit()
                        .game_id()
                        .filter(game_id)
                        .any(|u| map::distance(*p, (u.x, u.z)) < 0.6)
                })
                .min_by(|a, b| map::distance(*a, (x, z)).total_cmp(&map::distance(*b, (x, z))))
                .ok_or("No room for this army.")?;
            x = p.0;
            z = p.1;
        }
        ctx.db.army_unit().insert(ArmyUnit {
            id: 0,
            game_id,
            owner: connection_id,
            color,
            number,
            health: 1.0,
            x,
            y: terrain_height(x, z) + 0.75,
            z,
            target_x: x,
            target_z: z,
        });
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_game(ctx: &ReducerContext) {
    if let Some(connection_id) = ctx.connection_id {
        if let Some(player) = ctx.db.participant().connection_id().find(connection_id) {
            if player.identity != ctx.sender {
                return;
            }
            let units: Vec<_> = ctx.db.army_unit().owner().filter(connection_id).collect();
            for unit in units {
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
                ctx.db.building().id().delete(building.id);
            }
            replan(ctx, player.game_id);
            ctx.db.participant().connection_id().delete(connection_id);
        }
        ctx.db.player().connection_id().delete(connection_id);
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
    let center_x = units.iter().map(|u| u.x).sum::<f32>() / units.len() as f32;
    let center_z = units.iter().map(|u| u.z).sum::<f32>() / units.len() as f32;
    for mut unit in units {
        // Formation offsets use authoritative positions, never client positions.
        unit.target_x = (target_x + unit.x - center_x).clamp(-MAP_LIMIT, MAP_LIMIT);
        unit.target_z = (target_z + unit.z - center_z).clamp(-MAP_LIMIT, MAP_LIMIT);
        let path = map::shortest_path((unit.x, unit.z), (unit.target_x, unit.target_z), &obstacles)
            .ok_or("Target is blocked or unreachable. Choose open ground.")?;
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
    // Upgrade any in-flight orders from the earlier movement implementation.
    for unit in ctx.db.army_unit().iter() {
        if (unit.x != unit.target_x || unit.z != unit.target_z)
            && ctx.db.unit_order().unit_id().find(unit.id).is_none()
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
    let mut changed_games = std::collections::HashSet::new();
    for mut order in ctx.db.unit_order().iter().collect::<Vec<_>>() {
        let Some(mut unit) = ctx.db.army_unit().id().find(order.unit_id) else {
            ctx.db.unit_order().unit_id().delete(order.unit_id);
            continue;
        };
        if let Some(id) = order.attack_building {
            let Some(mut building) = ctx.db.building().id().find(id) else {
                stop_unit(ctx, unit);
                continue;
            };
            let shape = Obstacle {
                x: building.x,
                z: building.z,
            };
            if map::distance((unit.x, unit.z), shape.closest((unit.x, unit.z))) <= 0.8 {
                building.health = (building.health - 0.05 * TICK_SECONDS).max(0.0);
                if building.health <= 0.0 {
                    ctx.db.building().id().delete(id);
                    changed_games.insert(building.game_id);
                    // Replanning is done after this tick to avoid updating a stale
                    // copy of another unit's route while iterating orders.
                } else {
                    ctx.db.building().id().update(building);
                }
                continue;
            }
        }
        let mut budget = UNIT_SPEED * TICK_SECONDS;
        while order.waypoints.len() >= 2 && budget > 0.0 {
            let goal = (order.waypoints[0], order.waypoints[1]);
            let distance = map::distance((unit.x, unit.z), goal);
            let (x, z) = step_towards(unit.x, unit.z, goal.0, goal.1, budget);
            unit.x = x;
            unit.z = z;
            budget -= distance;
            if distance <= UNIT_SPEED * TICK_SECONDS && (x, z) == goal {
                order.waypoints.drain(..2);
            } else {
                break;
            }
        }
        unit.y = terrain_height(unit.x, unit.z) + 0.75;
        if order.waypoints.is_empty() && order.attack_building.is_none() {
            stop_unit(ctx, unit);
        } else {
            ctx.db.army_unit().id().update(unit);
            ctx.db.unit_order().unit_id().update(order);
        }
    }
    for game_id in changed_games {
        replan(ctx, game_id);
    }
    if ctx.db.unit_order().iter().next().is_none() {
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
    if ids.is_empty() || ids.len() > 20 {
        return Err("Select between 1 and 20 units.".into());
    }
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|id| {
            let u = ctx.db.army_unit().id().find(id).ok_or("Unknown unit.")?;
            if u.game_id != player.game_id || u.owner != player.connection_id {
                return Err("You can only command your own units in your game.".into());
            }
            Ok(u)
        })
        .collect()
}
fn obstacles(ctx: &ReducerContext, game_id: u64) -> Vec<Obstacle> {
    ctx.db
        .building()
        .game_id()
        .filter(game_id)
        .map(|b| Obstacle { x: b.x, z: b.z })
        .collect()
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
fn replan(ctx: &ReducerContext, game_id: u64) {
    let obstacles = obstacles(ctx, game_id);
    for order in ctx
        .db
        .unit_order()
        .game_id()
        .filter(game_id)
        .collect::<Vec<_>>()
    {
        let Some(mut u) = ctx.db.army_unit().id().find(order.unit_id) else {
            ctx.db.unit_order().unit_id().delete(order.unit_id);
            continue;
        };
        let path =
            if let Some(id) = order.attack_building {
                ctx.db.building().id().find(id).and_then(|b| {
                    map::attack_path((u.x, u.z), Obstacle { x: b.x, z: b.z }, &obstacles)
                })
            } else {
                map::shortest_path((u.x, u.z), (u.target_x, u.target_z), &obstacles)
            };
        if let Some(path) = path {
            if let Some(goal) = path.last() {
                u.target_x = goal.0;
                u.target_z = goal.1;
            }
            save_order(ctx, &u, order.attack_building, path);
            ctx.db.army_unit().id().update(u);
        } else {
            stop_unit(ctx, u);
        }
    }
}

#[spacetimedb::reducer]
pub fn build_granary(ctx: &ReducerContext, x: f32, z: f32) -> Result<(), String> {
    let player = participant(ctx)?;
    let shape = Obstacle { x, z };
    if !shape.in_map() {
        return Err("Place the granary inside the map.".into());
    }
    if ctx.db.building().game_id().filter(player.game_id).count() >= 128 {
        return Err("This game has reached the 128-building limit.".into());
    }
    if obstacles(ctx, player.game_id)
        .iter()
        .any(|b| shape.overlaps(*b))
    {
        return Err("Buildings cannot overlap.".into());
    }
    if ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .any(|u| shape.contains((u.x, u.z), map::CLEARANCE))
    {
        return Err("Units are standing in the construction area.".into());
    }
    let y = terrain_height(x, z);
    if [-map::HALF_X, 0.0, map::HALF_X].into_iter().any(|dx| {
        [-map::HALF_Z, 0.0, map::HALF_Z]
            .into_iter()
            .any(|dz| (terrain_height(x + dx, z + dz) - y).abs() > 0.05)
    }) {
        return Err("Place the granary on level ground.".into());
    }
    ctx.db.building().insert(Building {
        id: 0,
        game_id: player.game_id,
        owner: player.connection_id,
        color: player.color,
        kind: "granary".into(),
        x,
        y,
        z,
        health: 1.0,
    });
    replan(ctx, player.game_id);
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
    ctx.db.building().id().delete(building_id);
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
    let obstacles = obstacles(ctx, player.game_id);
    for mut u in units {
        let path = map::attack_path((u.x, u.z), Obstacle { x: b.x, z: b.z }, &obstacles)
            .ok_or("This building is unreachable.")?;
        if let Some(goal) = path.last() {
            u.target_x = goal.0;
            u.target_z = goal.1;
        }
        save_order(ctx, &u, Some(building_id), path);
        ctx.db.army_unit().id().update(u);
    }
    start_tick(ctx);
    Ok(())
}
