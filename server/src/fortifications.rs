use map::fortifications as fort;
#[derive(Clone)]
#[spacetimedb::table(name=gate_state,public)]
pub struct GateState {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub open: bool,
}
#[derive(Clone)]
#[spacetimedb::table(name=moat,public)]
pub struct Moat {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub owner: spacetimedb::ConnectionId,
    pub x: f32,
    pub z: f32,
}
#[derive(Clone)]
#[spacetimedb::table(name=wall_stair,public)]
pub struct WallStair {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub owner: spacetimedb::ConnectionId,
    pub wall_id: u64,
    pub base_x: f32,
    pub base_z: f32,
    pub top_x: f32,
    pub top_z: f32,
    pub base_y: f32,
    pub top_y: f32,
}
fn gate_is_open(ctx: &ReducerContext, id: u64) -> bool {
    ctx.db
        .gate_state()
        .building_id()
        .find(id)
        .is_some_and(|g| g.open)
}
fn moat_shape(m: &Moat) -> Obstacle {
    Obstacle {
        x: m.x,
        z: m.z,
        half_x: 2.,
        half_z: 2.,
    }
}
fn stair_shape(s: &WallStair) -> Obstacle {
    fort::stair_bounds((s.base_x, s.base_z), (s.top_x, s.top_z))
}
fn fort_obstacles(ctx: &ReducerContext, game: u64) -> Vec<Obstacle> {
    let mut solids = Vec::new();
    let mut decks = Vec::new();
    for b in ctx.db.building().game_id().filter(game) {
        let k = map::BuildingKind::from_name(&b.kind).unwrap_or_default();
        let r = rotation(ctx, b.id);
        let open = gate_is_open(ctx, b.id);
        if k.is_gate() {
            solids.extend(fort::gate_solids(k, (b.x, b.z), r, open));
            if k == map::BuildingKind::DrawbridgeGate && open {
                decks.push(fort::bridge_span((b.x, b.z), r));
            }
        } else {
            solids.extend(k.ground_solids(b.x, b.z, r));
            if k == map::BuildingKind::RomanBridge {
                decks.push(k.rotated_obstacle(b.x, b.z, r));
            }
        }
    }
    solids.extend(
        fort::joins(&fort_pieces(ctx, game))
            .into_iter()
            .map(|j| j.shape),
    );
    let mut water = map::terrain::obstacles();
    water.extend(ctx.db.moat().game_id().filter(game).map(|m| moat_shape(&m)));
    for deck in decks {
        water = water
            .into_iter()
            .flat_map(|s| fort::subtract(s, deck))
            .collect();
    }
    solids.extend(water);
    solids.extend(
        ctx.db
            .wall_stair()
            .game_id()
            .filter(game)
            .map(|s| stair_shape(&s)),
    );
    solids
}
#[spacetimedb::reducer]
pub fn set_gate(ctx: &ReducerContext, building_id: u64, open: bool) -> Result<(), String> {
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .filter(|b| b.owner == p.connection_id && b.game_id == p.game_id)
        .ok_or("Wähle ein eigenes Tor.")?;
    let k = map::BuildingKind::from_name(&b.kind)
        .filter(|k| k.is_gate())
        .ok_or("Dieses Gebäude ist kein Tor.")?;
    if !open {
        let closing = if k == map::BuildingKind::DrawbridgeGate {
            fort::bridge_span((b.x, b.z), rotation(ctx, b.id))
        } else {
            fort::rect((b.x, b.z), rotation(ctx, b.id), (0., 0.), 1.9, 1.)
        };
        if ctx
            .db
            .army_unit()
            .game_id()
            .filter(p.game_id)
            .any(|u| on_ground(ctx, &u) && closing.contains((u.x, u.z), map::CLEARANCE))
        {
            return Err("Der Durchgang ist noch belegt. Bewege die Einheiten zuerst weg.".into());
        }
    }
    let row = GateState {
        building_id,
        game_id: p.game_id,
        open,
    };
    if ctx
        .db
        .gate_state()
        .building_id()
        .find(building_id)
        .is_some()
    {
        ctx.db.gate_state().building_id().update(row);
    } else {
        ctx.db.gate_state().insert(row);
    }
    replan(ctx, p.game_id);
    Ok(())
}
#[spacetimedb::reducer]
pub fn dig_moat(
    ctx: &ReducerContext,
    start_x: f32,
    start_z: f32,
    end_x: f32,
    end_z: f32,
) -> Result<(), String> {
    let p = participant(ctx)?;
    let points = fort::moat_line((start_x, start_z), (end_x, end_z));
    if points.is_empty() || points.len() > 128 {
        return Err("Ziehe einen Graben mit höchstens 128 Abschnitten.".into());
    }
    if ctx.db.moat().game_id().filter(p.game_id).count() + points.len() > 2048 {
        return Err("Zu viele Grabenabschnitte.".into());
    }
    let mut fresh = Vec::new();
    for (x, z) in points {
        if ctx
            .db
            .moat()
            .game_id()
            .filter(p.game_id)
            .any(|m| m.x == x && m.z == z)
        {
            continue;
        }
        let area = Obstacle {
            x,
            z,
            half_x: 2.,
            half_z: 2.,
        };
        if !area.in_build_area()
            || !map::terrain::buildable(map::BuildingKind::Housing, x, z)
            || ctx.db.building().game_id().filter(p.game_id).any(|b| {
                map::BuildingKind::from_name(&b.kind)
                    .unwrap_or_default()
                    .construction_bounds(b.x, b.z, rotation(ctx, b.id))
                    .overlaps(area)
            })
            || ctx
                .db
                .army_unit()
                .game_id()
                .filter(p.game_id)
                .any(|u| on_ground(ctx, &u) && area.contains((u.x, u.z), map::CLEARANCE))
            || ctx
                .db
                .wall_stair()
                .game_id()
                .filter(p.game_id)
                .any(|s| stair_shape(&s).overlaps(area))
        {
            return Err(
                "Der Graben braucht freies Land ohne Gebäude, Treppen oder Einheiten.".into(),
            );
        }
        fresh.push((x, z));
    }
    debit_cost(
        ctx,
        &p,
        map::Cost::new((fresh.len() as u32).div_ceil(4), 0, 0),
    )?;
    for (x, z) in fresh {
        let m = ctx.db.moat().insert(Moat {
            id: 0,
            game_id: p.game_id,
            owner: p.connection_id,
            x,
            z,
        });
        clear_resource_trees(ctx, p.game_id, moat_shape(&m));
    }
    replan(ctx, p.game_id);
    Ok(())
}
#[spacetimedb::reducer]
pub fn fill_moat(ctx: &ReducerContext, moat_id: u64) -> Result<(), String> {
    let p = participant(ctx)?;
    let m = ctx
        .db
        .moat()
        .id()
        .find(moat_id)
        .filter(|m| m.game_id == p.game_id && m.owner == p.connection_id)
        .ok_or("Kein eigener Graben.")?;
    ctx.db.moat().id().delete(m.id);
    replan(ctx, p.game_id);
    Ok(())
}
#[spacetimedb::reducer]
pub fn build_wall_stair(
    ctx: &ReducerContext,
    wall_id: u64,
    base_x: f32,
    base_z: f32,
    top_x: f32,
    top_z: f32,
) -> Result<(), String> {
    let p = participant(ctx)?;
    if ![base_x, base_z, top_x, top_z]
        .iter()
        .all(|v| v.is_finite() && v.abs() < map::MAP_LIMIT)
    {
        return Err("Ungültige Treppenposition.".into());
    }
    let wall = ctx
        .db
        .building()
        .id()
        .find(wall_id)
        .filter(|b| b.owner == p.connection_id && b.game_id == p.game_id && is_wall(b))
        .ok_or("Das obere Ende muss auf einer eigenen Mauer liegen.")?;
    if ctx
        .db
        .wall_stair()
        .game_id()
        .filter(p.game_id)
        .any(|s| s.wall_id == wall.id)
    {
        return Err("Diese Mauer hat bereits eine Treppe.".into());
    }
    let roof = tower_geometry(ctx, &wall);
    let target = roof.slot(roof.sorted_slots((top_x, top_z))[0]);
    let base = (base_x, base_z);
    let top = fort::stair_top(base, target, roof.profile.height);
    let length = map::distance(base, top);
    if length < roof.profile.height * 1.2 || length > roof.profile.height * 4. + 4. {
        return Err(
            "Die Treppe braucht einen passenden Lauf: etwa 1,2 bis 4 Meter je Höhenmeter.".into(),
        );
    }
    let obs = obstacles(ctx, p.game_id);
    if !map::free(base, &obs) {
        return Err("Der Treppenfuß muss auf freiem Land liegen.".into());
    }
    // A narrow swept corridor may touch its target wall, but never cross another building or water.
    for i in 0..=40 {
        let t = i as f32 / 40.;
        let q = (base.0 + (top.0 - base.0) * t, base.1 + (top.1 - base.1) * t);
        let shape = building_shape(ctx, &wall);
        let other = obs
            .iter()
            .filter(|s| (s.x - shape.x).abs() > 0.01 || (s.z - shape.z).abs() > 0.01)
            .copied()
            .collect::<Vec<_>>();
        if !map::free(q, &other) {
            return Err("Die Treppe schneidet ein Gebäude oder unpassierbares Gelände.".into());
        }
    }
    let s = WallStair {
        id: 0,
        game_id: p.game_id,
        owner: p.connection_id,
        wall_id,
        base_x,
        base_z,
        top_x: top.0,
        top_z: top.1,
        base_y: terrain_height(base_x, base_z),
        top_y: roof.roof_y(),
    };
    let area = stair_shape(&s);
    if !area.in_build_area()
        || s.top_y <= s.base_y
        || ctx
            .db
            .building()
            .game_id()
            .filter(p.game_id)
            .filter(|b| b.id != wall_id)
            .any(|b| {
                map::BuildingKind::from_name(&b.kind)
                    .unwrap_or_default()
                    .construction_bounds(b.x, b.z, rotation(ctx, b.id))
                    .overlaps(area)
            })
        || ctx
            .db
            .moat()
            .game_id()
            .filter(p.game_id)
            .any(|m| moat_shape(&m).overlaps(area))
    {
        return Err("Der gesamte Treppenlauf braucht einen freien Bauplatz.".into());
    }
    if ctx
        .db
        .army_unit()
        .game_id()
        .filter(p.game_id)
        .any(|u| on_ground(ctx, &u) && stair_shape(&s).contains((u.x, u.z), map::CLEARANCE))
    {
        return Err("Einheiten stehen im Treppenlauf.".into());
    }
    if ctx
        .db
        .wall_stair()
        .game_id()
        .filter(p.game_id)
        .any(|a| stair_shape(&a).overlaps(stair_shape(&s)))
    {
        return Err("Treppen dürfen sich nicht überbauen.".into());
    }
    debit_cost(ctx, &p, map::Cost::new(2, (length / 2.).ceil() as u32, 0))?;
    ctx.db.wall_stair().insert(s);
    replan(ctx, p.game_id);
    Ok(())
}
fn stair_for(ctx: &ReducerContext, wall: &Building) -> Option<WallStair> {
    let ids = map::walls::wall_component(wall.id, &roof_pieces(ctx, wall.game_id, wall.owner))
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();
    ctx.db
        .wall_stair()
        .game_id()
        .filter(wall.game_id)
        .filter(|s| ids.contains(&s.wall_id))
        .min_by_key(|s| s.id)
}
fn stair_landing(s: &WallStair) -> map::Point {
    let d = map::distance((s.base_x, s.base_z), (s.top_x, s.top_z)).max(0.01);
    (
        s.base_x + (s.base_x - s.top_x) * 1.7 / d,
        s.base_z + (s.base_z - s.top_z) * 1.7 / d,
    )
}
fn access_door(ctx: &ReducerContext, u: u64, b: &Building, door: u8) -> Option<map::Point> {
    let access = access_building(ctx, u, b)?;
    if is_wall(&access) {
        let s = stair_for(ctx, &access)?;
        Some(stair_landing(&s))
    } else {
        Some(tower_geometry(ctx, &access).door(door))
    }
}

