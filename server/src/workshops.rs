// Extra workshop cargo remains additive to the original warehouse schema.
#[derive(Clone)]
#[spacetimedb::table(name=workshop_stock,public)]
pub struct WorkshopStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub tools: u32,
    pub grapes: u32,
    pub olives: u32,
    pub wine: u32,
    pub oil: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=workshop_recipe,public)]
pub struct WorkshopRecipe {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub item: String,
}
fn goods_value(s: &WorkshopStock, k: Commodity) -> u32 {
    match k {
        Commodity::Tools => s.tools,
        Commodity::Grapes => s.grapes,
        Commodity::Olives => s.olives,
        Commodity::Wine => s.wine,
        Commodity::Oil => s.oil,
        _ => 0,
    }
}
fn goods_add(s: &mut WorkshopStock, k: Commodity, n: i32) {
    let a = match k {
        Commodity::Tools => &mut s.tools,
        Commodity::Grapes => &mut s.grapes,
        Commodity::Olives => &mut s.olives,
        Commodity::Wine => &mut s.wine,
        Commodity::Oil => &mut s.oil,
        _ => return,
    };
    *a = (*a as i64 + i64::from(n)).max(0) as u32;
}
fn goods_stock(ctx: &ReducerContext, id: u64, game: u64) -> WorkshopStock {
    ctx.db
        .workshop_stock()
        .building_id()
        .find(id)
        .unwrap_or(WorkshopStock {
            building_id: id,
            game_id: game,
            tools: 0,
            grapes: 0,
            olives: 0,
            wine: 0,
            oil: 0,
        })
}
fn put_goods(ctx: &ReducerContext, s: WorkshopStock) {
    if ctx
        .db
        .workshop_stock()
        .building_id()
        .find(s.building_id)
        .is_some()
    {
        ctx.db.workshop_stock().building_id().update(s);
    } else {
        ctx.db.workshop_stock().insert(s);
    }
}
fn consume_goods(ctx: &ReducerContext, b: &Building, k: Commodity, n: u32) -> bool {
    let mut stocks = ctx
        .db
        .workshop_stock()
        .game_id()
        .filter(b.game_id)
        .filter(|s| {
            ctx.db
                .building()
                .id()
                .find(s.building_id)
                .is_some_and(|x| x.owner == b.owner)
        })
        .collect::<Vec<_>>();
    stocks.sort_by_key(|s| s.building_id);
    if stocks.iter().map(|s| goods_value(s, k)).sum::<u32>() < n {
        return false;
    }
    let mut left = n;
    for mut s in stocks {
        let take = left.min(goods_value(&s, k));
        goods_add(&mut s, k, -(take as i32));
        put_goods(ctx, s);
        left -= take;
        if left == 0 {
            break;
        }
    }
    true
}
#[spacetimedb::reducer]
pub fn set_workshop_recipe(
    ctx: &ReducerContext,
    building_id: u64,
    item: String,
) -> Result<(), String> {
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .filter(|b| {
            b.owner == p.connection_id && b.game_id == p.game_id && b.kind == "weapon_smith"
        })
        .ok_or("Wähle deinen Waffenschmied.")?;
    Equipment::parse(&item).ok_or("Wähle Bogen, Schwert oder Schild.")?;
    let row = WorkshopRecipe {
        building_id: b.id,
        game_id: b.game_id,
        item,
    };
    if ctx.db.workshop_recipe().building_id().find(b.id).is_some() {
        ctx.db.workshop_recipe().building_id().update(row);
    } else {
        ctx.db.workshop_recipe().insert(row);
    }
    Ok(())
}
fn produce_special(ctx: &ReducerContext, b: &Building, p: &mut Producer) {
    if b.kind == "hospital" {
        for mut u in ctx
            .db
            .army_unit()
            .owner()
            .filter(b.owner)
            .filter(|u| {
                u.game_id == b.game_id
                    && map::distance((u.x, u.z), (b.x, b.z)) < 16.
                    && ctx.db.unit_engagement().unit_id().find(u.id).is_none()
            })
            .collect::<Vec<_>>()
        {
            let max = ctx
                .db
                .unit_profile()
                .unit_id()
                .find(u.id)
                .and_then(|p| map::units::UnitKind::from_name(&p.kind))
                .map_or(1., |k| k.max_health());
            if u.health < max {
                u.health = (u.health + 0.035).min(max);
                ctx.db.army_unit().id().update(u);
            }
        }
        return;
    }
    p.progress += advanced_work(ctx, b, p);
    if p.progress < 12 {
        return;
    }
    if b.kind == "tool_smith" {
        let Some(stock) = own_warehouses(ctx, b.game_id, b.owner)
            .into_iter()
            .find(|s| {
                goods_value(
                    &goods_stock(ctx, s.building_id, b.game_id),
                    Commodity::Tools,
                ) < 120
            })
        else {
            return;
        };
        let player = ctx.db.participant().connection_id().find(b.owner).unwrap();
        if debit_cost(ctx, &player, map::Cost::new(1, 0, 1)).is_err() {
            return;
        }
        let mut goods = goods_stock(ctx, stock.building_id, b.game_id);
        goods.tools += 1;
        put_goods(ctx, goods);
        p.progress = 0;
    }
    if b.kind == "weapon_smith" {
        let item = ctx
            .db
            .workshop_recipe()
            .building_id()
            .find(b.id)
            .and_then(|r| Equipment::parse(&r.item))
            .unwrap_or(Equipment::Sword);
        let Some(mut stock) = arsenal_destination(ctx, b, item) else {
            return;
        };
        let player = ctx.db.participant().connection_id().find(b.owner).unwrap();
        if debit_cost(ctx, &player, item.cost()).is_err() {
            return;
        }
        gear_add(&mut stock, item, 1);
        ctx.db.arsenal_stock().building_id().update(stock);
        p.progress = 0;
    }
}
#[spacetimedb::reducer]
pub fn sell_workshop_goods(ctx: &ReducerContext, resource: u8, amount: u32) -> Result<(), String> {
    let p = participant(ctx)?;
    own_market(ctx, &p)?;
    let k = Commodity::from_u8(resource)
        .filter(|_| resource >= 11 && resource <= 15)
        .ok_or("Wähle ein Handelsgut.")?;
    if amount == 0 || amount > 120 {
        return Err("Wähle 1 bis 120 Waren.".into());
    }
    let b = ctx
        .db
        .building()
        .owner()
        .filter(p.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Palast fehlt.")?;
    if !consume_goods(ctx, &b, k, amount) {
        return Err("Nicht genug Waren im Lager.".into());
    }
    let pressure = ctx
        .db
        .market_pressure()
        .game_id()
        .filter(p.game_id)
        .find(|r| r.resource == resource);
    let old = pressure.as_ref().map_or(0, |r| r.pressure);
    let base = match k {
        Commodity::Tools => 8,
        Commodity::Wine => 9,
        Commodity::Oil => 8,
        _ => 3,
    };
    let value = (0..amount)
        .map(|i| ((base * (100 + (old - i as i32).clamp(-60, 300)) * 85 / 10000).max(1)) as u64)
        .sum::<u64>();
    let mut t = ctx
        .db
        .treasury()
        .connection_id()
        .find(p.connection_id)
        .unwrap();
    t.gold = t.gold.saturating_add(value);
    ctx.db.treasury().connection_id().update(t);
    let row = MarketPressure {
        id: pressure.as_ref().map_or(0, |r| r.id),
        game_id: p.game_id,
        resource,
        pressure: (old - amount as i32).max(-60),
    };
    if pressure.is_some() {
        ctx.db.market_pressure().id().update(row);
    } else {
        ctx.db.market_pressure().insert(row);
    }
    Ok(())
}

fn luxury_bonus(ctx: &ReducerContext, b: &Building) -> i32 {
    let stocks = ctx
        .db
        .workshop_stock()
        .game_id()
        .filter(b.game_id)
        .filter(|s| {
            ctx.db
                .building()
                .id()
                .find(s.building_id)
                .is_some_and(|x| x.owner == b.owner)
        })
        .collect::<Vec<_>>();
    i32::from(stocks.iter().any(|s| s.wine > 0)) * 2
        + i32::from(stocks.iter().any(|s| s.oil > 0)) * 2
}
