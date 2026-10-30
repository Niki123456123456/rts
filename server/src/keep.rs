// A start grant is separate from the building so demolition does not grant a
// new free keep on the next join/connection. Cleared when participation ends.
#[spacetimedb::table(name = player_keep_spawn)]
pub struct PlayerKeepSpawn {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
    pub game_id: u64,
}

fn ensure_player_keep(ctx: &ReducerContext, player: &Participant) -> Result<(), String> {
    if ctx
        .db
        .player_keep_spawn()
        .connection_id()
        .find(player.connection_id)
        .is_some_and(|grant| grant.game_id == player.game_id)
    {
        return Ok(());
    }
    if !ctx
        .db
        .building()
        .owner()
        .filter(player.connection_id)
        .any(|b| b.kind == "keep")
    {
        let home = ctx
            .db
            .army_unit()
            .owner()
            .filter(player.connection_id)
            .find(|u| u.number == 1)
            .map_or((0., 0.), |u| (u.x, u.z));
        // Separate plots leave room for the forecourt, stairs and ground paths.
        // Sort a finite deterministic grid by proximity to this player's lord.
        let mut candidates = (-10..=10)
            .flat_map(|x| (-10..=10).map(move |z| (x as f32 * 32., z as f32 * 36.)))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| map::distance(*a, home).total_cmp(&map::distance(*b, home)));
        let mut placed = false;
        for (x, z) in candidates {
            if place_building_for(ctx, player, x, z, "keep".into(), 0).is_ok() {
                placed = true;
                break;
            }
        }
        if !placed {
            return Err("Kein freier Bauplatz für den Startpalast vorhanden.".into());
        }
    }
    ctx.db.player_keep_spawn().insert(PlayerKeepSpawn {
        connection_id: player.connection_id,
        game_id: player.game_id,
    });
    Ok(())
}

// Additive records retain existing games and schemas during module upgrades.
#[derive(Clone)]
#[spacetimedb::table(name = keep_population, public)]
pub struct KeepPopulation {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub arrived: u32,
    pub seconds: u32,
}
#[spacetimedb::table(name = migration_tick, scheduled(arrive_at_keep))]
pub struct MigrationTick {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}
fn register_keep(ctx: &ReducerContext, b: &Building) {
    let existing_jobs = employed_peasants(ctx, b.owner);
    ctx.db.keep_population().insert(KeepPopulation {
        building_id: b.id,
        game_id: b.game_id,
        arrived: existing_jobs,
        seconds: 0,
    });
    // The player's existing lord takes his position; never create a second lord.
    if let Some(mut lord) = ctx.db.army_unit().owner().filter(b.owner).find(|u| {
        ctx.db
            .unit_profile()
            .unit_id()
            .find(u.id)
            .is_some_and(|p| p.kind == "lord")
    }) {
        clear_tower(ctx, lord.id);
        ctx.db.unit_order().unit_id().delete(lord.id);
        ctx.db.unit_engagement().unit_id().delete(lord.id);
        let tower = tower_geometry(ctx, b);
        let slot = tower.sorted_slots((b.x, b.z + 2.0))[0];
        let point = tower.slot(slot);
        lord.x = point.0;
        lord.z = point.1;
        lord.y = tower.roof_y() + roof::UNIT_CENTER_HEIGHT;
        lord.target_x = lord.x;
        lord.target_z = lord.z;
        ctx.db.tower_occupant().insert(TowerOccupant {
            unit_id: lord.id,
            game_id: b.game_id,
            building_id: b.id,
            slot: slot as u8,
            door: 0,
            next_building: None,
            next_slot: 0,
        });
        ctx.db.army_unit().id().update(lord);
    }
    settlement_satisfaction(ctx, b, existing_jobs);
    start_migration(ctx);
}
fn start_migration(ctx: &ReducerContext) {
    if ctx.db.keep_population().iter().next().is_some()
        && ctx.db.migration_tick().iter().next().is_none()
    {
        ctx.db.migration_tick().insert(MigrationTick {
            scheduled_id: 0,
            scheduled_at: Duration::from_secs(1).into(),
        });
    }
}
#[spacetimedb::reducer]
pub fn arrive_at_keep(ctx: &ReducerContext, _tick: MigrationTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Only the server schedules immigration.".into());
    }
    for mut state in ctx.db.keep_population().iter().collect::<Vec<_>>() {
        let Some(keep) = ctx.db.building().id().find(state.building_id) else {
            ctx.db
                .keep_population()
                .building_id()
                .delete(state.building_id);
            continue;
        };
        ensure_ecology(ctx, keep.game_id);
        let population = map::population::Population::from_buildings(
            ctx.db
                .building()
                .owner()
                .filter(keep.owner)
                .filter_map(|b| map::BuildingKind::from_name(&b.kind)),
        );
        state.seconds = state.seconds.saturating_add(1);
        let employed = employed_peasants(ctx, keep.owner);
        state.arrived = state.arrived.max(employed);
        if state.seconds % 30 == 0 {
            feed_population(ctx, &keep, state.arrived, state.seconds / 30);
        }
        let satisfaction = settlement_satisfaction(ctx, &keep, state.arrived);
        let old_progress = ctx.db.settlement_progress().building_id().find(keep.id);
        let mut ticks = old_progress.as_ref().map_or(0, |p| p.ticks);
        if satisfaction.immigration_seconds > 0 && state.arrived < population.capacity {
            ticks += 1;
            if ticks >= satisfaction.immigration_seconds {
                state.arrived += 1;
                ticks = 0;
            }
        } else if satisfaction.happiness < 40 && state.arrived > employed {
            ticks += 1;
            if ticks >= 14 {
                state.arrived -= 1;
                ticks = 0;
            }
        } else {
            ticks = 0;
        }
        let progress = SettlementProgress {
            building_id: keep.id,
            ticks,
        };
        if old_progress.is_some() {
            ctx.db.settlement_progress().building_id().update(progress);
        } else {
            ctx.db.settlement_progress().insert(progress);
        }
        ctx.db.keep_population().building_id().update(state);
        auto_assign_peasants(ctx, keep.owner);
    }
    start_migration(ctx);
    Ok(())
}