fn ground_height(ctx: &ReducerContext, game: u64, x: f32, z: f32) -> f32 {
    if let Some(b) = ctx
        .db
        .building()
        .game_id()
        .filter(game)
        .filter(|b| map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_road()))
        .find(|b| building_shape(ctx, b).contains((x, z), 0.))
    {
        return b.y
            + match b.kind.as_str() {
                "roman_bridge" => 0.18,
                "stone_road" => 0.08,
                _ => 0.05,
            };
    }
    ctx.db
        .building()
        .game_id()
        .filter(game)
        .filter(|b| b.kind == "drawbridge_gate" && gate_is_open(ctx, b.id))
        .find(|b| fort::bridge_deck((b.x, b.z), rotation(ctx, b.id)).contains((x, z), 0.))
        .map_or_else(|| terrain_height(x, z), |b| b.y + 0.29)
}
fn remove_fort_building(ctx: &ReducerContext, id: u64) {
    ctx.db.gate_state().building_id().delete(id);
    for s in ctx
        .db
        .wall_stair()
        .iter()
        .filter(|s| s.wall_id == id)
        .collect::<Vec<_>>()
    {
        ctx.db.wall_stair().id().delete(s.id);
    }
}

fn fort_piece(ctx: &ReducerContext, b: &Building) -> fort::Structure {
    fort::Structure {
        id: b.id,
        kind: map::BuildingKind::from_name(&b.kind).unwrap_or_default(),
        x: b.x,
        y: b.y,
        z: b.z,
        rotation: rotation(ctx, b.id),
        owner: b.color,
    }
}
fn fort_pieces(ctx: &ReducerContext, game: u64) -> Vec<fort::Structure> {
    ctx.db
        .building()
        .game_id()
        .filter(game)
        .filter(|b| !map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_road()))
        .map(|b| fort_piece(ctx, &b))
        .collect()
}

