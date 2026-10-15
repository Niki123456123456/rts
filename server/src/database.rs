use crate::simulation::{step_towards, terrain_height};
use spacetimedb::{Identity, ReducerContext, ScheduleAt, Table};
use std::time::Duration;

const TICK_SECONDS: f32 = 0.05;
const UNIT_SPEED: f32 = 3.0;
const MAP_LIMIT: f32 = 39.0;

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

#[spacetimedb::table(name = movement_tick, scheduled(move_units))]
pub struct MovementTick {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    ctx.db.movement_tick().insert(MovementTick {
        scheduled_id: 0,
        scheduled_at: Duration::from_millis(50).into(),
    });
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
    for x in -5..5 {
        for z in -5..5 {
            let (px, pz) = (x as f32 * 0.7, z as f32 * 0.7);
            ctx.db.unit().insert(Unit {
                id: 0,
                game_id: game.id,
                number: ((x + 5) * 10 + z + 6) as u32,
                x: px,
                y: terrain_height(px, pz) + 0.75,
                z: pz,
                target_x: px,
                target_z: pz,
            });
        }
    }
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
    ctx.db.player().connection_id().delete(connection_id);
    ctx.db.player().insert(Player {
        connection_id,
        identity: ctx.sender,
        game_id,
    });
    Ok(())
}

#[spacetimedb::reducer]
pub fn leave_game(ctx: &ReducerContext) {
    if let Some(connection_id) = ctx.connection_id {
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
    let connection_id = ctx
        .connection_id
        .ok_or("A client connection is required.")?;
    let player = ctx
        .db
        .player()
        .connection_id()
        .find(connection_id)
        .ok_or("Join a game first.")?;
    if unit_ids.is_empty() || unit_ids.len() > 100 {
        return Err("Select between 1 and 100 units.".into());
    }
    let mut ids = unit_ids;
    ids.sort_unstable();
    ids.dedup();
    let mut units = Vec::with_capacity(ids.len());
    for id in ids {
        let unit = ctx.db.unit().id().find(id).ok_or("Unknown unit.")?;
        if unit.game_id != player.game_id {
            return Err("You can only command units in your game.".into());
        }
        units.push(unit);
    }
    let center_x = units.iter().map(|u| u.x).sum::<f32>() / units.len() as f32;
    let center_z = units.iter().map(|u| u.z).sum::<f32>() / units.len() as f32;
    for mut unit in units {
        // Formation offsets use authoritative positions, never client positions.
        unit.target_x = (target_x + unit.x - center_x).clamp(-MAP_LIMIT, MAP_LIMIT);
        unit.target_z = (target_z + unit.z - center_z).clamp(-MAP_LIMIT, MAP_LIMIT);
        ctx.db.unit().id().update(unit);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn move_units(ctx: &ReducerContext, _tick: MovementTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Movement may only be run by the server scheduler.".into());
    }
    for mut unit in ctx.db.unit().iter() {
        let (x, z) = step_towards(
            unit.x,
            unit.z,
            unit.target_x,
            unit.target_z,
            UNIT_SPEED * TICK_SECONDS,
        );
        if x == unit.x && z == unit.z {
            continue;
        }
        unit.x = x;
        unit.z = z;
        unit.y = terrain_height(x, z) + 0.75;
        ctx.db.unit().id().update(unit);
    }
    Ok(())
}
