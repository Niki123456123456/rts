// Combat uses physical roof connectivity, including enemy fortifications.
#[spacetimedb::table(name = combat_surface_home)]
pub struct CombatSurfaceHome {
    #[primary_key]
    pub unit_id: u64,
    pub building_id: u64,
    pub slot: u8,
}
fn combat_roof_pieces(ctx: &ReducerContext, game: u64) -> Vec<map::walls::RoofPiece> {
    ctx.db
        .building()
        .game_id()
        .filter(game)
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
        .collect()
}
fn combat_roof_peers(ctx: &ReducerContext, u: &ArmyUnit) -> Vec<map::Point> {
    ctx.db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|v| v.id != u.id && (v.y - u.y).abs() < 0.6)
        .map(|v| (v.x, v.z))
        .collect()
}
fn combat_surface(ctx: &ReducerContext, u: &ArmyUnit) -> Option<map::walls::RoofPiece> {
    if on_ground(ctx, u) {
        return None;
    }
    combat_roof_pieces(ctx, u.game_id)
        .into_iter()
        .filter(|p| {
            p.roof().roof_contains((u.x, u.z))
                && (p.roof().roof_y() + roof::UNIT_CENTER_HEIGHT - u.y).abs() < 0.2
        })
        .min_by(|a, b| {
            map::distance((a.x, a.z), (u.x, u.z)).total_cmp(&map::distance((b.x, b.z), (u.x, u.z)))
        })
}
fn combat_surface_candidates(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    target: &ArmyUnit,
) -> Vec<(map::walls::RoofPiece, usize)> {
    let Some(surface) = combat_surface(ctx, target) else {
        return Vec::new();
    };
    let pieces = combat_roof_pieces(ctx, u.game_id);
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|v| v.id != u.id)
        .collect::<Vec<_>>();
    let mut slots = map::walls::surface_component(surface.id, &pieces)
        .into_iter()
        .flat_map(|p| (0..p.roof().capacity()).map(move |slot| (p, slot)))
        .filter(|(p, s)| {
            !slot_reserved(ctx, p.id, *s as u8, u.id)
                && peers.iter().all(|v| {
                    let point = p.roof().slot(*s);
                    (v.y - p.roof().roof_y() - roof::UNIT_CENTER_HEIGHT).abs() > 0.6
                        || map::distance(point, (v.x, v.z)) >= map::UNIT_SPACING
                })
        })
        .collect::<Vec<_>>();
    slots.sort_by(|(a, i), (b, j)| {
        let score = |p: map::walls::RoofPiece, s: usize| {
            let q = p.roof().slot(s);
            let d = map::distance(q, (target.x, target.z))
                .hypot(p.roof().roof_y() + roof::UNIT_CENTER_HEIGHT - target.y);
            d * 10. + map::distance(q, (u.x, u.z)) * 0.01
        };
        score(*a, *i)
            .total_cmp(&score(*b, *j))
            .then(a.id.cmp(&b.id))
            .then(i.cmp(j))
    });
    slots
}
fn combat_entry(ctx: &ReducerContext, u: ArmyUnit, b: &Building, slot: u8) -> bool {
    let pieces = combat_roof_pieces(ctx, u.game_id);
    let plan = if is_wall(b) {
        wall_entry_plan_using(ctx, &u, b, &pieces)
    } else {
        entry_plan(ctx, &u, b, slot)
    };
    let Some((access, door, approach, mut points, mut modes)) = plan else {
        return false;
    };
    if is_wall(b) {
        let Some(route) = map::walls::tower_routes(b.id, &pieces)
            .into_iter()
            .find(|(id, _)| *id == access)
            .map(|(_, route)| route)
        else {
            return false;
        };
        let tower = pieces.iter().find(|p| p.id == access).unwrap().roof();
        points = vec![
            (
                route[0].0,
                tower.roof_y() + roof::UNIT_CENTER_HEIGHT,
                route[0].1,
            ),
            (
                route[1].0,
                tower.roof_y() + roof::UNIT_CENTER_HEIGHT,
                route[1].1,
            ),
        ];
        modes = vec![3, 4];
        if (points[1].1 - wall_level(b)).abs() > 0.01 {
            points.push((route[1].0, wall_level(b), route[1].1));
            modes.push(5);
        }
        points.extend(route.into_iter().skip(2).map(|p| (p.0, wall_level(b), p.1)));
        modes.resize(points.len(), 4);
    }
    start_roof_entry_using(ctx, u, b, slot, (access, door, approach, points, modes)).is_ok()
}
fn combat_surface_move(
    ctx: &ReducerContext,
    u: &mut ArmyUnit,
    dest: map::walls::RoofPiece,
    slot: usize,
) -> bool {
    if ctx.db.tower_travel().unit_id().find(u.id).is_some() {
        return true;
    }
    let Some(b) = ctx.db.building().id().find(dest.id) else {
        return false;
    };
    if on_ground(ctx, u) {
        // Compute the entry before discarding a saved reservation.
        let available = if is_wall(&b) {
            wall_entry_plan_using(ctx, u, &b, &combat_roof_pieces(ctx, u.game_id)).is_some()
        } else {
            entry_plan(ctx, u, &b, slot as u8).is_some()
        };
        if !available {
            return false;
        }
        clear_tower(ctx, u.id);
        if combat_entry(ctx, u.clone(), &b, slot as u8) {
            start_tick(ctx);
            return true;
        }
        return false;
    }
    let Some(start) = combat_surface(ctx, u) else {
        return false;
    };
    let pieces = combat_roof_pieces(ctx, u.game_id);
    let goal = dest.roof().slot(slot);
    let mut points = Vec::new();
    let mut modes = Vec::new();
    if start.id == dest.id {
        let Some(path) = dest.roof().roof_path((u.x, u.z), goal) else {
            return false;
        };
        points = path;
        modes.resize(points.len(), 4);
    } else {
        let route = if start.kind == map::BuildingKind::StoneWall
            && dest.kind == map::BuildingKind::StoneWall
        {
            map::walls::wall_path(start.id, dest.id, &pieces)
        } else if start.kind != map::BuildingKind::StoneWall
            && dest.kind == map::BuildingKind::StoneWall
        {
            map::walls::tower_routes(dest.id, &pieces)
                .into_iter()
                .find(|(id, _)| *id == start.id)
                .map(|(_, r)| r)
        } else if start.kind == map::BuildingKind::StoneWall
            && dest.kind != map::BuildingKind::StoneWall
        {
            map::walls::tower_routes(start.id, &pieces)
                .into_iter()
                .find(|(id, _)| *id == dest.id)
                .map(|(_, mut r)| {
                    r.reverse();
                    r
                })
        } else {
            None
        };
        let Some(route) = route else {
            // Separate towers retain their ordinary door/hatch transfer.
            if depart_tower(ctx, u).is_err() {
                return false;
            }
            if let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) {
                o.next_building = Some(dest.id);
                o.next_slot = slot as u8;
                ctx.db.tower_occupant().unit_id().update(o);
                start_tick(ctx);
                return true;
            }
            return false;
        };
        let mut y = u.y;
        for (index, p) in route.iter().enumerate() {
            if index == 0 && start.kind != map::BuildingKind::StoneWall {
                let Some(path) = start.roof().roof_path((u.x, u.z), *p) else {
                    return false;
                };
                points.extend(path);
                modes.resize(points.len(), 4);
            } else {
                points.push((p.0, y, p.1));
                modes.push(4);
            }
            // Change height at the joining edge, never walk under a tower roof.
            if start.kind != map::BuildingKind::StoneWall && index == 1 {
                y = dest.roof().roof_y() + roof::UNIT_CENTER_HEIGHT;
                if (points.last().unwrap().1 - y).abs() > 0.01 {
                    points.push((p.0, y, p.1));
                    modes.push(5);
                }
            }
            if start.kind == map::BuildingKind::StoneWall
                && dest.kind != map::BuildingKind::StoneWall
                && index + 2 == route.len()
            {
                let next_y = dest.roof().roof_y() + roof::UNIT_CENTER_HEIGHT;
                if (y - next_y).abs() > 0.01 {
                    y = next_y;
                    points.push((p.0, y, p.1));
                    modes.push(5);
                }
            }
        }
        let desired_y = dest.roof().roof_y() + roof::UNIT_CENTER_HEIGHT;
        if (y - desired_y).abs() > 0.01 {
            let p = *route.last().unwrap();
            points.push((p.0, desired_y, p.1));
            modes.push(5);
        }
        let Some(path) = dest.roof().roof_path(*route.last().unwrap(), goal) else {
            return false;
        };
        points.extend(path);
        modes.resize(points.len(), 4);
    }
    let Some(mut o) = ctx.db.tower_occupant().unit_id().find(u.id) else {
        return false;
    };
    o.building_id = dest.id;
    o.slot = slot as u8;
    o.next_building = None;
    if is_wall(&b) && ctx.db.wall_entry().unit_id().find(u.id).is_none() {
        let access = if start.kind != map::BuildingKind::StoneWall {
            start.id
        } else {
            map::walls::tower_routes(dest.id, &pieces)
                .first()
                .map(|r| r.0)
                .unwrap_or(dest.id)
        };
        ctx.db.wall_entry().insert(WallEntry {
            unit_id: u.id,
            tower_id: access,
        });
    }
    ctx.db.unit_order().unit_id().delete(u.id);
    ctx.db.tower_occupant().unit_id().update(o);
    u.target_x = goal.0;
    u.target_z = goal.1;
    ctx.db.army_unit().id().update(u.clone());
    store_travel(ctx, u, dest.id, points, modes, false, false);
    start_tick(ctx);
    true
}
fn approach_surface_enemy(ctx: &ReducerContext, u: &mut ArmyUnit, target: &ArmyUnit) -> bool {
    if ctx.db.tower_travel().unit_id().find(u.id).is_some() {
        return true;
    }
    for (piece, slot) in combat_surface_candidates(ctx, u, target) {
        if combat_surface_move(ctx, u, piece, slot) {
            return true;
        }
    }
    false
}
