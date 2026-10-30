use map::building_roof::{self as roof, BuildingRoof, Position};

fn tower_geometry(ctx: &ReducerContext, b: &Building) -> BuildingRoof {
    BuildingRoof::new(
        map::BuildingKind::from_name(&b.kind).unwrap(),
        b.x,
        b.y,
        b.z,
        rotation(ctx, b.id),
    )
    .expect("Roof reservations only reference buildings with a roof profile")
}
fn clear_tower(ctx: &ReducerContext, id: u64) {
    ctx.db.wall_entry().unit_id().delete(id);
    ctx.db.tower_travel().unit_id().delete(id);
    ctx.db.tower_occupant().unit_id().delete(id);
}
fn on_ground(ctx: &ReducerContext, u: &ArmyUnit) -> bool {
    if ctx.db.tower_occupant().unit_id().find(u.id).is_none() {
        return true;
    }
    ctx.db.tower_travel().unit_id().find(u.id).is_some_and(|t| {
        !t.leaving
            && (ctx.db.unit_order().unit_id().find(u.id).is_some()
                || t.modes.get(t.next as usize) == Some(&2))
    })
}
fn ground_reservations(ctx: &ReducerContext, u: &ArmyUnit) -> Vec<map::Point> {
    let mut points = Vec::new();
    if on_ground(ctx, u) {
        points.push((u.x, u.z));
    }
    if ctx.db.unit_order().unit_id().find(u.id).is_some() {
        points.push((u.target_x, u.target_z));
    }
    points
}
fn ground_start(ctx: &ReducerContext, u: &ArmyUnit) -> map::Point {
    ctx.db
        .tower_travel()
        .unit_id()
        .find(u.id)
        .filter(|t| t.leaving)
        .and_then(|t| t.path.chunks_exact(3).last().map(|p| (p[0], p[2])))
        .unwrap_or((u.x, u.z))
}
fn travel_points(t: &TowerTravel) -> Vec<Position> {
    t.path.chunks_exact(3).map(|p| (p[0], p[1], p[2])).collect()
}
fn store_travel(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    building_id: u64,
    points: Vec<Position>,
    modes: Vec<u8>,
    leaving: bool,
    active: bool,
) {
    ctx.db.tower_travel().unit_id().delete(u.id);
    ctx.db.tower_travel().insert(TowerTravel {
        unit_id: u.id,
        game_id: u.game_id,
        building_id,
        path: points.into_iter().flat_map(|p| [p.0, p.1, p.2]).collect(),
        modes,
        next: 0,
        leaving,
        active,
    });
}
fn slot_reserved(ctx: &ReducerContext, building_id: u64, slot: u8, except: u64) -> bool {
    ctx.db.tower_occupant().iter().any(|o| {
        o.unit_id != except
            && ((o.building_id == building_id && o.slot == slot)
                || (o.next_building == Some(building_id) && o.next_slot == slot))
    })
}
fn door_clear(ctx: &ReducerContext, b: &Building, door: u8) -> bool {
    map::free(
        tower_geometry(ctx, b).door(door),
        &obstacles(ctx, b.game_id),
    )
}
fn entry_plan(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    b: &Building,
    _slot: u8,
) -> Option<(u64, u8, Vec<map::Point>, Vec<Position>, Vec<u8>)> {
    if is_wall(b) {
        return wall_entry_plan(ctx, u, b);
    }
    let tower = tower_geometry(ctx, b);
    let obstacles = obstacles(ctx, u.game_id);
    (0..tower.door_count())
        .filter_map(|door| {
            let path = map::shortest_path((u.x, u.z), tower.door(door), &obstacles)?;
            let hatch = tower.hatch_for_door(door);
            let cost = map::path_length((u.x, u.z), &path);
            let (points, modes) = if b.kind == "keep" || b.kind == "drawbridge_gate" {
                let top = tower.world(if b.kind == "keep" {
                    (if door == 1 { -6.5 } else { 6.5 }, 3.0)
                } else {
                    (6.4, 2.5)
                });
                (
                    vec![
                        (top.0, tower.roof_y() + roof::UNIT_CENTER_HEIGHT, top.1),
                        (hatch.0, tower.roof_y() + roof::UNIT_CENTER_HEIGHT, hatch.1),
                    ],
                    vec![4, 4],
                )
            } else {
                (
                    vec![(hatch.0, tower.roof_y() + roof::UNIT_CENTER_HEIGHT, hatch.1)],
                    vec![4],
                )
            };
            Some((cost, door, path, points, modes))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, door, path, points, modes)| (b.id, door, path, points, modes))
}
fn start_roof_entry(
    ctx: &ReducerContext,
    u: ArmyUnit,
    b: &Building,
    slot: u8,
) -> Result<(), String> {
    let plan = entry_plan(ctx, &u, b, slot).ok_or("No tower door is reachable.")?;
    start_roof_entry_using(ctx, u, b, slot, plan)
}
fn start_roof_entry_using(
    ctx: &ReducerContext,
    mut u: ArmyUnit,
    b: &Building,
    slot: u8,
    plan: (u64, u8, Vec<map::Point>, Vec<Position>, Vec<u8>),
) -> Result<(), String> {
    let (access, door, approach, mut points, mut modes) = plan;
    let tower = tower_geometry(ctx, b);
    if is_wall(b) {
        let goal = tower.slot(slot as usize);
        points.push((goal.0, wall_level(b), goal.1));
        modes.push(4);
        ctx.db.wall_entry().unit_id().delete(u.id);
        ctx.db.wall_entry().insert(WallEntry {
            unit_id: u.id,
            tower_id: access,
        });
    } else {
        let roof = tower
            .roof_path(tower.hatch_for_door(door), tower.slot(slot as usize))
            .ok_or("Roof destination is unreachable.")?;
        points.extend(roof);
        modes.resize(points.len(), 0);
    }
    let goal = *approach.last().ok_or("No tower approach.")?;
    u.target_x = goal.0;
    u.target_z = goal.1;
    ctx.db.tower_occupant().insert(TowerOccupant {
        unit_id: u.id,
        game_id: u.game_id,
        building_id: b.id,
        slot,
        door,
        next_building: None,
        next_slot: 0,
    });
    store_travel(ctx, &u, b.id, points, modes, false, false);
    save_order(ctx, &u, None, approach);
    ctx.db.army_unit().id().update(u);
    Ok(())
}

