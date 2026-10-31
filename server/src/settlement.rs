#[derive(Clone)]
#[spacetimedb::table(name = settlement_needs, public)]
pub struct SettlementNeeds {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub happiness: u32,
    pub food: i32,
    pub variety: i32,
    pub housing: i32,
    pub water: i32,
    pub health: i32,
    pub leisure: i32,
    pub goods: i32,
    pub employment: i32,
    pub immigration_seconds: u32,
}
// Migration progress is separate so changing satisfaction cannot manufacture an
// arrival at a modulo boundary. Existing public population rows stay compatible.
#[spacetimedb::table(name = settlement_progress)]
pub struct SettlementProgress {
    #[primary_key]
    pub building_id: u64,
    pub ticks: u32,
}
fn employed_peasants(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) -> u32 {
    ctx.db
        .worker()
        .iter()
        .filter(|w| {
            ctx.db
                .building()
                .id()
                .find(w.source_building)
                .is_some_and(|b| b.owner == owner)
        })
        .count() as u32
}
fn settlement_satisfaction(
    ctx: &ReducerContext,
    keep: &Building,
    population: u32,
) -> map::needs::Satisfaction {
    let buildings = ctx
        .db
        .building()
        .owner()
        .filter(keep.owner)
        .collect::<Vec<_>>();
    let capacity = map::population::Population::from_buildings(
        buildings
            .iter()
            .filter_map(|b| map::BuildingKind::from_name(&b.kind)),
    )
    .capacity;
    let food = ctx
        .db
        .granary_stock()
        .game_id()
        .filter(keep.game_id)
        .filter(|s| s.owner == keep.owner)
        .fold([0u32; 4], |n, s| {
            [
                n[0].saturating_add(s.apples),
                n[1].saturating_add(s.bread),
                n[2].saturating_add(s.cheese),
                n[3].saturating_add(s.meat).saturating_add(
                    ctx.db
                        .fish_stock()
                        .building_id()
                        .find(s.building_id)
                        .map_or(0, |f| f.amount),
                ),
            ]
        });
    let stock = ctx.db.player_resources().connection_id().find(keep.owner);
    let mut value = map::needs::Satisfaction::calculate(map::needs::NeedsInput {
        food,
        goods: stock.map_or([0; 3], |s| [s.wood, s.stone, s.iron]),
        population,
        capacity,
        employed: employed_peasants(ctx, keep.owner),
        workplaces: buildings
            .iter()
            .filter(|b| {
                building_open(ctx, b.id)
                    && map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.produces().is_some())
            })
            .count() as u32,
        water: buildings
            .iter()
            .any(|b| matches!(b.kind.as_str(), "well" | "aqueduct") && building_open(ctx, b.id)),
        healthcare: buildings.iter().any(|b| {
            matches!(b.kind.as_str(), "apothecary" | "hospital" | "baths")
                && building_open(ctx, b.id)
        }),
        leisure: buildings
            .iter()
            .filter(|b| {
                matches!(
                    b.kind.as_str(),
                    "inn" | "gardeners_hut" | "theater" | "colosseum" | "temple"
                ) && building_open(ctx, b.id)
            })
            .count() as u32,
    });
    value.happiness = (value.happiness as i32
        + civic_happiness(ctx, keep, food.iter().sum())
        + map::infrastructure::wonder_happiness(
            buildings
                .iter()
                .filter(|b| building_open(ctx, b.id))
                .filter_map(|b| map::BuildingKind::from_name(&b.kind)),
        ))
    .clamp(0, 100) as u32;
    value.immigration_seconds = civic::immigration(value.happiness);
    let row = SettlementNeeds {
        building_id: keep.id,
        game_id: keep.game_id,
        happiness: value.happiness,
        food: value.food,
        variety: value.variety,
        housing: value.housing,
        water: value.water,
        health: value.health,
        leisure: value.leisure,
        goods: value.goods,
        employment: value.employment,
        immigration_seconds: value.immigration_seconds,
    };
    if ctx
        .db
        .settlement_needs()
        .building_id()
        .find(keep.id)
        .is_some()
    {
        ctx.db.settlement_needs().building_id().update(row);
    } else {
        ctx.db.settlement_needs().insert(row);
    }
    value
}
fn feed_population(ctx: &ReducerContext, keep: &Building, population: u32, round: u32) {
    // One ration per four peasants every thirty seconds, from real granary stock.
    let mut policy = policy_for(ctx, keep);
    let (mut remaining, remainder) =
        civic::ration_use(population, policy.rations, policy.food_remainder);
    policy.food_remainder = remainder;
    ctx.db.civic_policy().building_id().update(policy);
    let mut stores = ctx
        .db
        .granary_stock()
        .game_id()
        .filter(keep.game_id)
        .filter(|s| s.owner == keep.owner)
        .collect::<Vec<_>>();
    stores.sort_by_key(|s| s.building_id);
    for store in &mut stores {
        if let Some(mut fish) = ctx.db.fish_stock().building_id().find(store.building_id) {
            let take = remaining.min(fish.amount);
            fish.amount -= take;
            remaining -= take;
            ctx.db.fish_stock().building_id().update(fish);
        }
        for offset in 0..4 {
            let amount = match (round + offset) % 4 {
                0 => &mut store.apples,
                1 => &mut store.bread,
                2 => &mut store.cheese,
                _ => &mut store.meat,
            };
            let take = remaining.min(*amount);
            *amount -= take;
            remaining -= take;
        }
        ctx.db.granary_stock().building_id().update(store.clone());
        if remaining == 0 {
            break;
        }
    }
}