fn reconcile_fort_heights(ctx: &ReducerContext, game: u64) {
    let pieces = fort_pieces(ctx, game);
    for p in pieces.iter().filter(|p| p.kind.is_crenellated()) {
        let next = fort::crenellation_kind(*p, &pieces);
        if next == p.kind {
            continue;
        }
        let Some(mut b) = ctx.db.building().id().find(p.id) else {
            continue;
        };
        let old = p.level() + roof::UNIT_CENTER_HEIGHT;
        let height = b.y + next.roof_profile().unwrap().height + roof::UNIT_CENTER_HEIGHT;
        b.kind = next.name().into();
        ctx.db.building().id().update(b);
        for o in ctx
            .db
            .tower_occupant()
            .iter()
            .filter(|o| o.building_id == p.id || o.next_building == Some(p.id))
            .collect::<Vec<_>>()
        {
            if let Some(mut u) = ctx.db.army_unit().id().find(o.unit_id) {
                if (u.y - old).abs() < 0.15 {
                    u.y = height;
                    ctx.db.army_unit().id().update(u);
                }
            }
            if let Some(mut t) = ctx.db.tower_travel().unit_id().find(o.unit_id) {
                for point in t.path.chunks_exact_mut(3) {
                    if (point[1] - old).abs() < 0.15 {
                        point[1] = height;
                    }
                }
                ctx.db.tower_travel().unit_id().update(t);
            }
        }
        for mut stair in ctx
            .db
            .wall_stair()
            .game_id()
            .filter(game)
            .filter(|s| s.wall_id == p.id)
            .collect::<Vec<_>>()
        {
            stair.top_y = height - roof::UNIT_CENTER_HEIGHT;
            ctx.db.wall_stair().id().update(stair);
        }
    }
}

