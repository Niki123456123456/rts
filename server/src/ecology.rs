use map::ecology as nature;
#[derive(Clone)]
#[spacetimedb::table(name=resource_node,public)]
pub struct ResourceNode {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub site: u32,
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub remaining: u32,
    pub capacity: u32,
    /// Trees: 0 standing, 1 falling, 2 logs being chopped, 3 exhausted.
    pub phase: u8,
    pub phase_ticks: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=wildlife,public)]
pub struct Wildlife {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub species: u8,
    pub group_id: u32,
    pub x: f32,
    pub z: f32,
    pub home_x: f32,
    pub home_z: f32,
    pub health: f32,
    pub state: u8,
    pub meat: u32,
    pub dead_ticks: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=ox_cart,public)]
pub struct OxCart {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub source_building: Option<u64>,
}
#[derive(Clone)]
#[spacetimedb::table(name=fish_stock,public)]
pub struct FishStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub amount: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=ecology_state)]
pub struct EcologyState {
    #[primary_key]
    pub game_id: u64,
    pub seconds: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=wildlife_order)]
pub struct WildlifeOrder {
    #[primary_key]
    pub animal_id: u64,
    pub path: Vec<f32>,
}
#[derive(Clone)]
#[spacetimedb::table(name=hunt_order)]
pub struct HuntOrder {
    #[primary_key]
    pub unit_id: u64,
    pub animal_id: u64,
}
#[spacetimedb::table(name=ecology_tick,scheduled(update_ecology))]
pub struct EcologyTick {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub scheduled_at: ScheduleAt,
}
fn start_ecology(ctx: &ReducerContext) {
    if ctx.db.ecology_tick().iter().next().is_none() {
        ctx.db.ecology_tick().insert(EcologyTick {
            id: 0,
            scheduled_at: Duration::from_secs(1).into(),
        });
    }
}
fn ensure_ecology(ctx: &ReducerContext, game_id: u64) {
    start_ecology(ctx);
    if ctx.db.ecology_state().game_id().find(game_id).is_some() {
        return;
    }
    ctx.db.ecology_state().insert(EcologyState {
        game_id,
        seconds: 0,
    });
    let clearings = ctx
        .db
        .terrain_clearing()
        .game_id()
        .filter(game_id)
        .collect::<Vec<_>>();
    for t in nature::trees() {
        let cleared = clearings.iter().any(|c| {
            Obstacle {
                x: c.x,
                z: c.z,
                half_x: c.half_x,
                half_z: c.half_z,
            }
            .contains((t.x, t.z), 2.5 * t.scale)
        });
        ctx.db.resource_node().insert(ResourceNode {
            id: 0,
            game_id,
            site: t.site,
            kind: 0,
            x: t.x,
            z: t.z,
            remaining: if cleared { 0 } else { nature::TREE_WOOD },
            capacity: nature::TREE_WOOD,
            phase: if cleared { 3 } else { 0 },
            phase_ticks: 0,
        });
    }
    for (kind, sites, _capacity, offset) in [
        (
            1,
            map::terrain::STONE.as_slice(),
            nature::STONE_AMOUNT,
            10000,
        ),
        (2, map::terrain::IRON.as_slice(), nature::IRON_AMOUNT, 20000),
    ] {
        for (i, p) in sites.iter().enumerate() {
            let capacity = nature::deposit_capacity(kind, i);
            ctx.db.resource_node().insert(ResourceNode {
                id: 0,
                game_id,
                site: offset + i as u32,
                kind,
                x: p.0,
                z: p.1,
                remaining: capacity,
                capacity,
                phase: 0,
                phase_ticks: 0,
            });
        }
    }
    for i in 0..nature::FISH_Z.len() {
        let p = nature::fish_center(i);
        ctx.db.resource_node().insert(ResourceNode {
            id: 0,
            game_id,
            site: 30000 + i as u32,
            kind: 7,
            x: p.0,
            z: p.1,
            remaining: nature::FISH_CAPACITY,
            capacity: nature::FISH_CAPACITY,
            phase: 0,
            phase_ticks: 0,
        });
    }
    let obs = obstacles(ctx, game_id);
    for (group, (species, count, x, z)) in nature::HERDS.into_iter().enumerate() {
        for i in 0..count {
            let a = i as f32 * 2.4;
            let p = (
                x + a.cos() * (2. + i as f32 * 0.8),
                z + a.sin() * (2. + i as f32 * 0.8),
            );
            if !map::free(p, &obs) {
                continue;
            }
            ctx.db.wildlife().insert(Wildlife {
                id: 0,
                game_id,
                species,
                group_id: group as u32,
                x: p.0,
                z: p.1,
                home_x: x,
                home_z: z,
                health: nature::animal_health(species),
                state: 0,
                meat: 0,
                dead_ticks: 0,
            });
        }
    }
}
fn register_ecology_building(ctx: &ReducerContext, b: &Building) {
    if b.kind == "ox_tether" && ctx.db.ox_cart().building_id().find(b.id).is_none() {
        ctx.db.ox_cart().insert(OxCart {
            building_id: b.id,
            game_id: b.game_id,
            source_building: None,
        });
    }
}
fn clear_resource_trees(ctx: &ReducerContext, game: u64, shape: Obstacle) {
    let ids = ctx
        .db
        .resource_node()
        .game_id()
        .filter(game)
        .filter(|n| n.kind == 0 && n.remaining > 0 && shape.contains((n.x, n.z), 3.5))
        .collect::<Vec<_>>();
    for mut n in ids {
        n.remaining = 0;
        n.phase = 3;
        ctx.db.resource_node().id().update(n);
    }
}
fn cart_for(ctx: &ReducerContext, source: u64) -> Option<OxCart> {
    ctx.db
        .ox_cart()
        .iter()
        .find(|c| c.source_building == Some(source))
}
fn unit_has_cart(ctx: &ReducerContext, id: u64) -> bool {
    ctx.db
        .worker()
        .unit_id()
        .find(id)
        .is_some_and(|w| cart_for(ctx, w.source_building).is_some())
}
fn worker_obstacles(ctx: &ReducerContext, u: &ArmyUnit) -> Vec<Obstacle> {
    let mut obs = obstacles(ctx, u.game_id);
    if unit_has_cart(ctx, u.id) {
        for o in &mut obs {
            o.half_x += 1.25;
            o.half_z += 1.25;
        }
    }
    obs
}
fn work_range(ctx: &ReducerContext, id: u64) -> f32 {
    if unit_has_cart(ctx, id) {
        2.5
    } else {
        1.2
    }
}
#[spacetimedb::reducer]
pub fn assign_ox_cart(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let b = production_building(ctx, building_id)?;
    if !matches!(b.kind.as_str(), "saw_pit" | "quarry" | "iron_mine") {
        return Err("Karren können Holz, Stein und Eisen transportieren.".into());
    }
    if cart_for(ctx, b.id).is_some() {
        return Ok(());
    }
    reconcile_carts(ctx);
    if cart_for(ctx, b.id).is_some() {
        Ok(())
    } else {
        Err("Baue einen freien Ochsenplatz im Umkreis von 80 m.".into())
    }
}

