// Additive metadata keeps existing army table schemas compatible with hosted games.
#[derive(Clone)]
#[spacetimedb::table(name = unit_profile, public)]
pub struct UnitProfile {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub kind: String,
    /// 0 idle, 1 walking, 2 fighting. Only the server writes activity and facing.
    pub activity: u8,
    pub facing_x: f32,
    pub facing_z: f32,
}
fn register_combat_unit(ctx: &ReducerContext, u: &ArmyUnit, kind: map::units::UnitKind) {
    ctx.db.unit_profile().insert(UnitProfile {
        unit_id: u.id,
        game_id: u.game_id,
        kind: kind.name().into(),
        activity: 0,
        facing_x: 0.,
        facing_z: -1.,
    });
    start_combat_scan(ctx);
}
fn update_unit_activity(
    ctx: &ReducerContext,
    previous: &std::collections::HashMap<u64, (f32, f32)>,
) {
    for mut profile in ctx.db.unit_profile().iter().collect::<Vec<_>>() {
        let Some(u) = ctx.db.army_unit().id().find(profile.unit_id) else {
            continue;
        };
        let old = previous.get(&u.id).copied().unwrap_or((u.x, u.z));
        let mut facing = (u.x - old.0, u.z - old.1);
        let mut activity = if map::distance((u.x, u.z), old) > 0.00001
            && (ctx.db.unit_order().unit_id().find(u.id).is_some()
                || ctx.db.tower_travel().unit_id().find(u.id).is_some())
        {
            1
        } else {
            0
        };
        if activity == 0 {
            if let Some(b) = ctx
                .db
                .unit_order()
                .unit_id()
                .find(u.id)
                .filter(|o| o.waypoints.is_empty())
                .and_then(|o| o.attack_building)
                .and_then(|id| ctx.db.building().id().find(id))
            {
                if map::distance((u.x, u.z), building_shape(ctx, &b).closest((u.x, u.z)))
                    <= map::ATTACK_RANGE
                {
                    activity = 2;
                    facing = (b.x - u.x, b.z - u.z);
                }
            }
        }
        if let Some(direction) = unit_combat_facing(ctx, &u) {
            activity = 2;
            facing = direction;
        }
        let length = map::distance((0., 0.), facing);
        let direction = if length > 0.00001 {
            (facing.0 / length, facing.1 / length)
        } else {
            (profile.facing_x, profile.facing_z)
        };
        if profile.activity != activity
            || map::distance(direction, (profile.facing_x, profile.facing_z)) > 0.001
        {
            profile.activity = activity;
            profile.facing_x = direction.0;
            profile.facing_z = direction.1;
            ctx.db.unit_profile().unit_id().update(profile);
        }
    }
}
#[spacetimedb::reducer]
pub fn recruit_unit(ctx: &ReducerContext, building_id: u64, kind: String) -> Result<(), String> {
    let player = participant(ctx)?;
    let kind = map::units::UnitKind::from_name(&kind)
        .filter(|k| *k != map::units::UnitKind::Lord)
        .ok_or("Recruit armed peasants or archers.")?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .filter(|b| {
            b.game_id == player.game_id && b.owner == player.connection_id && b.kind == "barracks"
        })
        .ok_or("Recruit at your own barracks.")?;
    let army = ctx
        .db
        .army_unit()
        .owner()
        .filter(player.connection_id)
        .filter(|u| ctx.db.worker().unit_id().find(u.id).is_none())
        .collect::<Vec<_>>();
    if army.len() >= map::units::MAX_COMBAT_UNITS {
        return Err("Army limit reached (200).".into());
    }
    let obstacles = obstacles(ctx, player.game_id);
    let shape = building_shape(ctx, &b);
    // The front is local +Z; rotation turns the preferred spawn area with the barracks.
    let angle = rotation(ctx, b.id) as f32 * std::f32::consts::FRAC_PI_2;
    let normal = (angle.sin(), angle.cos());
    let front_distance = normal.0.abs() * shape.half_x + normal.1.abs() * shape.half_z;
    let preferred = (
        b.x + normal.0 * (front_distance + 1.),
        b.z + normal.1 * (front_distance + 1.),
    );
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .flat_map(|u| ground_reservations(ctx, &u))
        .collect::<Vec<_>>();
    let mut points = map::attack_positions(shape)
        .into_iter()
        .filter(|p| {
            p.ring <= 2
                && (p.point.0 - b.x) * normal.0 + (p.point.1 - b.z) * normal.1
                    >= front_distance + 0.4
        })
        .map(|p| p.point)
        .collect::<Vec<_>>();
    points.sort_by(|a, b| map::distance(*a, preferred).total_cmp(&map::distance(*b, preferred)));
    let p = points
        .into_iter()
        .find(|p| {
            map::free(*p, &obstacles)
                && peers
                    .iter()
                    .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
        })
        .ok_or("No free space in front of the barracks.")?;
    let keep = ctx
        .db
        .building()
        .owner()
        .filter(player.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Kein eigener Palast.")?;
    let mut population = ctx
        .db
        .keep_population()
        .building_id()
        .find(keep.id)
        .ok_or("Keine Bauern vorhanden.")?;
    if population.arrived <= employed_peasants(ctx, player.connection_id) {
        return Err("Kein freier Bauer für die Rekrutierung. Schließe einen Betrieb oder warte auf Zuwanderer.".into());
    }
    debit_cost(ctx, &player, kind.cost())?;
    population.arrived -= 1;
    ctx.db.keep_population().building_id().update(population);
    let u = ctx.db.army_unit().insert(ArmyUnit {
        id: 0,
        game_id: player.game_id,
        owner: player.connection_id,
        color: player.color,
        number: army.iter().map(|u| u.number).max().unwrap_or(0) + 1,
        health: 1.,
        x: p.0,
        y: terrain_height(p.0, p.1) + 0.75,
        z: p.1,
        target_x: p.0,
        target_z: p.1,
    });
    register_combat_unit(ctx, &u, kind);
    Ok(())
}