// Existing worlds retain their crenellated wall kind; migrate the old floor once.
#[spacetimedb::table(name=fort_height_revision)]
pub struct FortHeightRevision {
    #[primary_key]
    pub game_id: u64,
}
fn migrate_fort_heights(ctx: &ReducerContext, game: u64) {
    if ctx.db.fort_height_revision().game_id().find(game).is_some() {
        return;
    }
    for b in ctx
        .db
        .building()
        .game_id()
        .filter(game)
        .filter(|b| b.kind == "stone_wall_crenellated")
        .collect::<Vec<_>>()
    {
        let old = b.y + 6.663 + roof::UNIT_CENTER_HEIGHT;
        let new = b.y + 10. + roof::UNIT_CENTER_HEIGHT;
        for o in ctx
            .db
            .tower_occupant()
            .iter()
            .filter(|o| o.building_id == b.id || o.next_building == Some(b.id))
            .collect::<Vec<_>>()
        {
            if let Some(mut u) = ctx.db.army_unit().id().find(o.unit_id) {
                if (u.y - old).abs() < 0.15 {
                    u.y = new;
                    ctx.db.army_unit().id().update(u);
                }
            }
            if let Some(mut t) = ctx.db.tower_travel().unit_id().find(o.unit_id) {
                for p in t.path.chunks_exact_mut(3) {
                    if (p[1] - old).abs() < 0.15 {
                        p[1] = new;
                    }
                }
                ctx.db.tower_travel().unit_id().update(t);
            }
        }
        for mut stair in ctx
            .db
            .wall_stair()
            .game_id()
            .filter(game)
            .filter(|s| s.wall_id == b.id)
            .collect::<Vec<_>>()
        {
            stair.top_y = b.y + 10.;
            ctx.db.wall_stair().id().update(stair);
        }
    }
    reconcile_fort_heights(ctx, game);
    ctx.db
        .fort_height_revision()
        .insert(FortHeightRevision { game_id: game });
}