/// A right click on an owned walkable roof reserves only the available slots.
#[spacetimedb::reducer]
pub fn move_to_roof(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    building_id: u64,
    x: f32,
    z: f32,
) -> Result<(), String> {
    let player = participant(ctx)?;
    let units = owned_units(ctx, &player, unit_ids)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .ok_or("Unknown tower.")?;
    if b.game_id != player.game_id
        || b.owner != player.connection_id
        || !roof::supports_roof(&b.kind)
    {
        return Err("Choose the roof of your own walkable tower.".into());
    }
    let tower = tower_geometry(ctx, &b);
    if units
        .iter()
        .any(|u| combat_kind(ctx, u) == map::units::UnitKind::Onager)
    {
        return Err("Onager werden auf festem Boden aufgestellt.".into());
    }
    if units.iter().any(|u| combat_kind(ctx, u).is_siege())
        && tower.profile.half_x.min(tower.profile.half_z) < 1.3
    {
        return Err("Für die Balliste brauchst du eine breitere Plattform.".into());
    }
    if !x.is_finite() || !z.is_finite() || !tower.roof_contains((x, z)) {
        return Err("Choose a point on the roof.".into());
    }
    let mut remaining = Vec::new();
    let mut transferred = 0;
    let structures = fort_pieces(ctx, b.game_id)
        .into_iter()
        .filter(|p| p.owner == b.color)
        .collect::<Vec<_>>();
    for mut u in units {
        let plan = ctx
            .db
            .tower_occupant()
            .unit_id()
            .find(u.id)
            .filter(|o| {
                o.building_id != b.id && ctx.db.tower_travel().unit_id().find(u.id).is_none()
            })
            .and_then(|o| fort::roof_transfer(o.building_id, b.id, &structures).map(|r| (o, r)));
        if let Some((mut o, route)) = plan {
            let Some(slot) = tower
                .sorted_slots((x, z))
                .into_iter()
                .find(|slot| !slot_reserved(ctx, b.id, *slot as u8, u.id))
            else {
                remaining.push(u);
                continue;
            };
            let old = ctx.db.building().id().find(o.building_id).unwrap();
            let old_roof = tower_geometry(ctx, &old);
            let Some(mut points) = old_roof.roof_path((u.x, u.z), (route[0].x, route[0].z)) else {
                remaining.push(u);
                continue;
            };
            let mut modes = vec![4; points.len()];
            for step in route.into_iter().skip(1) {
                points.push((step.x, step.y, step.z));
                modes.push(step.mode);
            }
            let tail = tower
                .roof_path(tower.hatch(), tower.slot(slot))
                .ok_or("Der Dachzugang ist blockiert.")?;
            modes.extend(vec![0; tail.len()]);
            points.extend(tail);
            store_travel(ctx, &u, b.id, points, modes, false, false);
            o.next_building = Some(b.id);
            o.next_slot = slot as u8;
            o.door = 0;
            ctx.db.tower_occupant().unit_id().update(o);
            ctx.db.unit_order().unit_id().delete(u.id);
            u.target_x = tower.slot(slot).0;
            u.target_z = tower.slot(slot).1;
            ctx.db.army_unit().id().update(u);
            transferred += 1;
        } else {
            remaining.push(u);
        }
    }
    let mut units = remaining;
    if units.is_empty() && transferred > 0 {
        start_tick(ctx);
        return Ok(());
    }
    if is_wall(&b) {
        return move_to_wall(ctx, units, &b, (x, z));
    }
    normalize_roof_slots(ctx);
    // Existing occupants retain their reservations; extra ground units keep their orders.
    units.sort_by_key(|u| (ctx.db.tower_occupant().unit_id().find(u.id).is_none(), u.id));
    let mut accepted = 0;
    for mut u in units {
        let Some(slot) = tower
            .sorted_slots((x, z))
            .into_iter()
            .find(|slot| !slot_reserved(ctx, b.id, *slot as u8, u.id))
        else {
            continue;
        };
        if let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) {
            let travel = ctx.db.tower_travel().unit_id().find(u.id);
            if o.building_id == b.id && !travel.as_ref().is_some_and(|t| t.leaving) {
                let (mut points, mut modes, active) = if let Some(t) = travel {
                    let points = travel_points(&t);
                    let remaining = &t.modes[t.next as usize..];
                    if remaining.iter().any(|m| *m != 0) {
                        let end = t.modes.iter().rposition(|m| *m != 0).unwrap() + 1;
                        let mut points = points[..end].to_vec();
                        let mut modes = t.modes[..end].to_vec();
                        points.extend(tower.roof_path(tower.hatch(), tower.slot(slot)).unwrap());
                        modes.resize(points.len(), 0);
                        let mut updated = t;
                        updated.path = points.into_iter().flat_map(|p| [p.0, p.1, p.2]).collect();
                        updated.modes = modes;
                        ctx.db.tower_travel().unit_id().update(updated);
                        o.slot = slot as u8;
                        ctx.db.tower_occupant().unit_id().update(o);
                        accepted += 1;
                        continue;
                    }
                    (
                        tower
                            .roof_path((u.x, u.z), tower.slot(slot))
                            .ok_or("Roof destination is unreachable.")?,
                        Vec::new(),
                        t.active,
                    )
                } else {
                    (
                        tower
                            .roof_path((u.x, u.z), tower.slot(slot))
                            .ok_or("Roof destination is unreachable.")?,
                        Vec::new(),
                        false,
                    )
                };
                modes.resize(points.len(), 0);
                u.target_x = tower.slot(slot).0;
                u.target_z = tower.slot(slot).1;
                store_travel(
                    ctx,
                    &u,
                    b.id,
                    std::mem::take(&mut points),
                    modes,
                    false,
                    active,
                );
                // Hold the current slot until the roof walk has completed.
                o.next_building = Some(b.id);
                o.next_slot = slot as u8;
                ctx.db.tower_occupant().unit_id().update(o);
                ctx.db.army_unit().id().update(u);
            } else if !on_ground(ctx, &u)
                && ctx.db.tower_travel().unit_id().find(u.id).is_none()
                && map::walls::surface_route(
                    o.building_id,
                    b.id,
                    &roof_pieces(ctx, b.game_id, b.owner),
                )
                .is_some()
            {
                let structures = roof_pieces(ctx, b.game_id, b.owner);
                let route = map::walls::surface_route(o.building_id, b.id, &structures).unwrap();
                let current = ctx.db.building().id().find(o.building_id).unwrap();
                let old_roof = tower_geometry(ctx, &current);
                let mut points = old_roof
                    .roof_path((u.x, u.z), route[0])
                    .ok_or("Der Dachübergang ist blockiert.")?;
                points.extend(
                    route
                        .iter()
                        .skip(1)
                        .map(|p| (p.0, tower.roof_y() + roof::UNIT_CENTER_HEIGHT, p.1)),
                );
                points.extend(
                    tower
                        .roof_path(*route.last().unwrap(), tower.slot(slot))
                        .ok_or("Dach nicht erreichbar.")?,
                );
                let modes = vec![4; points.len()];
                store_travel(ctx, &u, b.id, points, modes, false, false);
                o.next_building = Some(b.id);
                o.next_slot = slot as u8;
                o.door = 0;
                ctx.db.tower_occupant().unit_id().update(o);
                ctx.db.unit_order().unit_id().delete(u.id);
            } else {
                depart_tower(ctx, &u)?;
                ctx.db.unit_order().unit_id().delete(u.id);
                if let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) {
                    o.next_building = Some(b.id);
                    o.next_slot = slot as u8;
                    ctx.db.tower_occupant().unit_id().update(o);
                } else {
                    start_roof_entry(ctx, u, &b, slot as u8)?;
                }
            }
        } else {
            start_roof_entry(ctx, u, &b, slot as u8)?;
        }
        accepted += 1;
    }
    if accepted == 0 {
        return Err(format!("The roof is full ({} units).", tower.capacity()));
    }
    start_tick(ctx);
    Ok(())
}

