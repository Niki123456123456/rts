use map::combat as melee;
#[derive(Clone)]
#[spacetimedb::table(name = unit_engagement)]
pub struct UnitEngagement {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub target: u64,
    pub enemies: Vec<u64>,
    pub automatic: bool,
    pub home_x: f32,
    pub home_z: f32,
    pub returning: bool,
}
#[spacetimedb::table(name = combat_tick, scheduled(scan_combat))]
pub struct CombatTick {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}
fn combatants(ctx: &ReducerContext) -> Vec<ArmyUnit> {
    ctx.db
        .army_unit()
        .iter()
        .filter(|u| {
            u.health > 0.
                && ctx.db.unit_profile().unit_id().find(u.id).is_some()
                && ctx.db.participant().connection_id().find(u.owner).is_some()
        })
        .collect()
}
fn hostile_pair(units: &[ArmyUnit]) -> bool {
    units.iter().any(|u| {
        units
            .iter()
            .any(|v| u.game_id == v.game_id && u.owner != v.owner)
    })
}
fn start_combat_scan(ctx: &ReducerContext) {
    if ctx.db.combat_tick().iter().next().is_none()
        && (ctx.db.unit_engagement().iter().next().is_some()
            || ctx.db.ranged_building_order().iter().next().is_some()
            || ctx.db.arrow_projectile().iter().next().is_some()
            || hostile_pair(&combatants(ctx)))
    {
        ctx.db.combat_tick().insert(CombatTick {
            scheduled_id: 0,
            scheduled_at: Duration::from_millis(250).into(),
        });
    }
}
fn combat_group(ctx: &ReducerContext, target: &ArmyUnit) -> Vec<u64> {
    let members = combatants(ctx)
        .into_iter()
        .filter(|u| u.game_id == target.game_id)
        .map(|u| melee::GroupMember {
            id: u.id,
            side: u.color,
            point: (u.x, u.z),
            height: u.y,
        })
        .collect::<Vec<_>>();
    melee::enemy_group(target.id, &members)
}
fn valid_enemy(ctx: &ReducerContext, u: &ArmyUnit, id: u64) -> Option<ArmyUnit> {
    ctx.db.army_unit().id().find(id).filter(|v| {
        v.game_id == u.game_id
            && v.owner != u.owner
            && v.health > 0.
            && ctx.db.participant().connection_id().find(v.owner).is_some()
            && ctx.db.unit_profile().unit_id().find(v.id).is_some()
    })
}
fn melee_contact(ctx: &ReducerContext, u: &ArmyUnit, v: &ArmyUnit) -> bool {
    if (u.y - v.y).abs() > 0.6 || map::distance((u.x, u.z), (v.x, v.z)) > melee::MELEE_RANGE {
        return false;
    }
    if on_ground(ctx, u) && on_ground(ctx, v) {
        map::segment_clear((u.x, u.z), (v.x, v.z), &obstacles(ctx, u.game_id))
    } else {
        let pieces = combat_roof_pieces(ctx, u.game_id);
        let (Some(a), Some(b)) = (combat_surface(ctx, u), combat_surface(ctx, v)) else {
            return false;
        };
        map::walls::surface_component(a.id, &pieces)
            .iter()
            .any(|p| p.id == b.id)
            && map::walls::surface_contact((u.x, u.y, u.z), (v.x, v.y, v.z), &pieces)
    }
}
fn unit_combat_facing(ctx: &ReducerContext, u: &ArmyUnit) -> Option<(f32, f32)> {
    if let Some(facing) = ranged_building_facing(ctx, u) {
        return Some(facing);
    }
    let e = ctx
        .db
        .unit_engagement()
        .unit_id()
        .find(u.id)
        .filter(|e| !e.returning)?;
    let v = valid_enemy(ctx, u, e.target).filter(|v| attack_contact(ctx, u, v))?;
    Some((v.x - u.x, v.z - u.z))
}
fn combat_pose(ctx: &ReducerContext, u: &ArmyUnit) {
    let Some(mut p) = ctx.db.unit_profile().unit_id().find(u.id) else {
        return;
    };
    if let Some(facing) = unit_combat_facing(ctx, u) {
        let length = map::distance((0., 0.), facing).max(0.001);
        p.activity = 2;
        p.facing_x = facing.0 / length;
        p.facing_z = facing.1 / length;
        ctx.db.unit_profile().unit_id().update(p);
    } else if p.activity == 2 {
        p.activity = 0;
        ctx.db.unit_profile().unit_id().update(p);
    }
}
fn begin_engagement(ctx: &ReducerContext, u: &ArmyUnit, target: &ArmyUnit, automatic: bool) {
    ctx.db.combat_surface_home().unit_id().delete(u.id);
    if automatic && !on_ground(ctx, u) {
        if let Some(o) = ctx.db.tower_occupant().unit_id().find(u.id) {
            ctx.db.combat_surface_home().insert(CombatSurfaceHome {
                unit_id: u.id,
                building_id: o.building_id,
                slot: o.slot,
            });
        }
    }
    ctx.db.unit_order().unit_id().delete(u.id);
    ctx.db.unit_engagement().unit_id().delete(u.id);
    ctx.db.unit_engagement().insert(UnitEngagement {
        unit_id: u.id,
        game_id: u.game_id,
        target: target.id,
        enemies: combat_group(ctx, target),
        automatic,
        home_x: u.x,
        home_z: u.z,
        returning: false,
    });
}
fn approach_enemy(ctx: &ReducerContext, u: &mut ArmyUnit, target: &ArmyUnit) -> bool {
    if is_archer(ctx, u) {
        return approach_bow_target(ctx, u, &BowTarget::Unit(target.clone()));
    }
    if !on_ground(ctx, target) {
        return approach_surface_enemy(ctx, u, target);
    }
    let Ok(start) = depart_tower(ctx, u) else {
        return false;
    };
    let obstacles = obstacles(ctx, u.game_id);
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|v| v.id != u.id)
        .flat_map(|v| ground_reservations(ctx, &v))
        .collect::<Vec<_>>();
    // Reserve separate melee positions; farther rows wait until front positions open.
    for ring in 0..=3 {
        let radius = melee::APPROACH_RADIUS + ring as f32 * map::UNIT_SPACING;
        let mut slots = (0..16)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 16.;
                (target.x + a.cos() * radius, target.z + a.sin() * radius)
            })
            .collect::<Vec<_>>();
        slots.sort_by(|a, b| map::distance(*a, start).total_cmp(&map::distance(*b, start)));
        for p in slots.into_iter().filter(|p| {
            map::free(*p, &obstacles)
                && peers
                    .iter()
                    .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
        }) {
            if let Some(path) = map::shortest_path(start, p, &obstacles) {
                u.target_x = p.0;
                u.target_z = p.1;
                save_order(ctx, u, None, path);
                ctx.db.army_unit().id().update(u.clone());
                start_tick(ctx);
                return true;
            }
        }
    }
    false
}
fn return_from_combat(ctx: &ReducerContext, u: &mut ArmyUnit, e: &UnitEngagement) {
    if let Some(home) = ctx.db.combat_surface_home().unit_id().find(u.id) {
        if let Some(piece) = combat_roof_pieces(ctx, u.game_id)
            .into_iter()
            .find(|p| p.id == home.building_id)
        {
            let slot = if !slot_reserved(ctx, piece.id, home.slot, u.id) {
                home.slot as usize
            } else {
                let Some(slot) = piece
                    .roof()
                    .sorted_slots((e.home_x, e.home_z))
                    .into_iter()
                    .find(|slot| !slot_reserved(ctx, piece.id, *slot as u8, u.id))
                else {
                    return;
                };
                slot
            };
            let goal = piece.roof().slot(slot);
            if (u.y - piece.roof().roof_y() - roof::UNIT_CENTER_HEIGHT).abs() < 0.1
                && map::distance((u.x, u.z), goal) < 0.1
            {
                ctx.db.unit_engagement().unit_id().delete(u.id);
                ctx.db.combat_surface_home().unit_id().delete(u.id);
                combat_pose(ctx, u);
                return;
            }
            combat_surface_move(ctx, u, piece, slot);
            return;
        }
        ctx.db.combat_surface_home().unit_id().delete(u.id);
    }
    if ctx.db.tower_travel().unit_id().find(u.id).is_some() {
        return;
    }
    let Ok(start) = depart_tower(ctx, u) else {
        return;
    };
    let obs = obstacles(ctx, u.game_id);
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|v| v.id != u.id)
        .flat_map(|v| ground_reservations(ctx, &v))
        .collect::<Vec<_>>();
    let Some(home) = free_destination((e.home_x, e.home_z), &peers, &obs) else {
        return;
    };
    if map::distance((u.x, u.z), home) < 0.1 {
        ctx.db.unit_engagement().unit_id().delete(u.id);
        stop_unit(ctx, u.clone());
        combat_pose(ctx, u);
        return;
    }
    if ctx.db.unit_order().unit_id().find(u.id).is_none()
        || map::distance((u.target_x, u.target_z), home) > 0.2
    {
        if let Some(path) = map::shortest_path(start, home, &obs) {
            u.target_x = home.0;
            u.target_z = home.1;
            save_order(ctx, u, None, path);
            ctx.db.army_unit().id().update(u.clone());
            start_tick(ctx);
        }
    }
}
fn kill_combat_unit(ctx: &ReducerContext, id: u64) {
    ctx.db.lord_vitality().unit_id().delete(id);
    ctx.db.hunt_order().unit_id().delete(id);
    if let Some(w) = ctx.db.worker().unit_id().find(id) {
        if let Some(b) = ctx.db.building().id().find(w.source_building) {
            if let Some(k) = ctx
                .db
                .building()
                .owner()
                .filter(b.owner)
                .find(|b| b.kind == "keep")
            {
                if let Some(mut p) = ctx.db.keep_population().building_id().find(k.id) {
                    p.arrived = p.arrived.saturating_sub(1);
                    ctx.db.keep_population().building_id().update(p);
                }
            }
        }
        ctx.db.worker().unit_id().delete(id);
    }

    clear_tower(ctx, id);
    ctx.db.unit_order().unit_id().delete(id);
    ctx.db.unit_engagement().unit_id().delete(id);
    ctx.db.combat_surface_home().unit_id().delete(id);
    ctx.db.archer_cooldown().unit_id().delete(id);
    ctx.db.ranged_building_order().unit_id().delete(id);
    ctx.db.unit_profile().unit_id().delete(id);
    ctx.db.army_unit().id().delete(id);
}
#[spacetimedb::reducer]
pub fn attack_units(
    ctx: &ReducerContext,
    unit_ids: Vec<u64>,
    target_unit_id: u64,
) -> Result<(), String> {
    let player = participant(ctx)?;
    let units = owned_units(ctx, &player, unit_ids)?;
    let target = valid_enemy(ctx, &units[0], target_unit_id)
        .ok_or("Choose an enemy combat unit in your game.")?;
    let mut accepted = 0;
    let group = combat_group(ctx, &target);
    for mut u in units {
        let Some(enemy) = group
            .iter()
            .filter_map(|id| valid_enemy(ctx, &u, *id))
            .min_by(|a, b| {
                let score = |v: &ArmyUnit| {
                    map::distance((u.x, u.z), (v.x, v.z))
                        + ctx
                            .db
                            .unit_engagement()
                            .iter()
                            .filter(|e| e.target == v.id && !e.returning)
                            .count() as f32
                            * 2.
                };
                score(a).total_cmp(&score(b))
            })
        else {
            continue;
        };
        begin_engagement(ctx, &u, &enemy, false);
        if attack_contact(ctx, &u, &enemy) {
            stop_unit(ctx, u.clone());
            accepted += 1;
        } else if approach_enemy(ctx, &mut u, &enemy) {
            accepted += 1;
        } else {
            ctx.db.unit_engagement().unit_id().delete(u.id);
        }
    }
    if accepted == 0 {
        return Err("No reachable position near the enemy group.".into());
    }
    start_combat_scan(ctx);
    Ok(())
}
#[spacetimedb::reducer]
pub fn scan_combat(ctx: &ReducerContext, tick: CombatTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Combat is server controlled.".into());
    }
    step_arrows(ctx);
    step_ranged_buildings(ctx);
    let units = combatants(ctx);
    // Only idle combatants acquire targets; a new explicit move order wins.
    for u in &units {
        if combat_kind(ctx, u).is_support() {
            continue;
        }
        if ctx.db.unit_engagement().unit_id().find(u.id).is_some()
            || ctx
                .db
                .ranged_building_order()
                .unit_id()
                .find(u.id)
                .is_some()
            || ctx
                .db
                .unit_order()
                .unit_id()
                .find(u.id)
                .is_some_and(|o| o.attack_building.is_none() || !o.waypoints.is_empty())
            || ctx.db.tower_travel().unit_id().find(u.id).is_some()
        {
            continue;
        }
        let mut near = units
            .iter()
            .filter(|v| {
                v.game_id == u.game_id
                    && v.owner != u.owner
                    && (if is_archer(ctx, u) {
                        attack_contact(ctx, u, v)
                    } else {
                        (on_ground(ctx, u) || (!on_ground(ctx, v) && (v.y - u.y).abs() < 0.6))
                            && map::distance((u.x, u.z), (v.x, v.z)).hypot(u.y - v.y)
                                <= melee::SENSE_RADIUS
                    })
            })
            .collect::<Vec<_>>();
        near.sort_by(|a, b| {
            map::distance((u.x, u.z), (a.x, a.z)).total_cmp(&map::distance((u.x, u.z), (b.x, b.z)))
        });
        for enemy in near {
            let mut copy = u.clone();
            let previous_order = ctx.db.unit_order().unit_id().find(u.id);
            begin_engagement(ctx, u, enemy, true);
            if attack_contact(ctx, u, enemy)
                || (!is_archer(ctx, u) && approach_enemy(ctx, &mut copy, enemy))
            {
                break;
            }
            ctx.db.unit_engagement().unit_id().delete(u.id);
            ctx.db.combat_surface_home().unit_id().delete(u.id);
            if let Some(order) = previous_order {
                ctx.db.unit_order().unit_id().delete(u.id);
                ctx.db.unit_order().insert(order);
            }
        }
    }
    let mut damage = std::collections::BTreeMap::<u64, f32>::new();
    for mut e in ctx.db.unit_engagement().iter().collect::<Vec<_>>() {
        let Some(mut u) = ctx.db.army_unit().id().find(e.unit_id) else {
            ctx.db.unit_engagement().unit_id().delete(e.unit_id);
            continue;
        };
        if e.returning {
            return_from_combat(ctx, &mut u, &e);
            continue;
        }
        let eligible = |v: &ArmyUnit| {
            !e.automatic
                || (if is_archer(ctx, &u) {
                    attack_contact(ctx, &u, v)
                } else {
                    map::distance((e.home_x, e.home_z), (v.x, v.z)) <= melee::PURSUIT_RADIUS
                        && map::distance((e.home_x, e.home_z), (u.x, u.z)) <= melee::PURSUIT_RADIUS
                })
        };
        let current = e
            .enemies
            .iter()
            .filter_map(|id| valid_enemy(ctx, &u, *id))
            .filter(|v| eligible(v) && attack_contact(ctx, &u, v))
            .min_by(|a, b| {
                map::distance((u.x, u.z), (a.x, a.z))
                    .total_cmp(&map::distance((u.x, u.z), (b.x, b.z)))
            })
            .or_else(|| valid_enemy(ctx, &u, e.target).filter(|v| eligible(v)));
        let target = if current.is_some() {
            current
        } else {
            e.enemies
                .iter()
                .filter_map(|id| valid_enemy(ctx, &u, *id))
                .filter(|v| eligible(v))
                .min_by(|a, b| {
                    let score = |v: &ArmyUnit| {
                        map::distance((u.x, u.z), (v.x, v.z))
                            + ctx
                                .db
                                .unit_engagement()
                                .iter()
                                .filter(|other| {
                                    other.target == v.id
                                        && other.unit_id != u.id
                                        && !other.returning
                                })
                                .count() as f32
                                * 1.2
                    };
                    score(a).total_cmp(&score(b))
                })
        };
        let Some(enemy) = target else {
            if is_archer(ctx, &u) && e.automatic {
                ctx.db.unit_engagement().unit_id().delete(u.id);
                ctx.db.combat_surface_home().unit_id().delete(u.id);
                combat_pose(ctx, &u);
                continue;
            }
            if on_ground(ctx, &u)
                && (u.y - terrain_height(u.x, u.z) - roof::UNIT_CENTER_HEIGHT).abs() < 0.1
            {
                clear_tower(ctx, u.id);
            } else if combat_surface(ctx, &u).is_some() {
                ctx.db.tower_travel().unit_id().delete(u.id);
            }
            stop_unit(ctx, u.clone());
            combat_pose(ctx, &u);
            if e.automatic {
                e.returning = true;
                e.target = 0;
                ctx.db.unit_engagement().unit_id().update(e.clone());
                return_from_combat(ctx, &mut u, &e);
            } else {
                ctx.db.unit_engagement().unit_id().delete(u.id);
                combat_pose(ctx, &u);
            }
            continue;
        };
        if e.target != enemy.id {
            e.target = enemy.id;
            ctx.db.unit_engagement().unit_id().update(e.clone());
            ctx.db.unit_order().unit_id().delete(u.id);
        }
        if attack_contact(ctx, &u, &enemy) {
            ctx.db.tower_travel().unit_id().delete(u.id);
            if ctx.db.unit_order().unit_id().find(u.id).is_some() {
                stop_unit(ctx, u.clone());
            }
            if is_archer(ctx, &u) {
                launch_arrow(ctx, &u, &BowTarget::Unit(enemy.clone()));
                combat_pose(ctx, &u);
                continue;
            }
            let dps = ctx
                .db
                .unit_profile()
                .unit_id()
                .find(u.id)
                .and_then(|p| map::units::UnitKind::from_name(&p.kind))
                .map(|k| {
                    tech::damage(
                        if k.is_support() { 0. } else { k.melee_dps() },
                        technology(ctx, u.owner).weapons,
                    )
                })
                .unwrap_or(0.);
            *damage.entry(enemy.id).or_default() += dps * melee::SCAN_SECONDS;
        } else if ctx.db.tower_travel().unit_id().find(u.id).is_none()
            && (ctx.db.unit_order().unit_id().find(u.id).is_none()
                || map::distance((u.target_x, u.target_z), (enemy.x, enemy.z)) > melee::MELEE_RANGE)
        {
            approach_enemy(ctx, &mut u, &enemy);
        }
        combat_pose(ctx, &u);
    }
    // Resolve every strike from the same tick before removing casualties.
    for (id, damage) in damage {
        if let Some(mut u) = ctx.db.army_unit().id().find(id) {
            if damage > 0. {
                alert_attack(ctx, u.game_id, u.owner, u.id, false, u.x, u.z);
            }
            u.health = (u.health - damage).max(0.);
            if u.health <= 0. {
                kill_combat_unit(ctx, id);
            } else {
                ctx.db.army_unit().id().update(u);
            }
        }
    }
    if ctx.db.unit_engagement().iter().next().is_none()
        && ctx.db.ranged_building_order().iter().next().is_none()
        && ctx.db.arrow_projectile().iter().next().is_none()
        && !hostile_pair(&combatants(ctx))
    {
        ctx.db
            .combat_tick()
            .scheduled_id()
            .delete(tick.scheduled_id);
    }
    Ok(())
}