fn attach_cart(ctx: &ReducerContext, mut cart: OxCart, b: &Building) -> Result<(), String> {
    cart.source_building = Some(b.id);
    ctx.db.ox_cart().building_id().update(cart);
    for w in ctx.db.worker().source_building().filter(b.id) {
        if let Some(mut u) = ctx.db.army_unit().id().find(w.unit_id) {
            let obs = worker_obstacles(ctx, &u);
            let actual = obstacles(ctx, u.game_id);
            if !map::free((u.x, u.z), &obs) {
                let peers = ctx
                    .db
                    .army_unit()
                    .game_id()
                    .filter(u.game_id)
                    .filter(|v| v.id != u.id && on_ground(ctx, &v))
                    .map(|v| (v.x, v.z))
                    .collect::<Vec<_>>();
                let point = free_destination((u.x, u.z), &peers, &obs)
                    .filter(|p| map::segment_clear((u.x, u.z), *p, &actual))
                    .ok_or("Kein Platz für den Karren am Arbeiter.")?;
                u.x = point.0;
                u.z = point.1;
                u.y = ground_height(ctx, u.game_id, u.x, u.z) + 0.75;
            }
            stop_unit(ctx, u);
        }
    }
    Ok(())
}
#[spacetimedb::reducer]
pub fn release_ox_cart(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let b = production_building(ctx, building_id)?;
    if ctx
        .db
        .worker()
        .source_building()
        .filter(b.id)
        .any(|w| w.cargo > 0)
    {
        return Err("Warte, bis die aktuelle Ladung abgeliefert ist.".into());
    }
    if let Some(mut c) = cart_for(ctx, b.id) {
        c.source_building = None;
        ctx.db.ox_cart().building_id().update(c);
    }
    Ok(())
}
fn remove_ecology_building(ctx: &ReducerContext, id: u64) {
    ctx.db.fish_stock().building_id().delete(id);
    if let Some(cart) = ctx.db.ox_cart().building_id().find(id) {
        if let Some(source) = cart.source_building {
            for mut w in ctx
                .db
                .worker()
                .source_building()
                .filter(source)
                .collect::<Vec<_>>()
            {
                if w.cargo > 0 {
                    if let Some(mut p) = ctx.db.producer().building_id().find(source) {
                        p.buffer = p.buffer.saturating_add(w.cargo);
                        ctx.db.producer().building_id().update(p);
                    }
                    w.cargo = 0;
                    w.warehouse = None;
                    ctx.db.worker().unit_id().update(w);
                }
            }
        }
    }
    if ctx.db.ox_cart().building_id().delete(id) {
        for w in ctx
            .db
            .worker()
            .iter()
            .filter(|w| ctx.db.building().id().find(w.source_building).is_some())
        {
            if let Some(u) = ctx.db.army_unit().id().find(w.unit_id) {
                stop_unit(ctx, u);
            }
        }
    }
    if let Some(mut c) = cart_for(ctx, id) {
        c.source_building = None;
        ctx.db.ox_cart().building_id().update(c);
    }
}
fn node_for(ctx: &ReducerContext, b: &Building) -> Option<ResourceNode> {
    let kind = match b.kind.as_str() {
        "saw_pit" => 0,
        "quarry" => 1,
        "iron_mine" => 2,
        "fishpond" => 7,
        _ => return None,
    };
    let site = if kind == 7 {
        nature::fish_school((b.x, b.z)).map(|i| 30000 + i as u32)
    } else {
        None
    };
    ctx.db
        .resource_node()
        .game_id()
        .filter(b.game_id)
        .filter(|n| {
            n.kind == kind
                && (kind == 7 || n.remaining > 0)
                && site.is_none_or(|s| s == n.site)
                && map::distance((b.x, b.z), (n.x, n.z)) < if kind == 0 { 40. } else { 35. }
        })
        .min_by(|a, c| {
            map::distance((b.x, b.z), (a.x, a.z)).total_cmp(&map::distance((b.x, b.z), (c.x, c.z)))
        })
}
fn hunt_target(ctx: &ReducerContext, b: &Building) -> Option<Wildlife> {
    ctx.db
        .wildlife()
        .game_id()
        .filter(b.game_id)
        .filter(|a| (a.health > 0. || a.meat > 0) && map::distance((b.x, b.z), (a.x, a.z)) < 80.)
        .min_by(|a, c| {
            (map::distance((b.x, b.z), (a.x, a.z)) + if a.health > 0. { 100. } else { 0. })
                .total_cmp(
                    &(map::distance((b.x, b.z), (c.x, c.z))
                        + if c.health > 0. { 100. } else { 0. }),
                )
        })
}
fn work_site(ctx: &ReducerContext, b: &Building, u: &ArmyUnit) -> Option<map::Point> {
    let obs = worker_obstacles(ctx, u);
    if b.kind == "fishpond" {
        return fishing_bank(b, (u.x, u.z), &obs);
    }
    let target = if b.kind == "hunters_post" {
        let a = hunt_target(ctx, b)?;
        if a.health > 0.
            && map::distance((u.x, u.z), (a.x, a.z)) < 12.
            && map::segment_clear((u.x, u.z), (a.x, a.z), &obs)
        {
            return Some((u.x, u.z));
        }
        Obstacle {
            x: a.x,
            z: a.z,
            half_x: if a.health > 0. { 8. } else { 0.4 },
            half_z: if a.health > 0. { 8. } else { 0.4 },
        }
    } else if b.kind == "saw_pit" {
        let n = node_for(ctx, b)?;
        Obstacle {
            x: n.x,
            z: n.z,
            half_x: 0.35,
            half_z: 0.35,
        }
    } else {
        return access_route(ctx, u, b).map(|p| p.0);
    };
    map::attack_positions(target)
        .into_iter()
        .filter(|p| p.ring == 0)
        .map(|p| p.point)
        .filter(|p| map::free(*p, &obs))
        .min_by(|a, c| map::distance((u.x, u.z), *a).total_cmp(&map::distance((u.x, u.z), *c)))
}
fn at_work_site(ctx: &ReducerContext, b: &Building, u: &ArmyUnit) -> bool {
    if b.kind == "fishpond" {
        return map::distance((b.x, b.z), (u.x, u.z)) <= 43. && river_bank((u.x, u.z));
    }
    if b.kind == "saw_pit" {
        return node_for(ctx, b).is_some_and(|n| map::distance((u.x, u.z), (n.x, n.z)) < 1.9);
    }
    if b.kind == "hunters_post" {
        return hunt_target(ctx, b).is_some_and(|a| {
            map::distance((u.x, u.z), (a.x, a.z)) < if a.health > 0. { 12. } else { 1.9 }
                && map::segment_clear((u.x, u.z), (a.x, a.z), &obstacles(ctx, b.game_id))
        });
    }
    map::distance((u.x, u.z), building_shape(ctx, b).closest((u.x, u.z))) <= work_range(ctx, u.id)
}
fn route_to_work(ctx: &ReducerContext, b: &Building, u: &mut ArmyUnit) {
    let Some(goal) = work_site(ctx, b, u) else {
        return;
    };
    if let Some(path) = map::shortest_path((u.x, u.z), goal, &worker_obstacles(ctx, u)) {
        u.target_x = goal.0;
        u.target_z = goal.1;
        save_order(ctx, u, None, path);
        ctx.db.army_unit().id().update(u.clone());
        start_tick(ctx);
    }
}
fn animal_damaged(ctx: &ReducerContext, mut a: Wildlife, damage: f32) {
    if a.health <= 0. {
        return;
    }
    a.health = (a.health - damage).max(0.);
    if a.health == 0. {
        a.state = 3;
        a.meat = nature::animal_meat(a.species);
        ctx.db.wildlife_order().animal_id().delete(a.id);
    } else if nature::predator(a.species) {
        a.state = 2;
    }
    ctx.db.wildlife().id().update(a);
}
fn take_harvest(ctx: &ReducerContext, b: &Building) -> u32 {
    if b.kind == "vineyard" || b.kind == "olive_grove" {
        return 2;
    }
    if b.kind == "vintners_workshop" {
        return u32::from(consume_intermediate(ctx, b, Commodity::Grapes, 2));
    }
    if b.kind == "olive_press" {
        return u32::from(consume_intermediate(ctx, b, Commodity::Olives, 2));
    }
    if b.kind == "windmill" {
        return u32::from(consume_intermediate(ctx, b, Commodity::Wheat, 1));
    }
    if b.kind == "bakery" {
        return if consume_intermediate(ctx, b, Commodity::Flour, 1) {
            2
        } else {
            0
        };
    }
    if b.kind == "wheat_farm" {
        return 2;
    }

    if b.kind == "hunters_post" {
        let Some(mut a) = hunt_target(ctx, b) else {
            return 0;
        };
        if a.health > 0. {
            animal_damaged(ctx, a, 0.65);
            return 0;
        }
        let (left, take) = nature::consume(a.meat, 1);
        a.meat = left;
        ctx.db.wildlife().id().update(a);
        return take;
    }
    if !matches!(
        b.kind.as_str(),
        "saw_pit" | "quarry" | "iron_mine" | "fishpond"
    ) {
        return 1;
    }
    let Some(mut n) = node_for(ctx, b) else {
        return 0;
    };
    if n.kind == 0 {
        if n.phase == 0 {
            n.phase = 1;
            n.phase_ticks = 0;
            ctx.db.resource_node().id().update(n);
            return 0;
        }
        if n.phase == 1 {
            return 0;
        }
    }
    let (left, take) = nature::consume(n.remaining, 1);
    n.remaining = left;
    if left == 0 && n.kind != 7 {
        n.phase = 3;
    }
    ctx.db.resource_node().id().update(n);
    take
}
fn source_exhausted(ctx: &ReducerContext, b: &Building) -> bool {
    match b.kind.as_str() {
        "saw_pit" | "quarry" | "iron_mine" => node_for(ctx, b).is_none(),
        "hunters_post" => hunt_target(ctx, b).is_none(),
        _ => false,
    }
}
#[spacetimedb::reducer]
pub fn hunt_animal(ctx: &ReducerContext, unit_ids: Vec<u64>, animal_id: u64) -> Result<(), String> {
    let p = participant(ctx)?;
    let a = ctx
        .db
        .wildlife()
        .id()
        .find(animal_id)
        .filter(|a| a.game_id == p.game_id && a.health > 0.)
        .ok_or("Dieses Wildtier ist nicht mehr verfügbar.")?;
    for u in owned_units(ctx, &p, unit_ids)? {
        ctx.db.hunt_order().unit_id().delete(u.id);
        ctx.db.hunt_order().insert(HuntOrder {
            unit_id: u.id,
            animal_id: a.id,
        });
    }
    start_ecology(ctx);
    Ok(())
}
fn move_animal_toward(
    _ctx: &ReducerContext,
    a: &mut Wildlife,
    goal: map::Point,
    obs: &[Obstacle],
    speed: f32,
) {
    let old = (a.x, a.z);
    let path = map::shortest_path(old, goal, obs);
    if let Some(path) = path {
        let mut left = speed;
        let mut point = old;
        for next in path {
            let d = map::distance(point, next);
            if d <= left {
                point = next;
                left -= d;
            } else {
                point = (
                    point.0 + (next.0 - point.0) * left / d,
                    point.1 + (next.1 - point.1) * left / d,
                );
                break;
            }
        }
        a.x = point.0;
        a.z = point.1;
    }
}
#[spacetimedb::reducer]
pub fn update_ecology(ctx: &ReducerContext, tick: EcologyTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Ecology is server controlled.".into());
    }
    let active = ctx
        .db
        .participant()
        .iter()
        .map(|p| p.game_id)
        .collect::<std::collections::HashSet<_>>();
    if active.is_empty() {
        ctx.db.ecology_tick().id().delete(tick.id);
        return Ok(());
    }
    for game in active {
        let Some(mut state) = ctx.db.ecology_state().game_id().find(game) else {
            continue;
        };
        state.seconds = state.seconds.saturating_add(1);
        for mut n in ctx
            .db
            .resource_node()
            .game_id()
            .filter(game)
            .filter(|n| n.kind == 7 || n.phase == 1)
            .collect::<Vec<_>>()
        {
            if n.phase == 1 {
                n.phase_ticks += 1;
                if n.phase_ticks >= 2 {
                    n.phase = 2;
                }
            }
            if n.kind == 7 {
                n.remaining = nature::regenerate_fish(n.remaining, state.seconds);
            }
            ctx.db.resource_node().id().update(n);
        }
        let obs = obstacles(ctx, game);
        balance_predators(ctx, game);
        upgrade_resource_sizes(ctx, game);
        let units = ctx
            .db
            .army_unit()
            .game_id()
            .filter(game)
            .filter(|u| ctx.db.participant().connection_id().find(u.owner).is_some())
            .collect::<Vec<_>>();
        // Explicit hunts and automatic defence use the same bounded damage and routes.
        for u in &units {
            if ctx
                .db
                .unit_profile()
                .unit_id()
                .find(u.id)
                .is_some_and(|p| p.kind == "lord")
                && ctx.db.lord_vitality().unit_id().find(u.id).is_none()
            {
                if u.health <= 1. {
                    let mut lord = u.clone();
                    lord.health *= 5.;
                    ctx.db.army_unit().id().update(lord);
                }
                ctx.db
                    .lord_vitality()
                    .insert(LordVitality { unit_id: u.id });
            }
            let requested = ctx
                .db
                .hunt_order()
                .unit_id()
                .find(u.id)
                .and_then(|h| ctx.db.wildlife().id().find(h.animal_id))
                .filter(|a| a.health > 0.);
            let defensive = (ctx.db.unit_profile().unit_id().find(u.id).is_some()
                || ctx.db.worker().unit_id().find(u.id).is_some())
            .then(|| {
                ctx.db
                    .wildlife()
                    .game_id()
                    .filter(game)
                    .filter(|a| {
                        a.health > 0. && a.state == 2 && map::distance((u.x, u.z), (a.x, a.z)) < 15.
                    })
                    .min_by(|a, b| {
                        map::distance((u.x, u.z), (a.x, a.z))
                            .total_cmp(&map::distance((u.x, u.z), (b.x, b.z)))
                    })
            })
            .flatten();
            if let Some(a) = requested.or(defensive) {
                let job = ctx
                    .db
                    .worker()
                    .unit_id()
                    .find(u.id)
                    .and_then(|w| ctx.db.building().id().find(w.source_building))
                    .map(|b| b.kind);
                let archer = is_archer(ctx, u) || job.as_deref() == Some("hunters_post");
                let defence = job.as_deref().map(nature::defence);
                let lord = ctx
                    .db
                    .unit_profile()
                    .unit_id()
                    .find(u.id)
                    .is_some_and(|p| p.kind == "lord");
                let range = defence.map_or_else(
                    || {
                        if archer {
                            map::ranged::bow_range(u.y)
                        } else {
                            2.4
                        }
                    },
                    |d| d.0,
                );
                let d = map::distance((u.x, u.z), (a.x, a.z));
                if d <= range
                    && (archer || on_ground(ctx, u))
                    && (if archer {
                        map::ranged::shot_clear(
                            (u.x, u.y + 0.56, u.z),
                            (a.x, terrain_height(a.x, a.z) + 0.6, a.z),
                            &shot_blocks(ctx, game),
                            None,
                            |p| terrain_height(p.0, p.1),
                        )
                    } else {
                        on_ground(ctx, u) && map::segment_clear((u.x, u.z), (a.x, a.z), &obs)
                    })
                {
                    animal_damaged(
                        ctx,
                        a,
                        defence.map_or(
                            if lord {
                                1.3
                            } else if archer {
                                0.7
                            } else {
                                0.65
                            },
                            |d| d.1,
                        ),
                    );
                } else if ctx.db.hunt_order().unit_id().find(u.id).is_some() && job.is_none() {
                    if let Ok(start) = depart_tower(ctx, u) {
                        let mut unit = u.clone();
                        if let Some(path) = map::shortest_path(start, (a.x, a.z), &obs) {
                            unit.target_x = a.x;
                            unit.target_z = a.z;
                            save_order(ctx, &unit, None, path);
                            ctx.db.army_unit().id().update(unit);
                            start_tick(ctx);
                        }
                    }
                }
            } else {
                ctx.db.hunt_order().unit_id().delete(u.id);
            }
        }
        let animals = ctx.db.wildlife().game_id().filter(game).collect::<Vec<_>>();
        let alerted = animals
            .iter()
            .filter(|a| {
                a.health > 0.
                    && nature::predator(a.species)
                    && units.iter().any(|u| {
                        on_ground(ctx, u)
                            && map::distance((u.x, u.z), (a.x, a.z))
                                < nature::aggression_radius(a.species)
                    })
            })
            .map(|a| a.group_id)
            .collect::<std::collections::HashSet<_>>();
        for mut a in animals {
            if a.health > 0. && nature::predator(a.species) && a.state == 2 {
                let defenders = ctx
                    .db
                    .building()
                    .game_id()
                    .filter(game)
                    .filter(|b| b.kind == "keep")
                    .filter_map(|b| {
                        let fire = tower_geometry(ctx, &b).world((0., 11.));
                        if map::distance((a.x, a.z), fire) < 8.
                            && map::segment_clear((a.x, a.z), fire, &obs)
                        {
                            ctx.db.keep_population().building_id().find(b.id).map(|p| {
                                p.arrived
                                    .saturating_sub(employed_peasants(ctx, b.owner))
                                    .min(6)
                            })
                        } else {
                            None
                        }
                    })
                    .sum::<u32>();
                if defenders > 0 {
                    animal_damaged(ctx, a.clone(), defenders as f32 * 0.14);
                    a = ctx.db.wildlife().id().find(a.id).unwrap();
                }
            }
            if a.health <= 0. {
                a.dead_ticks += 1;
                if a.dead_ticks > 180 {
                    a.meat = 0;
                }
                ctx.db.wildlife().id().update(a);
                continue;
            }
            let predator = nature::predator(a.species);
            if alerted.contains(&a.group_id) {
                a.state = 2;
            }
            let near = units
                .iter()
                .filter(|u| on_ground(ctx, u) && ctx.db.army_unit().id().find(u.id).is_some())
                .min_by(|u, v| {
                    map::distance((u.x, u.z), (a.x, a.z))
                        .total_cmp(&map::distance((v.x, v.z), (a.x, a.z)))
                });
            if predator && a.state == 2 {
                if let Some(u) = near.filter(|u| map::distance((u.x, u.z), (a.x, a.z)) < 45.) {
                    let d = map::distance((a.x, a.z), (u.x, u.z));
                    if d < 1.6 {
                        if let Some(mut target) = ctx.db.army_unit().id().find(u.id) {
                            alert_attack(
                                ctx,
                                target.game_id,
                                target.owner,
                                target.id,
                                false,
                                target.x,
                                target.z,
                            );
                            target.health =
                                (target.health - if a.species == 4 { 0.12 } else { 0.045 }).max(0.);
                            if target.health <= 0. {
                                kill_combat_unit(ctx, target.id);
                            } else {
                                ctx.db.army_unit().id().update(target);
                            }
                        }
                    } else {
                        let speed = if a.species == 4 { 2.0 } else { 3.2 };
                        move_animal_toward(ctx, &mut a, (u.x, u.z), &obs, speed);
                    }
                } else {
                    a.state = 0;
                }
            } else if !predator && near.is_some_and(|u| map::distance((u.x, u.z), (a.x, a.z)) < 6.)
            {
                let u = near.unwrap();
                let dx = a.x - u.x;
                let dz = a.z - u.z;
                let d = dx.hypot(dz).max(0.1);
                let goal = (a.x + dx / d * 1.8, a.z + dz / d * 1.8);
                if map::free(goal, &obs) && map::segment_clear((a.x, a.z), goal, &obs) {
                    a.x = goal.0;
                    a.z = goal.1;
                    a.state = 1;
                }
            } else {
                let phase = state.seconds as f32 * 0.12 + a.id as f32 * 2.4;
                let goal = (
                    a.home_x + phase.sin() * 7.,
                    a.home_z + (phase * 0.73).cos() * 7.,
                );
                let d = map::distance((a.x, a.z), goal);
                a.state = if d > 0.3 { 1 } else { 0 };
                if d > 0.3 && map::segment_clear((a.x, a.z), goal, &obs) {
                    let step = 1.1f32.min(d);
                    a.x += (goal.0 - a.x) * step / d;
                    a.z += (goal.1 - a.z) * step / d;
                }
            }
            // Rare pack/bear excursions only enter a settlement if its camp is reachable.
            if predator && nature::raid_attempt(game, state.seconds, a.group_id) {
                if let Some(keep) = ctx
                    .db
                    .building()
                    .game_id()
                    .filter(game)
                    .filter(|b| b.kind == "keep")
                    .min_by(|b, c| {
                        map::distance((a.x, a.z), (b.x, b.z))
                            .total_cmp(&map::distance((a.x, a.z), (c.x, c.z)))
                    })
                {
                    let goal = tower_geometry(ctx, &keep).world((0., 11.));
                    if map::distance((a.x, a.z), goal) < 750.
                        && map::shortest_path((a.x, a.z), goal, &obs).is_some()
                    {
                        ctx.db.wildlife_order().animal_id().delete(a.id);
                        ctx.db.wildlife_order().insert(WildlifeOrder {
                            animal_id: a.id,
                            path: vec![goal.0, goal.1],
                        });
                    }
                }
            }
            if let Some(order) = ctx.db.wildlife_order().animal_id().find(a.id) {
                let goal = (order.path[0], order.path[1]);
                if a.state != 2 {
                    move_animal_toward(ctx, &mut a, goal, &obs, 2.4);
                }
                if map::distance((a.x, a.z), goal) < 3. {
                    a.state = 2;
                    // Waiting peasants are population records, not controllable army units.
                    // An accessible camp can be raided even when its lord remains safely on the roof.
                    if state.seconds % 8 == 0 {
                        if let Some(keep) = ctx
                            .db
                            .building()
                            .game_id()
                            .filter(game)
                            .filter(|b| b.kind == "keep")
                            .find(|b| {
                                !policy_for(ctx, b).alarm
                                    && map::distance(tower_geometry(ctx, b).world((0., 11.)), goal)
                                        < 1.
                            })
                        {
                            if let Some(mut pop) =
                                ctx.db.keep_population().building_id().find(keep.id)
                            {
                                if pop.arrived > employed_peasants(ctx, keep.owner) {
                                    pop.arrived -= 1;
                                    ctx.db.keep_population().building_id().update(pop);
                                }
                            }
                        }
                    }
                    let elapsed = order.path.get(2).copied().unwrap_or(0.) + 1.;
                    if elapsed >= 45. {
                        ctx.db.wildlife_order().animal_id().delete(a.id);
                        a.state = 0;
                    } else {
                        ctx.db.wildlife_order().animal_id().update(WildlifeOrder {
                            animal_id: a.id,
                            path: vec![goal.0, goal.1, elapsed],
                        });
                    }
                }
            }
            ctx.db.wildlife().id().update(a);
        }
        ctx.db.ecology_state().game_id().update(state);
    }
    Ok(())
}

