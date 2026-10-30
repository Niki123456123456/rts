fn roof_pieces(
    ctx: &ReducerContext,
    game_id: u64,
    owner: spacetimedb::ConnectionId,
) -> Vec<map::walls::RoofPiece> {
    let mut pieces = ctx
        .db
        .building()
        .game_id()
        .filter(game_id)
        .filter(|b| b.owner == owner)
        .filter_map(|b| {
            Some(map::walls::RoofPiece {
                id: b.id,
                kind: map::BuildingKind::from_name(&b.kind)
                    .filter(|k| k.roof_profile().is_some())?,
                x: b.x,
                y: b.y,
                z: b.z,
                rotation: rotation(ctx, b.id),
            })
        })
        .collect::<Vec<_>>();
    pieces.sort_by_key(|p| p.id);
    pieces
}
fn is_wall(b: &Building) -> bool {
    map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_wall())
}
fn wall_level(b: &Building) -> f32 {
    b.y + map::BuildingKind::from_name(&b.kind)
        .and_then(|k| k.roof_profile())
        .map_or(map::walls::WALL_HEIGHT, |p| p.height)
        + roof::UNIT_CENTER_HEIGHT
}
fn wall_connected(ctx: &ReducerContext, b: &Building) -> bool {
    stair_for(ctx, b).is_some()
        || !map::walls::tower_routes(b.id, &roof_pieces(ctx, b.game_id, b.owner)).is_empty()
}
fn access_building(ctx: &ReducerContext, unit_id: u64, b: &Building) -> Option<Building> {
    let id = if is_wall(b) {
        ctx.db.wall_entry().unit_id().find(unit_id)?.tower_id
    } else {
        b.id
    };
    ctx.db.building().id().find(id)
}
// Ground approach to any accessible tower of the owner's connected wall network.
fn wall_entry_plan(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    b: &Building,
) -> Option<(u64, u8, Vec<map::Point>, Vec<Position>, Vec<u8>)> {
    let pieces = roof_pieces(ctx, b.game_id, b.owner);
    wall_entry_plan_using(ctx, u, b, &pieces)
}
fn wall_entry_plan_using(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    b: &Building,
    pieces: &[map::walls::RoofPiece],
) -> Option<(u64, u8, Vec<map::Point>, Vec<Position>, Vec<u8>)> {
    let obstacles = obstacles(ctx, b.game_id);
    if let Some(s) = stair_for(ctx, b) {
        let landing = stair_landing(&s);
        if let Some(path) = map::shortest_path((u.x, u.z), landing, &obstacles) {
            let mut points = vec![
                (s.base_x, s.base_y + roof::UNIT_CENTER_HEIGHT, s.base_z),
                (s.top_x, s.top_y + roof::UNIT_CENTER_HEIGHT, s.top_z),
            ];
            let route = map::walls::wall_path(s.wall_id, b.id, pieces)?;
            points.extend(route.into_iter().map(|p| (p.0, wall_level(b), p.1)));
            let modes = vec![4; points.len()];
            return Some((s.wall_id, 0, path, points, modes));
        }
    }
    let mut best = None;
    for (id, route) in map::walls::tower_routes(b.id, &pieces) {
        let access = pieces.iter().find(|p| p.id == id)?.roof();
        for door in 0..access.door_count() {
            let Some(path) = map::shortest_path((u.x, u.z), access.door(door), &obstacles) else {
                continue;
            };
            let cost =
                map::path_length((u.x, u.z), &path) + map::path_length(route[0], &route[1..]);
            if best
                .as_ref()
                .is_some_and(|(old, _): &(f32, _)| *old <= cost)
            {
                continue;
            }
            let hatch = access.hatch();
            let mut points = vec![(hatch.0, access.roof_y() + roof::UNIT_CENTER_HEIGHT, hatch.1)];
            let mut modes = vec![4];
            if (points[0].1 - wall_level(b)).abs() > 0.01 {
                points.push((hatch.0, wall_level(b), hatch.1));
                modes.push(5);
            }
            points.extend(
                route
                    .iter()
                    .copied()
                    .skip(1)
                    .map(|p| (p.0, wall_level(b), p.1)),
            );
            modes.resize(points.len(), 4);
            best = Some((cost, (id, door, path, points, modes)));
        }
    }
    best.map(|(_, plan)| plan)
}
fn depart_wall(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    b: &Building,
    mut o: TowerOccupant,
) -> Result<map::Point, String> {
    if let Some(s) = stair_for(ctx, b) {
        let pieces = roof_pieces(ctx, b.game_id, b.owner);
        let current = pieces
            .iter()
            .find(|p| p.kind.is_wall() && p.roof().roof_contains((u.x, u.z)));
        if current.is_none() && ctx.db.tower_travel().unit_id().find(u.id).is_some() {
            let mut points = Vec::new();
            if u.y >= s.top_y + roof::UNIT_CENTER_HEIGHT - 0.1 {
                points.push((s.top_x, s.top_y + roof::UNIT_CENTER_HEIGHT, s.top_z));
            }
            points.push((s.base_x, s.base_y + roof::UNIT_CENTER_HEIGHT, s.base_z));
            let landing = stair_landing(&s);
            points.push((
                landing.0,
                terrain_height(landing.0, landing.1) + roof::UNIT_CENTER_HEIGHT,
                landing.1,
            ));
            let modes = vec![4; points.len()];
            store_travel(ctx, u, b.id, points, modes, true, false);
            return Ok(landing);
        }
        let current = current.ok_or("Einheit steht nicht auf der Mauer.")?;
        let route = map::walls::wall_path(current.id, s.wall_id, &pieces)
            .ok_or("Keine Verbindung zur Treppe.")?;
        let mut points = route
            .into_iter()
            .map(|p| (p.0, wall_level(b), p.1))
            .collect::<Vec<_>>();
        points.extend([
            (s.top_x, s.top_y + roof::UNIT_CENTER_HEIGHT, s.top_z),
            (s.base_x, s.base_y + roof::UNIT_CENTER_HEIGHT, s.base_z),
        ]);
        let landing = stair_landing(&s);
        points.push((
            landing.0,
            terrain_height(landing.0, landing.1) + roof::UNIT_CENTER_HEIGHT,
            landing.1,
        ));
        let modes = vec![4; points.len()];
        store_travel(ctx, u, b.id, points, modes, true, false);
        return Ok(landing);
    }

    let pieces = combat_roof_pieces(ctx, b.game_id);
    let current = pieces
        .iter()
        .find(|p| p.kind.is_wall() && p.roof().roof_contains((u.x, u.z)));
    let mut routes = if let Some(current) = current {
        map::walls::tower_routes(current.id, &pieces)
    } else {
        let access = access_building(ctx, u.id, b).ok_or("No tower exit.")?;
        vec![(access.id, vec![tower_geometry(ctx, &access).hatch()])]
    };
    routes.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then(a.0.cmp(&b.0)));
    for (id, mut route) in routes {
        let access = ctx.db.building().id().find(id).unwrap();
        let tower = tower_geometry(ctx, &access);
        let Some(door) = (0..tower.door_count()).find(|door| door_clear(ctx, &access, *door))
        else {
            continue;
        };
        route.reverse();
        let mut points = route
            .into_iter()
            .map(|p| (p.0, wall_level(b), p.1))
            .collect::<Vec<_>>();
        let mut modes = vec![4; points.len()];
        let hatch = tower.hatch();
        if (wall_level(b) - tower.roof_y() - roof::UNIT_CENTER_HEIGHT).abs() > 0.01 {
            let roof_y = tower.roof_y() + roof::UNIT_CENTER_HEIGHT;
            let last = points.len() - 1;
            if last > 0 {
                let portal = points[last - 1];
                points[last].1 = roof_y;
                points.insert(last, (portal.0, roof_y, portal.2));
                modes.insert(last, 5);
            } else {
                points.push((hatch.0, roof_y, hatch.1));
                modes.push(5);
            }
        }
        let landing = tower.door(door);
        points.push((landing.0, access.y + roof::UNIT_CENTER_HEIGHT, landing.1));
        modes.push(4);
        o.door = door;
        ctx.db.tower_occupant().unit_id().update(o);
        ctx.db.wall_entry().unit_id().delete(u.id);
        ctx.db.wall_entry().insert(WallEntry {
            unit_id: u.id,
            tower_id: id,
        });
        store_travel(ctx, u, b.id, points, modes, true, false);
        return Ok(landing);
    }
    Err("This wall has no reachable tower exit. Connect it to a tower with a clear door.".into())
}
fn queue_wall_walk(
    ctx: &ReducerContext,
    u: &mut ArmyUnit,
    b: &Building,
    slot: usize,
    mut o: TowerOccupant,
    start: u64,
) -> Result<(), String> {
    let pieces = roof_pieces(ctx, b.game_id, b.owner);
    let route =
        map::walls::wall_path(start, b.id, &pieces).ok_or("These walls are not connected.")?;
    let mut points = route
        .into_iter()
        .map(|p| (p.0, wall_level(b), p.1))
        .collect::<Vec<_>>();
    let goal = tower_geometry(ctx, b).slot(slot);
    points.push((goal.0, wall_level(b), goal.1));
    let modes = vec![4; points.len()];
    ctx.db.unit_order().unit_id().delete(u.id);
    o.building_id = b.id;
    o.slot = slot as u8;
    o.next_building = None;
    ctx.db.tower_occupant().unit_id().update(o);
    u.target_x = goal.0;
    u.target_z = goal.1;
    store_travel(ctx, u, b.id, points, modes, false, false);
    ctx.db.army_unit().id().update(u.clone());
    Ok(())
}
// Demolition cancels routes crossing the removed piece; no travel crosses a gap.
fn cancel_surface_routes_through(ctx: &ReducerContext, id: u64) {
    let Some(b) = ctx.db.building().id().find(id) else {
        return;
    };
    if !roof::supports_roof(&b.kind) {
        return;
    }
    let shape = map::BuildingKind::from_name(&b.kind)
        .unwrap()
        .rotated_build_footprint(b.x, b.z, rotation(ctx, id));
    let affected = ctx
        .db
        .tower_travel()
        .iter()
        .filter(|t| {
            t.game_id == b.game_id
                && (ctx
                    .db
                    .wall_entry()
                    .unit_id()
                    .find(t.unit_id)
                    .is_some_and(|a| a.tower_id == id)
                    || (t.modes.contains(&4)
                        && ctx
                            .db
                            .army_unit()
                            .id()
                            .find(t.unit_id)
                            .is_some_and(|u| shape.contains((u.x, u.z), 0.001)))
                    || (t.modes.contains(&4)
                        && t.path
                            .chunks_exact(3)
                            .skip(t.next as usize)
                            .any(|p| shape.contains((p[0], p[2]), 0.001))))
        })
        .map(|t| t.unit_id)
        .collect::<Vec<_>>();
    for unit_id in affected {
        if let Some(mut u) = ctx.db.army_unit().id().find(unit_id) {
            clear_tower(ctx, unit_id);
            let obs = obstacles(ctx, u.game_id);
            let peers = ctx
                .db
                .army_unit()
                .game_id()
                .filter(u.game_id)
                .filter(|p| p.id != u.id && on_ground(ctx, p))
                .map(|p| (p.x, p.z))
                .collect::<Vec<_>>();
            if let Some(p) = free_destination((u.x, u.z), &peers, &obs) {
                u.x = p.0;
                u.z = p.1;
            }
            u.y = terrain_height(u.x, u.z) + roof::UNIT_CENTER_HEIGHT;
            stop_unit(ctx, u);
        }
    }
}

