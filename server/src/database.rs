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
        let x = origin_x + ((number - 1) % 5) as f32 * 0.7;
        let z = origin_z + ((number - 1) / 5) as f32 * 0.7;
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
                ctx.db.army_unit().id().delete(unit.id);
            }
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
    let connection_id = ctx
        .connection_id
        .ok_or("A client connection is required.")?;
    let player = ctx
        .db
        .participant()
        .connection_id()
        .find(connection_id)
        .ok_or("Join a game first.")?;
    if player.identity != ctx.sender {
        return Err("This connection belongs to another player.".into());
    }
    if unit_ids.is_empty() || unit_ids.len() > 20 {
        return Err("Select between 1 and 20 units.".into());
    }
    let mut ids = unit_ids;
    ids.sort_unstable();
    ids.dedup();
    let mut units = Vec::with_capacity(ids.len());
    for id in ids {
        let unit = ctx.db.army_unit().id().find(id).ok_or("Unknown unit.")?;
        if unit.game_id != player.game_id {
            return Err("You can only command units in your game.".into());
        }
        if unit.owner != connection_id {
            return Err("You can only command your own units.".into());
        }
        units.push(unit);
    }
    let center_x = units.iter().map(|u| u.x).sum::<f32>() / units.len() as f32;
    let center_z = units.iter().map(|u| u.z).sum::<f32>() / units.len() as f32;
    for mut unit in units {
        // Formation offsets use authoritative positions, never client positions.
        unit.target_x = (target_x + unit.x - center_x).clamp(-MAP_LIMIT, MAP_LIMIT);
        unit.target_z = (target_z + unit.z - center_z).clamp(-MAP_LIMIT, MAP_LIMIT);
        ctx.db.army_unit().id().update(unit);
    }
    // No background calls while idle: important for usage-based cloud hosting.
    if ctx.db.movement_tick().iter().next().is_none() {
        ctx.db.movement_tick().insert(MovementTick {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(50).into(),
        });
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn move_units(ctx: &ReducerContext, tick: MovementTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Movement may only be run by the server scheduler.".into());
    }
    let mut moving = false;
    for mut unit in ctx.db.army_unit().iter() {
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
        moving |= x != unit.target_x || z != unit.target_z;
        ctx.db.army_unit().id().update(unit);
    }
    if !moving {
        ctx.db
            .movement_tick()
            .scheduled_id()
            .delete(tick.scheduled_id);
    }
    Ok(())
}