#[spacetimedb::table(name=predator_revision)]
pub struct PredatorRevision {
    #[primary_key]
    pub game_id: u64,
}
#[spacetimedb::table(name=ecology_balance)]
pub struct EcologyBalance {
    #[primary_key]
    pub game_id: u64,
}
fn balance_predators(ctx: &ReducerContext, game: u64) {
    let keeps = ctx
        .db
        .building()
        .game_id()
        .filter(game)
        .filter(|b| b.kind == "keep")
        .collect::<Vec<_>>();
    let initial = ctx.db.predator_revision().game_id().find(game).is_none();
    for group in 9..15usize {
        let mut animals = ctx
            .db
            .wildlife()
            .game_id()
            .filter(game)
            .filter(|a| a.group_id == group as u32)
            .collect::<Vec<_>>();
        animals.sort_by_key(|a| a.id);
        let (_, count, hx, hz) = nature::HERDS[group];
        for (i, mut a) in animals.into_iter().enumerate() {
            if initial && i >= count as usize {
                ctx.db.wildlife_order().animal_id().delete(a.id);
                ctx.db.wildlife().id().delete(a.id);
                continue;
            }
            if a.health <= 0. {
                continue;
            }
            if initial
                || keeps.iter().any(|b| {
                    map::distance((a.home_x, a.home_z), (b.x, b.z))
                        < nature::PREDATOR_START_DISTANCE
                })
            {
                let obs = obstacles(ctx, game);
                let point = [
                    (hx, hz),
                    (-340., 300.),
                    (340., -320.),
                    (-340., -330.),
                    (340., 330.),
                ]
                .into_iter()
                .find(|p| {
                    map::free(*p, &obs)
                        && keeps.iter().all(|b| {
                            map::distance(*p, (b.x, b.z)) >= nature::PREDATOR_START_DISTANCE
                        })
                });
                if let Some(p) = point {
                    a.x = p.0 + i as f32 * 1.3;
                    a.z = p.1;
                    a.home_x = p.0;
                    a.home_z = p.1;
                    a.state = 0;
                    ctx.db.wildlife_order().animal_id().delete(a.id);
                    ctx.db.wildlife().id().update(a);
                }
            }
        }
    }
    if initial {
        ctx.db
            .predator_revision()
            .insert(PredatorRevision { game_id: game });
    }
}

