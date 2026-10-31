// Additive equipment, horse market and three authoritative research branches.
use map::advancement::{self as tech, Equipment};
#[derive(Clone)]
#[spacetimedb::table(name=arsenal_stock,public)]
pub struct ArsenalStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub bows: u32,
    pub swords: u32,
    pub shields: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=research_state,public)]
pub struct ResearchState {
    #[primary_key]
    pub keep_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub weapons: u32,
    pub economy: u32,
    pub politics: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=research_order,public)]
pub struct ResearchOrder {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub branch: String,
    pub level: u32,
    pub progress: u32,
    pub duration: u32,
}
fn technology(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) -> ResearchState {
    let keep = ctx
        .db
        .building()
        .owner()
        .filter(owner)
        .find(|b| b.kind == "keep");
    keep.and_then(|b| ctx.db.research_state().keep_id().find(b.id))
        .unwrap_or(ResearchState {
            keep_id: 0,
            game_id: 0,
            weapons: 0,
            economy: 0,
            politics: 0,
        })
}
fn branch_level(s: &ResearchState, branch: &str) -> u32 {
    match branch {
        "weapons" => s.weapons,
        "economy" => s.economy,
        _ => s.politics,
    }
}
#[spacetimedb::reducer]
pub fn start_research(
    ctx: &ReducerContext,
    building_id: u64,
    branch: String,
) -> Result<(), String> {
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(building_id)
        .filter(|b| {
            b.owner == p.connection_id && b.game_id == p.game_id && b.kind == "research_center"
        })
        .ok_or("Wähle dein Forschungszentrum.")?;
    if !building_open(ctx, b.id) || owner_alarm(ctx, b.owner) {
        return Err("Das Forschungszentrum muss geöffnet sein.".into());
    }
    if !tech::BRANCHES.contains(&branch.as_str()) {
        return Err("Unbekannter Forschungszweig.".into());
    }
    if ctx
        .db
        .research_order()
        .game_id()
        .filter(p.game_id)
        .any(|o| {
            o.building_id == b.id
                || o.branch == branch
                    && ctx
                        .db
                        .building()
                        .id()
                        .find(o.building_id)
                        .is_some_and(|x| x.owner == b.owner)
        })
    {
        return Err("Diese Forschung läuft bereits.".into());
    }
    let mut state = technology(ctx, b.owner);
    let level = branch_level(&state, &branch) + 1;
    if level > 3 {
        return Err("Dieser Forschungszweig ist abgeschlossen.".into());
    }
    let keep = ctx
        .db
        .building()
        .owner()
        .filter(b.owner)
        .find(|x| x.kind == "keep")
        .ok_or("Palast fehlt.")?;
    let mut gold = ctx
        .db
        .treasury()
        .connection_id()
        .find(b.owner)
        .ok_or("Schatzkammer fehlt.")?;
    if gold.gold < tech::research_gold(level) {
        return Err("Nicht genug Sesterzen für die Forschung.".into());
    }
    debit_cost(ctx, &p, tech::research_cost(level))?;
    gold.gold -= tech::research_gold(level);
    ctx.db.treasury().connection_id().update(gold);
    if state.keep_id == 0 {
        state.keep_id = keep.id;
        state.game_id = b.game_id;
        ctx.db.research_state().insert(state);
    }
    ctx.db.research_order().insert(ResearchOrder {
        building_id: b.id,
        game_id: b.game_id,
        branch,
        level,
        progress: 0,
        duration: tech::research_seconds(level),
    });
    start_production(ctx);
    Ok(())
}
fn research_tick(ctx: &ReducerContext, b: &Building) {
    let Some(mut o) = ctx.db.research_order().building_id().find(b.id) else {
        return;
    };
    o.progress += 1 + u32::from(
        ctx.db
            .building()
            .owner()
            .filter(b.owner)
            .any(|x| x.kind == "library" && building_open(ctx, x.id))
            && ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000 % 4 == 0,
    );
    if o.progress >= o.duration {
        let mut state = technology(ctx, b.owner);
        if state.keep_id != 0 {
            match o.branch.as_str() {
                "weapons" => state.weapons = o.level,
                "economy" => state.economy = o.level,
                _ => state.politics = o.level,
            };
            ctx.db.research_state().keep_id().update(state);
        }
        ctx.db.research_order().building_id().delete(b.id);
    } else {
        ctx.db.research_order().building_id().update(o);
    }
}
fn gear_value(s: &ArsenalStock, k: Equipment) -> u32 {
    match k {
        Equipment::Bow => s.bows,
        Equipment::Sword => s.swords,
        Equipment::Shield => s.shields,
    }
}
fn gear_add(s: &mut ArsenalStock, k: Equipment, n: i32) {
    let a = match k {
        Equipment::Bow => &mut s.bows,
        Equipment::Sword => &mut s.swords,
        Equipment::Shield => &mut s.shields,
    };
    *a = (*a as i64 + i64::from(n)).max(0) as u32;
}
fn arsenal_destination(ctx: &ReducerContext, b: &Building, k: Equipment) -> Option<ArsenalStock> {
    ctx.db
        .building()
        .game_id()
        .filter(b.game_id)
        .filter(|s| s.owner == b.owner && s.kind == "arsenal" && building_open(ctx, s.id))
        .filter_map(|s| ctx.db.arsenal_stock().building_id().find(s.id))
        .find(|s| gear_value(s, k) < 40)
}
fn queue_equipment(
    ctx: &ReducerContext,
    b: &Building,
    p: &Participant,
    k: Equipment,
) -> Result<(), String> {
    if arsenal_destination(ctx, b, k).is_none() {
        return Err(
            "Ein geöffnetes Waffenlager mit freiem Platz wird benötigt (40 je Waffentyp).".into(),
        );
    }
    debit_cost(ctx, p, k.cost())?;
    ctx.db.factory_order().insert(FactoryOrder {
        building_id: b.id,
        game_id: b.game_id,
        kind: k.name().into(),
        progress: 0,
        duration: k.seconds(),
    });
    start_production(ctx);
    Ok(())
}
fn produce_equipment(ctx: &ReducerContext, b: &Building, o: &mut FactoryOrder, k: Equipment) {
    o.progress = (o.progress + 1).min(o.duration);
    if o.progress >= o.duration {
        if let Some(mut stock) = arsenal_destination(ctx, b, k) {
            gear_add(&mut stock, k, 1);
            ctx.db.arsenal_stock().building_id().update(stock);
            ctx.db.factory_order().building_id().delete(b.id);
            return;
        }
    }
    ctx.db.factory_order().building_id().update(o.clone());
}
// Stored equipment pays the matching part of a recruit's price; missing items are
// made directly at the barracks for their ordinary raw-material price.
fn equip_recruit(ctx: &ReducerContext, p: &Participant, k: map::units::UnitKind) -> map::Cost {
    let mut cost = k.cost();
    for item in Equipment::ALL.into_iter().filter(|item| item.required(k)) {
        let row = ctx
            .db
            .arsenal_stock()
            .game_id()
            .filter(p.game_id)
            .find(|s| {
                gear_value(s, item) > 0
                    && ctx
                        .db
                        .building()
                        .id()
                        .find(s.building_id)
                        .is_some_and(|b| b.owner == p.connection_id && building_open(ctx, b.id))
            });
        if let Some(mut s) = row {
            gear_add(&mut s, item, -1);
            ctx.db.arsenal_stock().building_id().update(s);
            let c = item.cost();
            cost.wood = cost.wood.saturating_sub(c.wood);
            cost.iron = cost.iron.saturating_sub(c.iron);
            cost.stone = cost.stone.saturating_sub(c.stone);
        }
    }
    cost
}
#[spacetimedb::reducer]
pub fn sell_horses(ctx: &ReducerContext, stable_id: u64, amount: u32) -> Result<(), String> {
    let p = participant(ctx)?;
    own_market(ctx, &p)?;
    let b = ctx
        .db
        .building()
        .id()
        .find(stable_id)
        .filter(|b| b.owner == p.connection_id && b.game_id == p.game_id && b.kind == "stable")
        .ok_or("Wähle deinen Pferdestall.")?;
    if amount == 0 || amount > 8 {
        return Err("Verkaufe 1 bis 8 ausgewachsene Pferde.".into());
    }
    let mut horses = ctx
        .db
        .stable_horse()
        .game_id()
        .filter(b.game_id)
        .filter(|h| h.stable_id == b.id && h.age >= HORSE_ADULT_SECONDS)
        .collect::<Vec<_>>();
    horses.sort_by_key(|h| h.id);
    if horses.len() < amount as usize {
        return Err(
            "Nicht genug ausgewachsene Pferde. Fohlen können nicht verkauft werden.".into(),
        );
    }
    let pressure = ctx
        .db
        .market_pressure()
        .game_id()
        .filter(b.game_id)
        .find(|m| m.resource == 10);
    let old = pressure.as_ref().map_or(0, |p| p.pressure);
    let value = map::civic::horse_quote(old, amount);
    let mut t = ctx
        .db
        .treasury()
        .connection_id()
        .find(b.owner)
        .ok_or("Schatzkammer fehlt.")?;
    for h in horses.into_iter().take(amount as usize) {
        ctx.db.stable_horse().id().delete(h.id);
    }
    t.gold = t.gold.saturating_add(value);
    ctx.db.treasury().connection_id().update(t);
    let row = MarketPressure {
        id: pressure.as_ref().map_or(0, |p| p.id),
        game_id: b.game_id,
        resource: 10,
        pressure: (old - amount as i32).max(-60),
    };
    if pressure.is_some() {
        ctx.db.market_pressure().id().update(row);
    } else {
        ctx.db.market_pressure().insert(row);
    }
    Ok(())
}
fn advanced_work(ctx: &ReducerContext, b: &Building, _p: &Producer) -> u32 {
    let level = technology(ctx, b.owner).economy;
    let second = ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000;
    let tools = ctx
        .db
        .workshop_stock()
        .game_id()
        .filter(b.game_id)
        .any(|s| {
            s.tools > 0
                && ctx
                    .db
                    .building()
                    .id()
                    .find(s.building_id)
                    .is_some_and(|x| x.owner == b.owner)
        });
    if tools && second % 30 == 0 {
        consume_goods(ctx, b, Commodity::Tools, 1);
    }
    1 + u32::from(tools && second % 5 == 0)
        + u32::from(
            level > 0
                && (ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000).rem_euclid(5)
                    < i64::from(level),
        )
}
fn remove_advanced_building(ctx: &ReducerContext, id: u64) {
    ctx.db.workshop_stock().building_id().delete(id);
    ctx.db.workshop_recipe().building_id().delete(id);
    ctx.db.arsenal_stock().building_id().delete(id);
    ctx.db.research_order().building_id().delete(id);
    ctx.db.research_state().keep_id().delete(id);
}
#[derive(spacetimedb::SpacetimeType)]
pub struct AdvancedWorld {
    pub stocks: Vec<ArsenalStock>,
    pub research: Vec<ResearchState>,
    pub orders: Vec<ResearchOrder>,
    pub goods: Vec<WorkshopStock>,
    pub recipes: Vec<WorkshopRecipe>,
    pub access: Vec<SiegeAccess>,
}
#[spacetimedb::table(name=advanced_save)]
pub struct AdvancedSave {
    #[primary_key]
    pub id: u64,
    pub data: AdvancedWorld,
}
fn save_advanced(ctx: &ReducerContext, id: u64, game: u64) {
    let row = AdvancedSave {
        id,
        data: AdvancedWorld {
            stocks: ctx.db.arsenal_stock().game_id().filter(game).collect(),
            research: ctx.db.research_state().game_id().filter(game).collect(),
            orders: ctx.db.research_order().game_id().filter(game).collect(),
            goods: ctx.db.workshop_stock().game_id().filter(game).collect(),
            recipes: ctx.db.workshop_recipe().game_id().filter(game).collect(),
            access: ctx.db.siege_access().game_id().filter(game).collect(),
        },
    };
    if ctx.db.advanced_save().id().find(id).is_some() {
        ctx.db.advanced_save().id().update(row);
    } else {
        ctx.db.advanced_save().insert(row);
    }
}
fn restore_advanced(
    ctx: &ReducerContext,
    id: u64,
    game: u64,
    bids: &std::collections::HashMap<u64, u64>,
    uids: &std::collections::HashMap<u64, u64>,
) {
    if let Some(s) = ctx.db.advanced_save().id().find(id) {
        for mut r in s.data.access {
            if let (Some(unit), Some(building)) = (uids.get(&r.unit_id), bids.get(&r.building_id)) {
                r.unit_id = *unit;
                r.building_id = *building;
                r.game_id = game;
                ctx.db.siege_access().insert(r);
            }
        }
        for mut r in s.data.goods {
            if let Some(b) = bids.get(&r.building_id) {
                r.building_id = *b;
                r.game_id = game;
                put_goods(ctx, r);
            }
        }
        for mut r in s.data.recipes {
            if let Some(b) = bids.get(&r.building_id) {
                r.building_id = *b;
                r.game_id = game;
                ctx.db.workshop_recipe().insert(r);
            }
        }
        for mut r in s.data.stocks {
            if let Some(b) = bids.get(&r.building_id) {
                r.building_id = *b;
                r.game_id = game;
                if ctx.db.arsenal_stock().building_id().find(*b).is_some() {
                    ctx.db.arsenal_stock().building_id().update(r);
                } else {
                    ctx.db.arsenal_stock().insert(r);
                }
            }
        }
        for mut r in s.data.research {
            if let Some(b) = bids.get(&r.keep_id) {
                r.keep_id = *b;
                r.game_id = game;
                ctx.db.research_state().insert(r);
            }
        }
        for mut r in s.data.orders {
            if let Some(b) = bids.get(&r.building_id) {
                r.building_id = *b;
                r.game_id = game;
                ctx.db.research_order().insert(r);
            }
        }
    }
}
