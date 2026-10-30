// Roman industry, horse breeding and paid artillery manufacture. Save data is additive.
#[derive(Clone)]
#[spacetimedb::table(name=industry_stock,public)]
pub struct IndustryStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub wheat: u32,
    pub flour: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=stable_horse,public)]
pub struct StableHorse {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub stable_id: u64,
    pub age: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=factory_order,public)]
pub struct FactoryOrder {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub kind: String,
    pub progress: u32,
    pub duration: u32,
}
const HORSE_ADULT_SECONDS: u32 = 90;
fn can_store_cargo(ctx: &ReducerContext, s: &WarehouseStock, r: Commodity, n: u32) -> bool {
    if !r.is_intermediate() {
        return economy::can_store(s.amounts(), r, n);
    }
    let stock = ctx.db.industry_stock().building_id().find(s.building_id);
    let old = stock.map_or(0, |s| {
        if r == Commodity::Wheat {
            s.wheat
        } else {
            s.flour
        }
    });
    old.checked_add(n).is_some_and(|v| v <= 120)
}
fn deliver_intermediate(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    id: u64,
    r: Commodity,
    amount: u32,
) -> bool {
    let Some(store) = ctx
        .db
        .warehouse_stock()
        .building_id()
        .find(id)
        .filter(|s| can_store_cargo(ctx, s, r, amount))
    else {
        return false;
    };
    if !ctx
        .db
        .building()
        .id()
        .find(id)
        .is_some_and(|b| b.owner == u.owner && b.game_id == u.game_id)
    {
        return false;
    }
    let old = ctx.db.industry_stock().building_id().find(id);
    let mut row = old.clone().unwrap_or(IndustryStock {
        building_id: id,
        game_id: store.game_id,
        wheat: 0,
        flour: 0,
    });
    if r == Commodity::Wheat {
        row.wheat += amount;
    } else {
        row.flour += amount;
    }
    if old.is_some() {
        ctx.db.industry_stock().building_id().update(row);
    } else {
        ctx.db.industry_stock().insert(row);
    }
    true
}
fn consume_intermediate(ctx: &ReducerContext, b: &Building, r: Commodity, amount: u32) -> bool {
    let mut stores = ctx
        .db
        .industry_stock()
        .game_id()
        .filter(b.game_id)
        .filter(|s| {
            ctx.db
                .building()
                .id()
                .find(s.building_id)
                .is_some_and(|w| w.owner == b.owner)
        })
        .collect::<Vec<_>>();
    stores.sort_by_key(|s| s.building_id);
    if stores
        .iter()
        .map(|s| {
            if r == Commodity::Wheat {
                s.wheat
            } else {
                s.flour
            }
        })
        .sum::<u32>()
        < amount
    {
        return false;
    }
    let mut left = amount;
    for mut stock in stores {
        let value = if r == Commodity::Wheat {
            &mut stock.wheat
        } else {
            &mut stock.flour
        };
        let take = left.min(*value);
        *value -= take;
        left -= take;
        ctx.db.industry_stock().building_id().update(stock);
        if left == 0 {
            break;
        }
    }
    true
}
fn age_horses(ctx: &ReducerContext, game: u64, owner: spacetimedb::ConnectionId) {
    for mut horse in ctx
        .db
        .stable_horse()
        .game_id()
        .filter(game)
        .filter(|h| {
            h.age < HORSE_ADULT_SECONDS
                && ctx
                    .db
                    .building()
                    .id()
                    .find(h.stable_id)
                    .is_some_and(|b| b.owner == owner)
        })
        .collect::<Vec<_>>()
    {
        horse.age += 1;
        ctx.db.stable_horse().id().update(horse);
    }
}
fn take_adult_horse(ctx: &ReducerContext, p: &Participant) -> Result<(), String> {
    let horse = ctx
        .db
        .stable_horse()
        .game_id()
        .filter(p.game_id)
        .find(|h| {
            h.age >= HORSE_ADULT_SECONDS
                && ctx
                    .db
                    .building()
                    .id()
                    .find(h.stable_id)
                    .is_some_and(|b| b.owner == p.connection_id)
        })
        .ok_or("Für einen Reiter brauchst du ein erwachsenes Pferd aus deinem Stall.")?;
    ctx.db.stable_horse().id().delete(horse.id);
    Ok(())
}
#[spacetimedb::reducer]
pub fn queue_siege(ctx: &ReducerContext, building_id: u64, kind: String) -> Result<(), String> {
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .filter(|b| {
            b.owner == p.connection_id && b.game_id == p.game_id && b.kind == "siege_workshop"
        })
        .ok_or("Wähle deine Fabrica.")?;
    if !building_open(ctx, b.id) {
        return Err("Die Fabrica ist geschlossen.".into());
    }
    if ctx.db.factory_order().building_id().find(b.id).is_some() {
        return Err("Die Fabrica bearbeitet bereits einen Auftrag.".into());
    }
    let k = map::units::UnitKind::from_name(&kind)
        .filter(|k| k.is_siege())
        .ok_or("Wähle eine Balliste oder einen Onager.")?;
    let keep = ctx
        .db
        .building()
        .owner()
        .filter(p.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Palast fehlt.")?;
    let mut pop = ctx
        .db
        .keep_population()
        .building_id()
        .find(keep.id)
        .ok_or("Keine Bauern vorhanden.")?;
    if pop
        .arrived
        .saturating_sub(employed_peasants(ctx, p.connection_id))
        < 2
    {
        return Err("Zwei freie Bauern werden als Bedienmannschaft benötigt.".into());
    }
    debit_cost(ctx, &p, k.cost())?;
    pop.arrived -= 2;
    ctx.db.keep_population().building_id().update(pop);
    ctx.db.factory_order().insert(FactoryOrder {
        building_id: b.id,
        game_id: p.game_id,
        kind,
        progress: 0,
        duration: if k == map::units::UnitKind::Ballista {
            30
        } else {
            50
        },
    });
    start_production(ctx);
    Ok(())
}
fn produce_industry(ctx: &ReducerContext, b: &Building, p: &mut Producer) {
    if b.kind == "stable" {
        if ctx
            .db
            .stable_horse()
            .game_id()
            .filter(b.game_id)
            .filter(|h| h.stable_id == b.id)
            .count()
            >= 8
        {
            return;
        }
        p.progress += 1;
        if p.progress >= 45 {
            p.progress = 0;
            ctx.db.stable_horse().insert(StableHorse {
                id: 0,
                game_id: b.game_id,
                stable_id: b.id,
                age: 0,
            });
        }
        return;
    }
    let Some(mut order) = ctx.db.factory_order().building_id().find(b.id) else {
        return;
    };
    order.progress = (order.progress + 1).min(order.duration);
    p.progress = order.progress;
    if order.progress >= order.duration {
        let shape = building_shape(ctx, b);
        let obs = obstacles(ctx, b.game_id);
        let peers = ctx
            .db
            .army_unit()
            .game_id()
            .filter(b.game_id)
            .filter(|u| on_ground(ctx, u))
            .map(|u| (u.x, u.z))
            .collect::<Vec<_>>();
        let angle = rotation(ctx, b.id) as f32 * std::f32::consts::FRAC_PI_2;
        let normal = (angle.sin(), angle.cos());
        let preferred = (
            b.x + normal.0 * (shape.half_x + 2.),
            b.z + normal.1 * (shape.half_z + 2.),
        );
        let mut spots = map::attack_positions(shape);
        spots.sort_by(|a, b| {
            map::distance(a.point, preferred).total_cmp(&map::distance(b.point, preferred))
        });
        let count = ctx
            .db
            .army_unit()
            .owner()
            .filter(b.owner)
            .filter(|u| ctx.db.worker().unit_id().find(u.id).is_none())
            .count();
        if count < map::units::MAX_COMBAT_UNITS {
            if let Some(spot) = spots.into_iter().find(|s| {
                s.ring <= 2
                    && map::free(s.point, &obs)
                    && peers
                        .iter()
                        .all(|p| map::distance(*p, s.point) >= map::UNIT_SPACING)
            }) {
                let k = map::units::UnitKind::from_name(&order.kind).unwrap();
                let u = ctx.db.army_unit().insert(ArmyUnit {
                    id: 0,
                    game_id: b.game_id,
                    owner: b.owner,
                    color: b.color,
                    number: count as u32 + 1,
                    health: k.max_health(),
                    x: spot.point.0,
                    y: terrain_height(spot.point.0, spot.point.1) + roof::UNIT_CENTER_HEIGHT,
                    z: spot.point.1,
                    target_x: spot.point.0,
                    target_z: spot.point.1,
                });
                register_combat_unit(ctx, &u, k);
                ctx.db.factory_order().building_id().delete(b.id);
                p.progress = 0;
                return;
            }
        }
    }
    ctx.db.factory_order().building_id().update(order);
}
fn remove_industry_building(ctx: &ReducerContext, id: u64) {
    ctx.db.industry_stock().building_id().delete(id);
    for horse in ctx
        .db
        .stable_horse()
        .iter()
        .filter(|h| h.stable_id == id)
        .collect::<Vec<_>>()
    {
        ctx.db.stable_horse().id().delete(horse.id);
    }
    if ctx.db.factory_order().building_id().delete(id) {
        if let Some(b) = ctx.db.building().id().find(id) {
            if let Some(keep) = ctx
                .db
                .building()
                .owner()
                .filter(b.owner)
                .find(|b| b.kind == "keep")
            {
                if let Some(mut p) = ctx.db.keep_population().building_id().find(keep.id) {
                    p.arrived += 2;
                    ctx.db.keep_population().building_id().update(p);
                }
            }
        }
    }
}
#[derive(spacetimedb::SpacetimeType)]
pub struct SavedIndustry {
    pub stocks: Vec<IndustryStock>,
    pub horses: Vec<StableHorse>,
    pub orders: Vec<FactoryOrder>,
}
#[spacetimedb::table(name=industry_save)]
pub struct IndustrySave {
    #[primary_key]
    pub id: u64,
    pub data: SavedIndustry,
}
fn save_industry(ctx: &ReducerContext, id: u64, game: u64) {
    let row = IndustrySave {
        id,
        data: SavedIndustry {
            stocks: ctx.db.industry_stock().game_id().filter(game).collect(),
            horses: ctx.db.stable_horse().game_id().filter(game).collect(),
            orders: ctx.db.factory_order().game_id().filter(game).collect(),
        },
    };
    if ctx.db.industry_save().id().find(id).is_some() {
        ctx.db.industry_save().id().update(row);
    } else {
        ctx.db.industry_save().insert(row);
    }
}
fn restore_industry(
    ctx: &ReducerContext,
    id: u64,
    game: u64,
    bids: &std::collections::HashMap<u64, u64>,
) {
    let Some(save) = ctx.db.industry_save().id().find(id) else {
        return;
    };
    for mut row in save.data.stocks {
        if let Some(b) = bids.get(&row.building_id) {
            row.building_id = *b;
            row.game_id = game;
            ctx.db.industry_stock().insert(row);
        }
    }
    for mut row in save.data.horses {
        if let Some(b) = bids.get(&row.stable_id) {
            row.id = 0;
            row.stable_id = *b;
            row.game_id = game;
            ctx.db.stable_horse().insert(row);
        }
    }
    for mut row in save.data.orders {
        if let Some(b) = bids.get(&row.building_id) {
            row.building_id = *b;
            row.game_id = game;
            ctx.db.factory_order().insert(row);
        }
    }
}
