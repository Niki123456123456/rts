// Portable ladders/towers provide a temporary route onto an enemy fortification.
#[derive(Clone)]
#[spacetimedb::table(name=siege_access,public)]
pub struct SiegeAccess {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub building_id: u64,
}
fn deploy_access(ctx: &ReducerContext, u: &ArmyUnit, b: &Building) -> bool {
    let k = combat_kind(ctx, u);
    if !matches!(
        k,
        map::units::UnitKind::SiegeTower | map::units::UnitKind::AssaultLadder
    ) {
        return false;
    }
    let Some(kind) = map::BuildingKind::from_name(&b.kind)
        .filter(|k| k.is_wall() || k.is_tower() || k.is_gate())
    else {
        return true;
    };
    let Some(profile) = kind.roof_profile() else {
        return true;
    };
    let height = b.y + profile.height - terrain_height(u.x, u.z);
    if height
        > if k == map::units::UnitKind::AssaultLadder {
            8.5
        } else {
            12.5
        }
    {
        return true;
    }
    let row = SiegeAccess {
        unit_id: u.id,
        game_id: u.game_id,
        building_id: b.id,
    };
    if ctx.db.siege_access().unit_id().find(u.id).is_some() {
        ctx.db.siege_access().unit_id().update(row);
    } else {
        ctx.db.siege_access().insert(row);
    }
    let dx = b.x - u.x;
    let dz = b.z - u.z;
    let len = map::distance((u.x, u.z), (b.x, b.z)).max(0.01);
    if let Some(mut p) = ctx.db.unit_profile().unit_id().find(u.id) {
        p.facing_x = dx / len;
        p.facing_z = dz / len;
        p.activity = 0;
        ctx.db.unit_profile().unit_id().update(p);
    }
    stop_unit(ctx, u.clone());
    true
}
fn refresh_access(ctx: &ReducerContext) {
    for a in ctx.db.siege_access().iter().collect::<Vec<_>>() {
        let valid = ctx
            .db
            .army_unit()
            .id()
            .find(a.unit_id)
            .zip(ctx.db.building().id().find(a.building_id))
            .is_some_and(|(u, b)| {
                u.game_id == b.game_id
                    && u.owner != b.owner
                    && ctx.db.unit_order().unit_id().find(u.id).is_none()
                    && map::distance((u.x, u.z), building_shape(ctx, &b).closest((u.x, u.z)))
                        <= map::ATTACK_RANGE + 0.1
            });
        if !valid {
            ctx.db.siege_access().unit_id().delete(a.unit_id);
        }
    }
}
#[spacetimedb::reducer]
pub fn assault_roof(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    access_id: u64,
) -> Result<(), String> {
    let p = participant(ctx)?;
    let units = owned_units(ctx, &p, unit_ids)?;
    let a =
        ctx.db.siege_access().unit_id().find(access_id).ok_or(
            "Lege zuerst Leiter oder Belagerungsturm mit einem Angriffsbefehl an die Mauer.",
        )?;
    let engine = ctx
        .db
        .army_unit()
        .id()
        .find(a.unit_id)
        .filter(|u| u.owner == p.connection_id && u.game_id == p.game_id)
        .ok_or("Wähle dein Belagerungsgerät.")?;
    let b = ctx
        .db
        .building()
        .id()
        .find(a.building_id)
        .filter(|b| b.owner != p.connection_id && b.game_id == p.game_id)
        .ok_or("Der gegnerische Wehrgang fehlt.")?;
    if map::distance(
        (engine.x, engine.z),
        building_shape(ctx, &b).closest((engine.x, engine.z)),
    ) > map::ATTACK_RANGE + 0.1
    {
        return Err("Das Belagerungsgerät steht nicht mehr an der Mauer.".into());
    }
    let roof = tower_geometry(ctx, &b);
    let mut accepted = 0;
    let obs = obstacles(ctx, p.game_id);
    for mut u in units {
        let k = combat_kind(ctx, &u);
        if k.is_siege() || k == map::units::UnitKind::LightCavalry || !on_ground(ctx, &u) {
            continue;
        }
        let Some(slot) = roof
            .sorted_slots((engine.x, engine.z))
            .into_iter()
            .find(|s| !slot_reserved(ctx, b.id, *s as u8, u.id))
        else {
            continue;
        };
        let Some(route) = map::shortest_path((u.x, u.z), (engine.x, engine.z), &obs) else {
            continue;
        };
        let mut path = route
            .into_iter()
            .map(|q| {
                (
                    q.0,
                    terrain_height(q.0, q.1) + roof::UNIT_CENTER_HEIGHT,
                    q.1,
                )
            })
            .collect::<Vec<_>>();
        let profile = roof.profile;
        let dx = engine.x - b.x;
        let dz = engine.z - b.z;
        let horizontal = dx.abs() > dz.abs();
        let edge = if horizontal {
            (
                b.x + dx.signum() * (profile.half_x - map::CLEARANCE - 0.05),
                engine
                    .z
                    .clamp(b.z - profile.half_z + 0.5, b.z + profile.half_z - 0.5),
            )
        } else {
            (
                engine
                    .x
                    .clamp(b.x - profile.half_x + 0.5, b.x + profile.half_x - 0.5),
                b.z + dz.signum() * (profile.half_z - map::CLEARANCE - 0.05),
            )
        };
        path.push((edge.0, roof.roof_y() + roof::UNIT_CENTER_HEIGHT, edge.1));
        let Some(tail) = roof.roof_path(edge, roof.slot(slot)) else {
            continue;
        };
        path.extend(tail);
        let modes = vec![4; path.len()];
        store_travel(ctx, &u, b.id, path, modes, false, false);
        ctx.db.siege_climber().unit_id().delete(u.id);
        ctx.db.siege_climber().insert(SiegeClimber {
            unit_id: u.id,
            game_id: u.game_id,
            access_id: engine.id,
            building_id: b.id,
            base_x: engine.x,
            base_z: engine.z,
            roof_x: edge.0,
            roof_z: edge.1,
        });
        ctx.db.tower_occupant().insert(TowerOccupant {
            unit_id: u.id,
            game_id: u.game_id,
            building_id: b.id,
            slot: slot as u8,
            door: 0,
            next_building: None,
            next_slot: 0,
        });
        ctx.db.unit_order().unit_id().delete(u.id);
        ctx.db.unit_engagement().unit_id().delete(u.id);
        u.target_x = roof.slot(slot).0;
        u.target_z = roof.slot(slot).1;
        ctx.db.army_unit().id().update(u);
        accepted += 1;
    }
    if accepted == 0 {
        return Err("Keine Fußtruppen oder kein freier Aufstieg zum Wehrgang.".into());
    }
    start_tick(ctx);
    Ok(())
}
#[derive(Clone)]
#[spacetimedb::table(name=siege_climber)]
pub struct SiegeClimber {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub access_id: u64,
    pub building_id: u64,
    pub base_x: f32,
    pub base_z: f32,
    pub roof_x: f32,
    pub roof_z: f32,
}
fn depart_assault(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    c: SiegeClimber,
) -> Result<map::Point, String> {
    if ctx
        .db
        .tower_travel()
        .unit_id()
        .find(u.id)
        .is_some_and(|t| t.leaving)
    {
        return Ok((c.base_x, c.base_z));
    }
    if ctx.db.siege_access().unit_id().find(c.access_id).is_none() {
        return Err(
            "Der Belagerungsaufstieg ist nicht mehr vorhanden. Nutze einen anderen Wehrgangzugang."
                .into(),
        );
    }
    let b = ctx
        .db
        .building()
        .id()
        .find(c.building_id)
        .ok_or("Der Wehrgang fehlt.")?;
    let roof = tower_geometry(ctx, &b);
    let mut points = if (u.y - roof.roof_y() - roof::UNIT_CENTER_HEIGHT).abs() < 0.15 {
        roof.roof_path((u.x, u.z), (c.roof_x, c.roof_z))
            .ok_or("Der Aufstieg ist nicht erreichbar.")?
    } else {
        Vec::new()
    };
    points.push((
        c.base_x,
        ground_height(ctx, u.game_id, c.base_x, c.base_z) + roof::UNIT_CENTER_HEIGHT,
        c.base_z,
    ));
    let modes = vec![4; points.len()];
    store_travel(ctx, u, b.id, points, modes, true, false);
    Ok((c.base_x, c.base_z))
}
#[spacetimedb::table(name=siege_climber_save)]
pub struct SiegeClimberSave {
    #[primary_key]
    pub id: u64,
    pub data: Vec<SiegeClimber>,
}
fn save_climbers(ctx: &ReducerContext, id: u64, game: u64) {
    let row = SiegeClimberSave {
        id,
        data: ctx.db.siege_climber().game_id().filter(game).collect(),
    };
    if ctx.db.siege_climber_save().id().find(id).is_some() {
        ctx.db.siege_climber_save().id().update(row);
    } else {
        ctx.db.siege_climber_save().insert(row);
    }
}
fn restore_climbers(
    ctx: &ReducerContext,
    id: u64,
    game: u64,
    bids: &std::collections::HashMap<u64, u64>,
    uids: &std::collections::HashMap<u64, u64>,
) {
    if let Some(s) = ctx.db.siege_climber_save().id().find(id) {
        for mut c in s.data {
            if let (Some(u), Some(a), Some(b)) = (
                uids.get(&c.unit_id),
                uids.get(&c.access_id),
                bids.get(&c.building_id),
            ) {
                c.unit_id = *u;
                c.access_id = *a;
                c.building_id = *b;
                c.game_id = game;
                ctx.db.siege_climber().insert(c);
            }
        }
    }
}
