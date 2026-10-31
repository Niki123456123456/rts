use map::ranged as bow;

#[derive(Clone)]
#[spacetimedb::table(name = arrow_projectile, public)]
pub struct ArrowProjectile {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub shooter: u64,
    pub owner: spacetimedb::ConnectionId,
    pub target_id: u64,
    pub building: bool,
    pub start_x: f32,
    pub start_y: f32,
    pub start_z: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub end_z: f32,
    pub age: f32,
    pub flight_seconds: f32,
}
#[spacetimedb::table(name = archer_cooldown)]
pub struct ArcherCooldown {
    #[primary_key]
    pub unit_id: u64,
    pub last_fire_micros: i64,
}
#[derive(Clone)]
#[spacetimedb::table(name = ranged_building_order)]
pub struct RangedBuildingOrder {
    #[primary_key]
    pub unit_id: u64,
    pub building_id: u64,
}
#[spacetimedb::table(name=projectile_style,public)]
pub struct ProjectileStyle {
    #[primary_key]
    pub projectile_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub kind: String,
}
fn combat_kind(ctx: &ReducerContext, u: &ArmyUnit) -> map::units::UnitKind {
    ctx.db
        .unit_profile()
        .unit_id()
        .find(u.id)
        .and_then(|p| map::units::UnitKind::from_name(&p.kind))
        .unwrap_or(map::units::UnitKind::ArmedPeasant)
}
fn is_archer(ctx: &ReducerContext, u: &ArmyUnit) -> bool {
    ctx.db
        .unit_profile()
        .unit_id()
        .find(u.id)
        .is_some_and(|p| map::units::UnitKind::from_name(&p.kind).is_some_and(|k| k.is_ranged()))
}
enum BowTarget {
    Unit(ArmyUnit),
    Building(Building),
}
impl BowTarget {
    fn aim(&self, u: &ArmyUnit, ctx: &ReducerContext) -> bow::Position {
        match self {
            Self::Unit(v) => (v.x, v.y + 0.15, v.z),
            Self::Building(b) => {
                let p = building_shape(ctx, b).closest((u.x, u.z));
                (p.0, b.y + 1.2, p.1)
            }
        }
    }
    fn center(&self) -> map::Point {
        match self {
            Self::Unit(u) => (u.x, u.z),
            Self::Building(b) => (b.x, b.z),
        }
    }
    fn ignored(&self) -> Option<u64> {
        match self {
            Self::Building(b) => Some(b.id),
            _ => None,
        }
    }
}
fn shot_blocks(ctx: &ReducerContext, game: u64) -> Vec<bow::ShotBlock> {
    let mut result = Vec::new();
    let structures = fort_pieces(ctx, game);
    for j in fort::joins(&structures) {
        result.push(bow::ShotBlock {
            id: j.a,
            shape: j.shape,
            base: j.base,
            top: j.top,
        });
    }
    for b in ctx.db.building().game_id().filter(game) {
        let Some(kind) = map::BuildingKind::from_name(&b.kind) else {
            continue;
        };
        if kind.is_road() {
            continue;
        }
        if kind == map::BuildingKind::Aqueduct {
            for shape in kind.ground_solids(b.x, b.z, rotation(ctx, b.id)) {
                result.push(bow::ShotBlock {
                    id: b.id,
                    shape,
                    base: b.y,
                    top: b.y + 4.3,
                });
            }
            result.push(bow::ShotBlock {
                id: b.id,
                shape: fort::rect((b.x, b.z), rotation(ctx, b.id), (0., 0.), 7., 0.7),
                base: b.y + 5.,
                top: b.y + 6.4,
            });
            for c in [-4., 0., 4.] {
                for i in 0..18 {
                    let a = std::f32::consts::PI * (i as f32 + 0.5) / 18.;
                    let h = 3.3 + 1.7 * a.sin();
                    result.push(bow::ShotBlock {
                        id: b.id,
                        shape: fort::rect(
                            (b.x, b.z),
                            rotation(ctx, b.id),
                            (c + 1.7 * a.cos(), 0.),
                            0.22,
                            0.3,
                        ),
                        base: b.y + h - 0.25,
                        top: b.y + h + 0.25,
                    });
                }
            }
            continue;
        }
        let height = kind
            .roof_profile()
            .map(|p| p.height)
            .unwrap_or_else(|| kind.height());
        let shapes = if kind.is_gate() {
            fort::gate_solids(
                kind,
                (b.x, b.z),
                rotation(ctx, b.id),
                gate_is_open(ctx, b.id),
            )
        } else {
            vec![building_shape(ctx, &b)]
        };
        for shape in shapes {
            result.push(bow::ShotBlock {
                id: b.id,
                shape,
                base: b.y,
                top: b.y + height,
            });
        }
        if kind.is_crenellated() || kind == map::BuildingKind::GatehouseSmall {
            let edges = if kind.is_wall() {
                fort::exposed_edges(fort_piece(ctx, &b), &structures)
            } else {
                vec![(1., 0.), (-1., 0.), (0., 1.), (0., -1.)]
            };
            for block in fort::parapets(
                kind,
                (b.x, b.z),
                if kind.is_wall() {
                    0
                } else {
                    rotation(ctx, b.id)
                },
                &edges,
            ) {
                result.push(bow::ShotBlock {
                    id: b.id,
                    shape: block.shape,
                    base: b.y + block.bottom,
                    top: b.y + block.top,
                });
            }
        }
    }
    for u in ctx
        .db
        .army_unit()
        .game_id()
        .filter(game)
        .filter(|u| combat_kind(ctx, u) == map::units::UnitKind::Pluteus)
    {
        let p = ctx.db.unit_profile().unit_id().find(u.id).unwrap();
        let (hx, hz) = if p.facing_x.abs() > p.facing_z.abs() {
            (0.18, 1.3)
        } else {
            (1.3, 0.18)
        };
        result.push(bow::ShotBlock {
            id: u64::MAX - u.id,
            shape: Obstacle {
                x: u.x,
                z: u.z,
                half_x: hx,
                half_z: hz,
            },
            base: u.y - 0.75,
            top: u.y + 2.3,
        });
    }
    result
}
fn arrow_origin(ctx: &ReducerContext, u: &ArmyUnit, end: bow::Position) -> bow::Position {
    let kind = combat_kind(ctx, u);
    let height = match kind {
        map::units::UnitKind::Onager => 2.75,
        map::units::UnitKind::Scorpion => 1.3,
        map::units::UnitKind::Ballista | map::units::UnitKind::StoneBallista => 0.85,
        _ => 0.56,
    };
    let forward = if kind.is_siege() { 0.9 } else { 0.45 };
    let length = map::distance((u.x, u.z), (end.0, end.2)).max(0.001);
    let dx = (end.0 - u.x) / length;
    let dz = (end.2 - u.z) / length;
    (
        u.x + dx * forward + dz * 0.13,
        u.y + height,
        u.z + dz * forward - dx * 0.13,
    )
}
fn bow_contact(ctx: &ReducerContext, u: &ArmyUnit, target: &BowTarget) -> bool {
    let end = target.aim(u, ctx);
    let range = combat_kind(ctx, u).ranged_range(u.y);
    map::distance((u.x, u.z), (end.0, end.2)) <= range
        && bow::projectile_clear(
            arrow_origin(ctx, u, end),
            end,
            &shot_blocks(ctx, u.game_id),
            target.ignored(),
            |p| terrain_height(p.0, p.1),
            combat_kind(ctx, u).name(),
        )
}
fn attack_contact(ctx: &ReducerContext, u: &ArmyUnit, v: &ArmyUnit) -> bool {
    if is_archer(ctx, u) {
        bow_contact(ctx, u, &BowTarget::Unit(v.clone()))
    } else {
        melee_contact(ctx, u, v)
    }
}
fn launch_arrow(ctx: &ReducerContext, u: &ArmyUnit, target: &BowTarget) {
    let kind = combat_kind(ctx, u);
    let now = ctx.timestamp.to_micros_since_unix_epoch();
    if ctx
        .db
        .archer_cooldown()
        .unit_id()
        .find(u.id)
        .is_some_and(|c| now - c.last_fire_micros < (kind.shot_interval() * 1_000_000.) as i64)
    {
        return;
    }
    let end = target.aim(u, ctx);
    let start = arrow_origin(ctx, u, end);
    ctx.db.archer_cooldown().unit_id().delete(u.id);
    ctx.db.archer_cooldown().insert(ArcherCooldown {
        unit_id: u.id,
        last_fire_micros: now,
    });
    let projectile = ctx.db.arrow_projectile().insert(ArrowProjectile {
        id: 0,
        game_id: u.game_id,
        shooter: u.id,
        owner: u.owner,
        target_id: match target {
            BowTarget::Unit(v) => v.id,
            BowTarget::Building(b) => b.id,
        },
        building: matches!(target, BowTarget::Building(_)),
        start_x: start.0,
        start_y: start.1,
        start_z: start.2,
        end_x: end.0,
        end_y: end.1,
        end_z: end.2,
        age: 0.,
        flight_seconds: (map::distance((start.0, start.2), (end.0, end.2)).hypot(start.1 - end.1)
            / if kind == map::units::UnitKind::Onager {
                14.
            } else {
                bow::ARROW_SPEED
            })
        .max(0.15),
    });
    if kind.is_siege() {
        ctx.db.projectile_style().insert(ProjectileStyle {
            projectile_id: projectile.id,
            game_id: u.game_id,
            kind: kind.name().into(),
        });
    }
}
fn step_arrows(ctx: &ReducerContext) {
    for mut arrow in ctx.db.arrow_projectile().iter().collect::<Vec<_>>() {
        arrow.age += melee::SCAN_SECONDS;
        let style = ctx.db.projectile_style().projectile_id().find(arrow.id);
        let kind = style
            .as_ref()
            .and_then(|s| map::units::UnitKind::from_name(&s.kind))
            .unwrap_or(map::units::UnitKind::Archer);
        let target = if arrow.building {
            ctx.db
                .building()
                .id()
                .find(arrow.target_id)
                .filter(|b| b.game_id == arrow.game_id && b.owner != arrow.owner)
                .map(BowTarget::Building)
        } else {
            ctx.db
                .army_unit()
                .id()
                .find(arrow.target_id)
                .filter(|v| {
                    v.game_id == arrow.game_id
                        && v.owner != arrow.owner
                        && ctx.db.unit_profile().unit_id().find(v.id).is_some()
                })
                .map(BowTarget::Unit)
        };
        let Some(target) = target else {
            ctx.db.arrow_projectile().id().delete(arrow.id);
            ctx.db.projectile_style().projectile_id().delete(arrow.id);
            continue;
        };
        if let BowTarget::Unit(v) = &target {
            arrow.end_x = v.x;
            arrow.end_y = v.y + 0.15;
            arrow.end_z = v.z;
        }
        if arrow.age < arrow.flight_seconds {
            ctx.db.arrow_projectile().id().update(arrow);
            continue;
        }
        ctx.db.arrow_projectile().id().delete(arrow.id);
        ctx.db.projectile_style().projectile_id().delete(arrow.id);
        if !bow::projectile_clear(
            (arrow.start_x, arrow.start_y, arrow.start_z),
            (arrow.end_x, arrow.end_y, arrow.end_z),
            &shot_blocks(ctx, arrow.game_id),
            target.ignored(),
            |p| terrain_height(p.0, p.1),
            kind.name(),
        ) {
            continue;
        }
        if matches!(
            kind,
            map::units::UnitKind::Onager | map::units::UnitKind::StoneBallista
        ) {
            for mut victim in ctx
                .db
                .army_unit()
                .game_id()
                .filter(arrow.game_id)
                .filter(|v| {
                    v.owner != arrow.owner
                        && v.id != arrow.target_id
                        && map::distance((v.x, v.z), (arrow.end_x, arrow.end_z)) < 2.
                })
                .collect::<Vec<_>>()
            {
                alert_attack(
                    ctx,
                    victim.game_id,
                    victim.owner,
                    victim.id,
                    false,
                    victim.x,
                    victim.z,
                );
                victim.health = (victim.health - 0.25).max(0.);
                if victim.health <= 0. {
                    kill_combat_unit(ctx, victim.id);
                } else {
                    ctx.db.army_unit().id().update(victim);
                }
            }
        }
        match target {
            BowTarget::Unit(mut v) => {
                alert_attack(ctx, v.game_id, v.owner, v.id, false, v.x, v.z);
                v.health = (v.health
                    - tech::damage(
                        kind.shot_damage(false),
                        technology(ctx, arrow.owner).weapons,
                    ) * if combat_kind(ctx, &v) == map::units::UnitKind::CoveredRam {
                        0.55
                    } else {
                        1.
                    })
                .max(0.);
                if v.health <= 0. {
                    kill_combat_unit(ctx, v.id);
                } else {
                    ctx.db.army_unit().id().update(v);
                }
            }
            BowTarget::Building(mut b) => {
                alert_attack(ctx, b.game_id, b.owner, b.id, true, b.x, b.z);
                b.health = (b.health
                    - tech::damage(kind.shot_damage(true), technology(ctx, arrow.owner).weapons))
                .max(0.);
                if b.health <= 0. {
                    delete_building(ctx, b.id);
                    replan(ctx, b.game_id);
                } else {
                    ctx.db.building().id().update(b);
                }
            }
        }
    }
}
fn approach_bow_target(ctx: &ReducerContext, u: &mut ArmyUnit, target: &BowTarget) -> bool {
    if bow_contact(ctx, u, target) {
        stop_unit(ctx, u.clone());
        return true;
    }
    if ctx.db.tower_travel().unit_id().find(u.id).is_some() {
        return true;
    }
    // An explicit attack may move along the current roof network to a firing slot.
    if let Some(surface) = combat_surface(ctx, u) {
        let pieces = combat_roof_pieces(ctx, u.game_id);
        let mut slots = map::walls::surface_component(surface.id, &pieces)
            .into_iter()
            .flat_map(|p| (0..p.roof().capacity()).map(move |s| (p, s)))
            .collect::<Vec<_>>();
        slots.sort_by(|(a, i), (b, j)| {
            map::distance(a.roof().slot(*i), (u.x, u.z))
                .total_cmp(&map::distance(b.roof().slot(*j), (u.x, u.z)))
        });
        for (p, slot) in slots {
            let mut candidate = u.clone();
            let q = p.roof().slot(slot);
            candidate.x = q.0;
            candidate.z = q.1;
            candidate.y = p.roof().roof_y() + roof::UNIT_CENTER_HEIGHT;
            if !slot_reserved(ctx, p.id, slot as u8, u.id)
                && bow_contact(ctx, &candidate, target)
                && combat_surface_move(ctx, u, p, slot)
            {
                return true;
            }
        }
    }
    let Ok(start) = depart_tower(ctx, u) else {
        return false;
    };
    let center = target.center();
    let obs = obstacles(ctx, u.game_id);
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|v| v.id != u.id)
        .flat_map(|v| ground_reservations(ctx, &v))
        .collect::<Vec<_>>();
    let extra = match target {
        BowTarget::Building(b) => {
            let s = building_shape(ctx, b);
            s.half_x.max(s.half_z)
        }
        _ => 0.,
    };
    let mut candidates = Vec::new();
    for radius in [
        bow::BASE_RANGE - 0.25,
        bow::BASE_RANGE - 1.5,
        10.,
        7.,
        4.,
        2.,
    ] {
        for i in 0..32 {
            let a = i as f32 * std::f32::consts::TAU / 32.;
            let p = (
                center.0 + a.cos() * (radius + extra),
                center.1 + a.sin() * (radius + extra),
            );
            if !map::free(p, &obs)
                || peers
                    .iter()
                    .any(|q| map::distance(p, *q) < map::UNIT_SPACING)
            {
                continue;
            }
            let mut v = u.clone();
            v.x = p.0;
            v.z = p.1;
            v.y = terrain_height(p.0, p.1) + 0.75;
            if bow_contact(ctx, &v, target) {
                candidates.push(p);
            }
        }
    }
    candidates.sort_by(|a, b| map::distance(*a, start).total_cmp(&map::distance(*b, start)));
    for p in candidates {
        if let Some(path) = map::shortest_path(start, p, &obs) {
            u.target_x = p.0;
            u.target_z = p.1;
            save_order(ctx, u, None, path);
            ctx.db.army_unit().id().update(u.clone());
            start_tick(ctx);
            return true;
        }
    }
    false
}
fn ranged_building_facing(ctx: &ReducerContext, u: &ArmyUnit) -> Option<(f32, f32)> {
    if !is_archer(ctx, u) {
        return None;
    }
    let order = ctx.db.ranged_building_order().unit_id().find(u.id)?;
    let b = ctx.db.building().id().find(order.building_id)?;
    bow_contact(ctx, u, &BowTarget::Building(b.clone())).then_some((b.x - u.x, b.z - u.z))
}
fn step_ranged_buildings(ctx: &ReducerContext) {
    for order in ctx.db.ranged_building_order().iter().collect::<Vec<_>>() {
        let Some(mut u) = ctx.db.army_unit().id().find(order.unit_id) else {
            ctx.db
                .ranged_building_order()
                .unit_id()
                .delete(order.unit_id);
            continue;
        };
        let Some(b) = ctx
            .db
            .building()
            .id()
            .find(order.building_id)
            .filter(|b| b.game_id == u.game_id && b.owner != u.owner)
        else {
            ctx.db.ranged_building_order().unit_id().delete(u.id);
            stop_unit(ctx, u.clone());
            combat_pose(ctx, &u);
            continue;
        };
        let target = BowTarget::Building(b);
        if bow_contact(ctx, &u, &target) {
            ctx.db.tower_travel().unit_id().delete(u.id);
            stop_unit(ctx, u.clone());
            launch_arrow(ctx, &u, &target);
        } else if ctx.db.unit_order().unit_id().find(u.id).is_none() {
            approach_bow_target(ctx, &mut u, &target);
        }
        combat_pose(ctx, &u);
    }
}