/// Walk to the hatch, then teleport to the door before a ground/attack order.
fn depart_tower(ctx: &ReducerContext, u: &ArmyUnit) -> Result<map::Point, String> {
    let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) else {
        return Ok((u.x, u.z));
    };
    o.next_building = None;
    if ctx
        .db
        .tower_travel()
        .unit_id()
        .find(u.id)
        .is_some_and(|t| t.leaving)
    {
        ctx.db.tower_occupant().unit_id().update(o);
        return Ok(ground_start(ctx, u));
    }
    if on_ground(ctx, u) {
        clear_tower(ctx, u.id);
        return Ok((u.x, u.z));
    }
    let b = ctx
        .db
        .building()
        .id()
        .find(o.building_id)
        .ok_or("Tower no longer exists.")?;
    if is_wall(&b) {
        return depart_wall(ctx, u, &b, o);
    }
    let tower = tower_geometry(ctx, &b);
    if !door_clear(ctx, &b, o.door) {
        o.door = (0..tower.door_count())
            .find(|door| door_clear(ctx, &b, *door))
            .ok_or("All tower doors are blocked. Clear a doorway first.")?;
    }
    let door = tower.door(o.door);
    // The external keep staircase ends outside the terrace's walkable rectangle.
    // A new ground order while crossing that landing must not require an impossible
    // roof path from the stairs. Finish the landing, then walk back down physically.
    let on_keep_stairs = b.kind == "keep"
        && ctx
            .db
            .tower_travel()
            .unit_id()
            .find(u.id)
            .is_some_and(|t| !t.leaving && t.modes.get(t.next as usize) == Some(&4));
    let mut points = if on_keep_stairs {
        Vec::new()
    } else {
        tower
            .roof_path((u.x, u.z), tower.hatch_for_door(o.door))
            .ok_or("Cannot reach the hatch.")?
    };
    let mut modes = vec![0; points.len()];
    if b.kind == "keep" || b.kind == "drawbridge_gate" {
        let top = tower.world(if b.kind == "keep" {
            (if o.door == 1 { -6.5 } else { 6.5 }, 3.0)
        } else {
            (6.4, 2.5)
        });
        points.push((top.0, tower.roof_y() + roof::UNIT_CENTER_HEIGHT, top.1));
        modes.push(4);
        points.push((
            door.0,
            terrain_height(door.0, door.1) + roof::UNIT_CENTER_HEIGHT,
            door.1,
        ));
        modes.push(4);
    } else {
        points.push((door.0, b.y + roof::UNIT_CENTER_HEIGHT, door.1));
        modes.push(4);
    }
    ctx.db.tower_occupant().unit_id().update(o);
    store_travel(ctx, u, b.id, points, modes, true, false);
    Ok(door)
}

