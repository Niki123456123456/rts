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
                n[3].saturating_add(s.meat),
            ]
        });
    let stock = ctx.db.player_resources().connection_id().find(keep.owner);
    let value = map::needs::Satisfaction::calculate(map::needs::NeedsInput {
        food,
        goods: stock.map_or([0; 3], |s| [s.wood, s.stone, s.iron]),
        population,
        capacity,
        employed: employed_peasants(ctx, keep.owner),
        workplaces: buildings
            .iter()
            .filter(|b| {
                map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.produces().is_some())
            })
            .count() as u32,
        water: buildings.iter().any(|b| b.kind == "well"),
        healthcare: buildings.iter().any(|b| b.kind == "apothecary"),
        leisure: buildings
            .iter()
            .filter(|b| b.kind == "inn" || b.kind == "gardeners_hut")
            .count() as u32,
    });
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
    let mut remaining = population.div_ceil(4);
    let mut stores = ctx
        .db
        .granary_stock()
        .game_id()
        .filter(keep.game_id)
        .filter(|s| s.owner == keep.owner)
        .collect::<Vec<_>>();
    stores.sort_by_key(|s| s.building_id);
    for store in &mut stores {
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

#[spacetimedb::reducer]
pub fn assign_peasant(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let player = participant(ctx)?;
    let building = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .ok_or("Unbekanntes Gebäude.")?;
    if building.owner != player.connection_id || building.game_id != player.game_id {
        return Err("Du kannst Bauern nur deinen eigenen Gebäuden zuweisen.".into());
    }
    if map::BuildingKind::from_name(&building.kind)
        .and_then(|k| k.produces())
        .is_none()
    {
        return Err("Dieses Gebäude hat keinen Produktionsarbeitsplatz.".into());
    }
    if ctx
        .db
        .worker()
        .source_building()
        .filter(building_id)
        .next()
        .is_some()
    {
        return Err("Der Arbeitsplatz ist bereits besetzt.".into());
    }
    let keep = ctx
        .db
        .building()
        .owner()
        .filter(player.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Es fehlt ein Bergfried mit Lagerfeuer.")?;
    let population = ctx
        .db
        .keep_population()
        .building_id()
        .find(keep.id)
        .ok_or("Es sind noch keine Bauern angekommen.")?;
    if population.arrived <= employed_peasants(ctx, player.connection_id) {
        return Err("Am Lagerfeuer wartet kein freier Bauer. Verbessere Versorgung und Wohnraum für weiteren Zuzug.".into());
    }
    let shape = building_shape(ctx, &building);
    let obs = obstacles(ctx, player.game_id);
    let reserved = ctx
        .db
        .army_unit()
        .game_id()
        .filter(player.game_id)
        .filter(|u| on_ground(ctx, u))
        .map(|u| (u.x, u.z))
        .collect::<Vec<_>>();
    let camp = tower_geometry(ctx, &keep).world((0., 11.));
    let point = (0..16)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 16.;
            (camp.0 + a.cos() * 2., camp.1 + a.sin() * 2.)
        })
        .chain(
            map::attack_positions(shape)
                .into_iter()
                .filter(|p| p.ring == 0)
                .map(|p| p.point),
        )
        .find(|p| {
            map::free(*p, &obs)
                && reserved
                    .iter()
                    .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
                && map::attack_positions(shape)
                    .into_iter()
                    .filter(|s| s.ring == 0)
                    .any(|s| map::shortest_path(*p, s.point, &obs).is_some())
        })
        .ok_or("Der Weg vom Lagerfeuer zum Arbeitsplatz ist blockiert.".to_owned())?;
    spawn_production_worker(ctx, &building, point)?;
    settlement_satisfaction(ctx, &keep, population.arrived);
    Ok(())
}
#[spacetimedb::reducer]
pub fn unassign_peasant(ctx: &ReducerContext, building_id: u64) -> Result<(), String> {
    let player = participant(ctx)?;
    let building = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .ok_or("Unbekanntes Gebäude.")?;
    if building.owner != player.connection_id || building.game_id != player.game_id {
        return Err("Das ist nicht dein Gebäude.".into());
    }
    let worker = ctx
        .db
        .worker()
        .source_building()
        .filter(building_id)
        .next()
        .ok_or("Kein Bauer zugewiesen.")?;
    if let Some(mut producer) = ctx.db.producer().building_id().find(building_id) {
        producer.buffer = producer.buffer.saturating_add(worker.cargo);
        ctx.db.producer().building_id().update(producer);
    }
    ctx.db.unit_order().unit_id().delete(worker.unit_id);
    ctx.db.army_unit().id().delete(worker.unit_id);
    ctx.db.worker().unit_id().delete(worker.unit_id);
    Ok(())
}