/// Fill nearest free slots across the connected wall, bounded by its total area.
fn move_to_wall(
    ctx: &ReducerContext,
    mut units: Vec<ArmyUnit>,
    b: &Building,
    point: map::Point,
) -> Result<(), String> {
    if !wall_connected(ctx, b) {
        return Err("Connect this wall to one of your walkable towers first.".into());
    }
    let pieces = roof_pieces(ctx, b.game_id, b.owner);
    let component = map::walls::wall_component(b.id, &pieces);
    let mut slots = component
        .iter()
        .flat_map(|p| (0..p.roof().capacity()).map(move |slot| (*p, slot)))
        .collect::<Vec<_>>();
    slots.sort_by(|a, b| {
        map::distance(a.0.roof().slot(a.1), point)
            .total_cmp(&map::distance(b.0.roof().slot(b.1), point))
            .then(a.0.id.cmp(&b.0.id))
            .then(a.1.cmp(&b.1))
    });
    units.sort_by_key(|u| (ctx.db.tower_occupant().unit_id().find(u.id).is_none(), u.id));
    let mut accepted = 0;
    for mut u in units {
        let Some((piece, slot)) = slots
            .iter()
            .find(|(p, s)| !slot_reserved(ctx, p.id, *s as u8, u.id))
        else {
            continue;
        };
        let target = ctx.db.building().id().find(piece.id).unwrap();
        if let Some(o) = ctx.db.tower_occupant().unit_id().find(u.id) {
            if on_ground(ctx, &u) {
                clear_tower(ctx, u.id);
                ctx.db.unit_order().unit_id().delete(u.id);
                start_roof_entry(ctx, u, &target, *slot as u8)?;
            } else {
                let current = pieces.iter().find(|p| {
                    p.kind.is_wall()
                        && p.roof().roof_contains((u.x, u.z))
                        && (u.y - wall_level(b)).abs() < 0.01
                });
                if let Some(current) =
                    current.filter(|p| map::walls::wall_path(p.id, piece.id, &pieces).is_some())
                {
                    queue_wall_walk(ctx, &mut u, &target, *slot, o, current.id)?;
                } else {
                    depart_tower(ctx, &u)?;
                    ctx.db.unit_order().unit_id().delete(u.id);
                    if let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) {
                        o.next_building = Some(piece.id);
                        o.next_slot = *slot as u8;
                        ctx.db.tower_occupant().unit_id().update(o);
                    } else {
                        start_roof_entry(ctx, u, &target, *slot as u8)?;
                    }
                }
            }
        } else {
            start_roof_entry(ctx, u, &target, *slot as u8)?;
        }
        accepted += 1;
    }
    if accepted == 0 {
        return Err(format!(
            "The connected wall is full ({} units).",
            component.len() * 9
        ));
    }
    start_tick(ctx);
    Ok(())
}