#[spacetimedb::table(name=lord_vitality)]
pub struct LordVitality {
    #[primary_key]
    pub unit_id: u64,
}

#[spacetimedb::table(name=predator_peace)]
pub struct PredatorPeace {
    #[primary_key]
    pub game_id: u64,
    pub until_seconds: u32,
}

fn river_bank(p: map::Point) -> bool {
    map::terrain::buildable(map::BuildingKind::Housing, p.0, p.1)
        && [(2.8, 0.), (-2.8, 0.), (0., 2.8), (0., -2.8)]
            .iter()
            .any(|d| map::terrain::biome(p.0 + d.0, p.1 + d.1) == map::terrain::Biome::River)
}

fn fishing_bank(b: &Building, from: map::Point, obs: &[Obstacle]) -> Option<map::Point> {
    let mut points = (-15..=15)
        .flat_map(|i| (-15..=15).map(move |j| (b.x + i as f32 * 2., b.z + j as f32 * 2.)))
        .filter(|p| river_bank(*p) && map::free(*p, obs))
        .collect::<Vec<_>>();
    points.sort_by(|a, b| map::distance(from, *a).total_cmp(&map::distance(from, *b)));
    return points
        .into_iter()
        .find(|p| map::shortest_path(from, *p, obs).is_some());
}