fn replan_tower_approach(ctx: &ReducerContext, u: ArmyUnit) {
    let Some(o) = ctx.db.tower_occupant().unit_id().find(u.id) else {
        return;
    };
    let Some(b) = ctx.db.building().id().find(o.building_id) else {
        return;
    };
    clear_tower(ctx, u.id);
    if start_roof_entry(ctx, u.clone(), &b, o.slot).is_err() {
        stop_unit(ctx, u);
    }
}

fn step_tower_travel(ctx: &ReducerContext) -> std::collections::HashSet<u64> {
    let mut moved = std::collections::HashSet::new();
    let mut travels = ctx.db.tower_travel().iter().collect::<Vec<_>>();
    // Every traveler advances independently; teleportation has no per-tower queue.
    travels.sort_by_key(|t| t.unit_id);
    for mut t in travels {
        let Some(mut u) = ctx.db.army_unit().id().find(t.unit_id) else {
            clear_tower(ctx, t.unit_id);
            continue;
        };
        if ctx.db.participant().connection_id().find(u.owner).is_none() {
            continue;
        }
        let Some(b) = ctx.db.building().id().find(t.building_id) else {
            clear_tower(ctx, t.unit_id);
            continue;
        };
        if !t.leaving && (t.modes.contains(&1) || t.modes.contains(&2)) && on_ground(ctx, &u) {
            replan_tower_approach(ctx, u);
            continue;
        }
        if !t.leaving && ctx.db.unit_order().unit_id().find(u.id).is_some() {
            let o = ctx.db.tower_occupant().unit_id().find(u.id).unwrap();
            let Some(_access) = access_building(ctx, u.id, &b) else {
                clear_tower(ctx, u.id);
                stop_unit(ctx, u);
                continue;
            };
            let Some(door) = access_door(ctx, u.id, &b, o.door) else {
                clear_tower(ctx, u.id);
                continue;
            };
            if map::distance((u.x, u.z), door) > 0.55 {
                continue;
            }
            ctx.db.unit_order().unit_id().delete(u.id);
        }
        // Replace any in-flight stair route from the preceding server version.
        if t.modes.contains(&1) || t.modes.contains(&2) {
            let o = ctx.db.tower_occupant().unit_id().find(u.id).unwrap();
            let tower = tower_geometry(ctx, &b);
            let target = if t.leaving {
                tower.door(o.door)
            } else {
                tower.hatch()
            };
            let y = if t.leaving {
                b.y + roof::UNIT_CENTER_HEIGHT
            } else {
                tower.roof_y() + roof::UNIT_CENTER_HEIGHT
            };
            let mut points = vec![(target.0, y, target.1)];
            let mut modes = vec![3];
            if !t.leaving {
                points.extend(
                    tower
                        .roof_path(target, tower.slot(o.slot as usize))
                        .unwrap(),
                );
                modes.resize(points.len(), 0);
            }
            t.path = points.into_iter().flat_map(|p| [p.0, p.1, p.2]).collect();
            t.modes = modes;
            t.next = 0;
        }
        moved.insert(u.id);
        t.active = true;
        let tower = tower_geometry(ctx, &b);
        let points = travel_points(&t);
        let mut budget = UNIT_SPEED * TICK_SECONDS;
        while (t.next as usize) < points.len() && budget > 0.00001 {
            if unit_combat_facing(ctx, &u).is_some() {
                break;
            }
            let i = t.next as usize;
            let goal = points[i];
            let start = (u.x, u.y, u.z);
            if t.modes[i] == 3 || t.modes[i] == 5 {
                // Door <-> hatch is instantaneous and may run for every unit this tick.
                if t.leaving && t.modes[i] == 3 {
                    let obstacles = obstacles(ctx, u.game_id);
                    let reserved = ctx
                        .db
                        .army_unit()
                        .game_id()
                        .filter(u.game_id)
                        .filter(|peer| peer.id != u.id && on_ground(ctx, peer))
                        .map(|peer| (peer.x, peer.z))
                        .collect::<Vec<_>>();
                    let Some(landing) = free_destination((goal.0, goal.2), &reserved, &obstacles)
                    else {
                        break;
                    };
                    u.x = landing.0;
                    u.z = landing.1;
                    u.y = terrain_height(u.x, u.z) + roof::UNIT_CENTER_HEIGHT;
                    if let Some(order) = ctx.db.unit_order().unit_id().find(u.id) {
                        if let Some(path) =
                            map::shortest_path(landing, (u.target_x, u.target_z), &obstacles)
                        {
                            save_order(ctx, &u, order.attack_building, path);
                        } else {
                            stop_unit(ctx, u.clone());
                        }
                    }
                } else {
                    u.x = goal.0;
                    u.y = goal.1;
                    u.z = goal.2;
                }
                t.next += 1;
                continue;
            }
            if t.modes[i] == 4 {
                let length = map::distance((u.x, u.z), (goal.0, goal.2)).hypot(goal.1 - u.y);
                let fraction = if length < 0.00001 {
                    1.
                } else {
                    (budget / length).min(1.)
                };
                if ctx.db.unit_engagement().unit_id().find(u.id).is_some() {
                    let peers = combat_roof_peers(ctx, &u);
                    let p = move_separated((u.x, u.z), (goal.0, goal.2), budget, &peers, &[]);
                    let y = u.y + (goal.1 - u.y) * fraction;
                    if !map::walls::surface_supports(
                        (p.0, y, p.1),
                        &combat_roof_pieces(ctx, u.game_id),
                    ) {
                        break;
                    }
                    u.x = p.0;
                    u.z = p.1;
                    u.y = y;
                } else {
                    u.x += (goal.0 - u.x) * fraction;
                    u.z += (goal.2 - u.z) * fraction;
                    u.y += (goal.1 - u.y) * fraction;
                }
            } else {
                let obstacles = tower.roof_obstacles();
                let peers = if ctx.db.unit_engagement().unit_id().find(u.id).is_some() {
                    combat_roof_peers(ctx, &u)
                } else {
                    Vec::new()
                };
                let p = move_separated((u.x, u.z), (goal.0, goal.2), budget, &peers, &obstacles);
                u.x = p.0;
                u.z = p.1;
                u.y = tower.roof_y() + roof::UNIT_CENTER_HEIGHT;
            }
            let travelled =
                ((u.x - start.0).powi(2) + (u.y - start.1).powi(2) + (u.z - start.2).powi(2))
                    .sqrt();
            budget -= travelled;
            if ((u.x - goal.0).powi(2) + (u.y - goal.1).powi(2) + (u.z - goal.2).powi(2)).sqrt()
                < 0.0001
            {
                u.x = goal.0;
                u.y = goal.1;
                u.z = goal.2;
                t.next += 1;
            } else {
                break;
            }
        }
        if (t.next as usize) == points.len() {
            ctx.db.tower_travel().unit_id().delete(u.id);
            if t.leaving {
                let pending = ctx
                    .db
                    .tower_occupant()
                    .unit_id()
                    .find(u.id)
                    .and_then(|o| o.next_building.map(|id| (id, o.next_slot)));
                clear_tower(ctx, u.id);
                if let Some((id, slot)) = pending {
                    ctx.db.army_unit().id().update(u.clone());
                    if let Some(next) = ctx.db.building().id().find(id) {
                        let entered = if ctx.db.unit_engagement().unit_id().find(u.id).is_some() {
                            combat_entry(ctx, u.clone(), &next, slot)
                        } else {
                            start_roof_entry(ctx, u.clone(), &next, slot).is_ok()
                        };
                        if entered {
                            continue;
                        }
                    }
                }
            }
            if !t.leaving {
                if let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) {
                    if let Some(next) = o.next_building {
                        o.building_id = next;
                        o.slot = o.next_slot;
                        o.next_building = None;
                        ctx.db.tower_occupant().unit_id().update(o);
                    }
                }
            }
            if ctx.db.unit_order().unit_id().find(u.id).is_none() {
                u.target_x = u.x;
                u.target_z = u.z;
            }
        } else {
            ctx.db.tower_travel().unit_id().update(t);
        }
        ctx.db.army_unit().id().update(u);
    }
    moved
}