// Missing operation row means open, including buildings from earlier versions.
#[derive(Clone)]
#[spacetimedb::table(name = building_operation, public)]
pub struct BuildingOperation {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub closed: bool,
}
fn building_open(ctx: &ReducerContext, id: u64) -> bool {
    !ctx.db
        .building_operation()
        .building_id()
        .find(id)
        .is_some_and(|s| s.closed)
}
fn production_building(ctx: &ReducerContext, id: u64) -> Result<Building, String> {
    let player = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(id)
        .ok_or("Unbekanntes Gebäude.")?;
    if b.owner != player.connection_id || b.game_id != player.game_id {
        return Err("Du kannst nur deine eigenen Gebäude öffnen und schließen.".into());
    }
    if map::BuildingKind::from_name(&b.kind)
        .and_then(|k| k.produces())
        .is_none()
    {
        return Err("Dieses Gebäude hat keinen Produktionsarbeitsplatz.".into());
    }
    Ok(b)
}
fn auto_assign_peasants(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) {
    if owner_alarm(ctx, owner) {
        return;
    }
    let Some(keep) = ctx
        .db
        .building()
        .owner()
        .filter(owner)
        .find(|b| b.kind == "keep")
    else {
        return;
    };
    let Some(population) = ctx.db.keep_population().building_id().find(keep.id) else {
        return;
    };
    let mut free = population
        .arrived
        .saturating_sub(employed_peasants(ctx, owner));
    if free == 0 {
        return;
    }
    let mut buildings = ctx
        .db
        .building()
        .owner()
        .filter(owner)
        .filter(|b| {
            b.game_id == keep.game_id
                && building_open(ctx, b.id)
                && map::BuildingKind::from_name(&b.kind)
                    .and_then(|k| k.produces())
                    .is_some()
                && ctx
                    .db
                    .worker()
                    .source_building()
                    .filter(b.id)
                    .next()
                    .is_none()
        })
        .collect::<Vec<_>>();
    buildings.sort_by_key(|b| b.id);
    let obs = obstacles(ctx, keep.game_id);
    let spots = map::camp::spots(tower_geometry(ctx, &keep), &obs, 64);
    for b in buildings {
        if free == 0 {
            break;
        }
        let reserved = ctx
            .db
            .army_unit()
            .game_id()
            .filter(keep.game_id)
            .filter(|u| on_ground(ctx, u))
            .map(|u| (u.x, u.z))
            .collect::<Vec<_>>();
        let shape = building_shape(ctx, &b);
        let point = spots.iter().copied().find(|p| {
            reserved
                .iter()
                .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
                && ((b.kind == "fishpond" && fishing_bank(&b, *p, &obs).is_some())
                    || (b.kind != "fishpond"
                        && map::attack_positions(shape)
                            .into_iter()
                            .filter(|s| s.ring == 0)
                            .any(|s| map::shortest_path(*p, s.point, &obs).is_some())))
        });
        if let Some(point) = point {
            if spawn_production_worker(ctx, &b, point).is_ok() {
                free -= 1;
            }
        }
    }
}
// Retain reducer names for existing clients; opening now queues automatic staffing.
#[spacetimedb::reducer]
pub fn assign_peasant(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let b = production_building(ctx, building_id)?;
    ctx.db.building_operation().building_id().delete(b.id);
    auto_assign_peasants(ctx, b.owner);
    Ok(())
}
#[spacetimedb::reducer]
pub fn unassign_peasant(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let b = production_building(ctx, building_id)?;
    let row = BuildingOperation {
        building_id: b.id,
        game_id: b.game_id,
        closed: true,
    };
    if ctx
        .db
        .building_operation()
        .building_id()
        .find(b.id)
        .is_some()
    {
        ctx.db.building_operation().building_id().update(row);
    } else {
        ctx.db.building_operation().insert(row);
    }
    for worker in ctx
        .db
        .worker()
        .source_building()
        .filter(b.id)
        .collect::<Vec<_>>()
    {
        if let Some(mut p) = ctx.db.producer().building_id().find(b.id) {
            p.buffer = p.buffer.saturating_add(worker.cargo);
            ctx.db.producer().building_id().update(p);
        }
        ctx.db.unit_order().unit_id().delete(worker.unit_id);
        ctx.db.army_unit().id().delete(worker.unit_id);
        ctx.db.worker().unit_id().delete(worker.unit_id);
    }
    auto_assign_peasants(ctx, b.owner);
    Ok(())
}