fn reconcile_carts(ctx: &ReducerContext) {
    let carts = ctx.db.ox_cart().iter().collect::<Vec<_>>();
    for mut c in carts {
        let Some(home) = ctx.db.building().id().find(c.building_id) else {
            continue;
        };
        if ctx
            .db
            .participant()
            .connection_id()
            .find(home.owner)
            .is_none()
        {
            continue;
        }
        if let Some(source) = c.source_building {
            if ctx
                .db
                .worker()
                .source_building()
                .filter(source)
                .any(|w| w.cargo > 0)
            {
                continue;
            }
            let valid = ctx.db.building().id().find(source).is_some_and(|b| {
                b.owner == home.owner
                    && map::distance((b.x, b.z), (home.x, home.z)) <= nature::CART_RANGE
                    && building_open(ctx, b.id)
                    && ctx
                        .db
                        .worker()
                        .source_building()
                        .filter(b.id)
                        .next()
                        .is_some()
            });
            if valid {
                continue;
            }
            c.source_building = None;
            ctx.db.ox_cart().building_id().update(c.clone());
        }
        let target = ctx
            .db
            .building()
            .game_id()
            .filter(home.game_id)
            .filter(|b| {
                b.owner == home.owner
                    && matches!(b.kind.as_str(), "quarry" | "iron_mine" | "saw_pit")
                    && building_open(ctx, b.id)
                    && cart_for(ctx, b.id).is_none()
                    && ctx
                        .db
                        .worker()
                        .source_building()
                        .filter(b.id)
                        .next()
                        .is_some()
                    && map::distance((home.x, home.z), (b.x, b.z)) <= nature::CART_RANGE
            })
            .min_by(|a, b| {
                let rank = |b: &Building| {
                    (
                        if b.kind == "saw_pit" { 1 } else { 0 },
                        map::distance((home.x, home.z), (b.x, b.z)) as u32,
                        b.id,
                    )
                };
                rank(a).cmp(&rank(b))
            });
        if let Some(b) = target {
            if attach_cart(ctx, c.clone(), &b).is_err() {
                c.source_building = None;
                ctx.db.ox_cart().building_id().update(c);
            }
        }
    }
}
#[spacetimedb::table(name=resource_sizing_v2)]
pub struct ResourceSizingV2 {
    #[primary_key]
    pub game_id: u64,
}
fn upgrade_resource_sizes(ctx: &ReducerContext, game: u64) {
    if ctx.db.resource_sizing_v2().game_id().find(game).is_some() {
        return;
    }

    for mut n in ctx
        .db
        .resource_node()
        .game_id()
        .filter(game)
        .filter(|n| n.kind <= 2)
        .collect::<Vec<_>>()
    {
        let target = match n.kind {
            0 => nature::TREE_WOOD,
            1 => nature::deposit_capacity(1, n.site.saturating_sub(10000) as usize),
            _ => nature::deposit_capacity(2, n.site.saturating_sub(20000) as usize),
        };
        if n.capacity != target {
            if n.remaining > 0 {
                n.remaining = target.saturating_sub(n.capacity.saturating_sub(n.remaining));
            }
            n.capacity = target;
            ctx.db.resource_node().id().update(n);
        }
    }
    ctx.db
        .resource_sizing_v2()
        .insert(ResourceSizingV2 { game_id: game });
}