fn evacuate_tower(ctx: &ReducerContext, building_id: u64) {
    for mut o in ctx.db.tower_occupant().iter().collect::<Vec<_>>() {
        if o.next_building == Some(building_id) {
            o.next_building = None;
            ctx.db.tower_occupant().unit_id().update(o.clone());
        }
        if o.building_id != building_id {
            continue;
        }
        clear_tower(ctx, o.unit_id);
        if let Some(mut u) = ctx.db.army_unit().id().find(o.unit_id) {
            let obstacles = obstacles(ctx, u.game_id);
            let reserved = ctx
                .db
                .army_unit()
                .game_id()
                .filter(u.game_id)
                .filter(|p| p.id != u.id && on_ground(ctx, p))
                .map(|p| (p.x, p.z))
                .collect::<Vec<_>>();
            if let Some(p) = free_destination((u.x, u.z), &reserved, &obstacles) {
                u.x = p.0;
                u.z = p.1;
            }
            u.y = terrain_height(u.x, u.z) + roof::UNIT_CENTER_HEIGHT;
            stop_unit(ctx, u);
        }
    }
}

// Refit saved occupants and destinations when a roof layout changes on deployment.
fn normalize_roof_slots(ctx: &ReducerContext) {
    for o in ctx.db.tower_occupant().iter().collect::<Vec<_>>() {
        if ctx.db.unit_engagement().unit_id().find(o.unit_id).is_some() {
            continue;
        }
        let Some(b) = ctx.db.building().id().find(o.building_id) else {
            continue;
        };
        let Some(u) = ctx.db.army_unit().id().find(o.unit_id) else {
            continue;
        };
        if ctx
            .db
            .tower_travel()
            .unit_id()
            .find(u.id)
            .is_some_and(|t| t.modes.contains(&4))
        {
            continue;
        }
        let tower = tower_geometry(ctx, &b);
        let slot = if o.next_building == Some(b.id) {
            o.next_slot
        } else {
            o.slot
        };
        let goal = tower.slot(slot as usize);
        if let Some(mut t) = ctx.db.tower_travel().unit_id().find(u.id) {
            if t.leaving {
                continue;
            }
            let n = t.path.len();
            if n >= 3 && map::distance((t.path[n - 3], t.path[n - 1]), goal) > 0.001 {
                t.path[n - 3] = goal.0;
                t.path[n - 2] = tower.roof_y() + roof::UNIT_CENTER_HEIGHT;
                t.path[n - 1] = goal.1;
                ctx.db.tower_travel().unit_id().update(t);
            }
        } else if ctx.db.unit_order().unit_id().find(u.id).is_none()
            && map::distance((u.x, u.z), goal) > 0.001
        {
            if let Some(path) = tower.roof_path((u.x, u.z), goal) {
                let modes = vec![0; path.len()];
                store_travel(ctx, &u, b.id, path, modes, false, false);
            }
        }
    }
}
