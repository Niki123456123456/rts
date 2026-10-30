use map::civic;
#[derive(Clone)]
#[spacetimedb::table(name=civic_policy,public)]
pub struct CivicPolicy {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub rations: u8,
    pub taxes: u8,
    pub alarm: bool,
    pub alarm_started: u32,
    pub food_remainder: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name=treasury,public)]
pub struct Treasury {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
    #[index(btree)]
    pub game_id: u64,
    pub gold: u64,
}
#[derive(Clone)]
#[spacetimedb::table(name=market_pressure,public)]
pub struct MarketPressure {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub resource: u8,
    pub pressure: i32,
}
#[derive(Clone)]
#[spacetimedb::table(name=close_gate_order)]
pub struct CloseGateOrder {
    #[primary_key]
    pub building_id: u64,
    pub owner: spacetimedb::ConnectionId,
}
#[spacetimedb::table(name=diplomatic_request,public)]
pub struct DiplomaticRequest {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub from: spacetimedb::ConnectionId,
    pub to: spacetimedb::ConnectionId,
    pub from_color: u32,
    pub to_color: u32,
    pub resource: u8,
    pub amount: u32,
    pub offered_gold: u64,
    pub note: String,
    pub status: u8,
}
fn policy_for(ctx: &ReducerContext, b: &Building) -> CivicPolicy {
    ctx.db
        .civic_policy()
        .building_id()
        .find(b.id)
        .unwrap_or(CivicPolicy {
            building_id: b.id,
            game_id: b.game_id,
            rations: 4,
            taxes: 3,
            alarm: false,
            alarm_started: 0,
            food_remainder: 0,
        })
}
fn ensure_civic(ctx: &ReducerContext, b: &Building) {
    if ctx.db.civic_policy().building_id().find(b.id).is_none() {
        ctx.db.civic_policy().insert(policy_for(ctx, b));
    }
    if ctx.db.treasury().connection_id().find(b.owner).is_none() {
        ctx.db.treasury().insert(Treasury {
            connection_id: b.owner,
            game_id: b.game_id,
            gold: civic::STARTING_GOLD,
        });
    }
}
fn owner_alarm(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) -> bool {
    ctx.db
        .building()
        .owner()
        .filter(owner)
        .find(|b| b.kind == "keep")
        .is_some_and(|b| policy_for(ctx, &b).alarm)
}
fn civic_happiness(ctx: &ReducerContext, b: &Building, food: u32) -> i32 {
    let p = policy_for(ctx, b);
    civic::TAX_HAPPINESS[p.taxes as usize]
        + if p.rations <= 4 || food > 0 {
            civic::RATION_HAPPINESS[p.rations as usize]
        } else {
            0
        }
}
#[spacetimedb::reducer]
pub fn set_civic_policy(ctx: &ReducerContext, rations: u8, taxes: u8) -> Result<(), String> {
    if rations > 7 || taxes > 7 {
        return Err("Wähle eine der acht Stufen.".into());
    }
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .owner()
        .filter(p.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Du brauchst deinen Palast.")?;
    ensure_civic(ctx, &b);
    let mut row = policy_for(ctx, &b);
    row.rations = rations;
    row.taxes = taxes;
    ctx.db.civic_policy().building_id().update(row);
    let pop = ctx
        .db
        .keep_population()
        .building_id()
        .find(b.id)
        .map_or(0, |p| p.arrived);
    settlement_satisfaction(ctx, &b, pop);
    Ok(())
}
fn queue_gate_closures(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) {
    for b in ctx
        .db
        .building()
        .owner()
        .filter(owner)
        .filter(|b| map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_gate()))
    {
        if ctx.db.close_gate_order().building_id().find(b.id).is_none() {
            ctx.db.close_gate_order().insert(CloseGateOrder {
                building_id: b.id,
                owner,
            });
        }
    }
}
fn apply_gate_closures(ctx: &ReducerContext, owner: spacetimedb::ConnectionId) {
    let orders = ctx
        .db
        .close_gate_order()
        .iter()
        .filter(|o| o.owner == owner)
        .collect::<Vec<_>>();
    for o in orders {
        let Some(b) = ctx
            .db
            .building()
            .id()
            .find(o.building_id)
            .filter(|b| b.owner == owner)
        else {
            ctx.db
                .close_gate_order()
                .building_id()
                .delete(o.building_id);
            continue;
        };
        let k = map::BuildingKind::from_name(&b.kind).unwrap();
        let closing = if k == map::BuildingKind::DrawbridgeGate {
            fort::bridge_span((b.x, b.z), rotation(ctx, b.id))
        } else {
            fort::rect((b.x, b.z), rotation(ctx, b.id), (0., 0.), 1.9, 1.)
        };
        if ctx
            .db
            .army_unit()
            .game_id()
            .filter(b.game_id)
            .any(|u| on_ground(ctx, &u) && closing.contains((u.x, u.z), map::CLEARANCE))
        {
            continue;
        }
        if let Some(mut g) = ctx.db.gate_state().building_id().find(b.id) {
            g.open = false;
            ctx.db.gate_state().building_id().update(g);
        }
        ctx.db
            .close_gate_order()
            .building_id()
            .delete(o.building_id);
        replan(ctx, b.game_id);
    }
}
#[spacetimedb::reducer]
pub fn close_all_gates(ctx: &ReducerContext) -> Result<(), String> {
    let p = participant(ctx)?;
    queue_gate_closures(ctx, p.connection_id);
    apply_gate_closures(ctx, p.connection_id);
    start_migration(ctx);
    Ok(())
}
#[spacetimedb::reducer]
pub fn palace_alarm(ctx: &ReducerContext, active: bool) -> Result<(), String> {
    let p = participant(ctx)?;
    let b = ctx
        .db
        .building()
        .owner()
        .filter(p.connection_id)
        .find(|b| b.kind == "keep")
        .ok_or("Du brauchst deinen Palast.")?;
    ensure_civic(ctx, &b);
    let mut policy = policy_for(ctx, &b);
    policy.alarm = active;
    policy.alarm_started = ctx
        .db
        .keep_population()
        .building_id()
        .find(b.id)
        .map_or(0, |p| p.seconds);
    ctx.db.civic_policy().building_id().update(policy);
    if !active {
        for w in ctx
            .db
            .worker()
            .game_id()
            .filter(p.game_id)
            .collect::<Vec<_>>()
        {
            if let Some(mut u) = ctx
                .db
                .army_unit()
                .id()
                .find(w.unit_id)
                .filter(|u| u.owner == p.connection_id)
            {
                if let Ok(door) = depart_tower(ctx, &u) {
                    u.target_x = door.0;
                    u.target_z = door.1;
                    save_order(ctx, &u, None, vec![door]);
                    ctx.db.army_unit().id().update(u);
                }
            }
        }
        start_production(ctx);
    }
    start_migration(ctx);
    start_tick(ctx);
    Ok(())
}
fn civic_tick(ctx: &ReducerContext, b: &Building, seconds: u32) {
    ensure_civic(ctx, b);
    let policy = policy_for(ctx, b);
    if policy.alarm {
        let workers = ctx
            .db
            .worker()
            .game_id()
            .filter(b.game_id)
            .filter_map(|w| {
                ctx.db
                    .army_unit()
                    .id()
                    .find(w.unit_id)
                    .filter(|u| u.owner == b.owner)
            })
            .collect::<Vec<_>>();
        for u in &workers {
            if ctx.db.tower_occupant().unit_id().find(u.id).is_some() {
                continue;
            }
            let tower = tower_geometry(ctx, b);
            if let Some(slot) =
                (0..tower.capacity()).find(|slot| !slot_reserved(ctx, b.id, *slot as u8, u.id))
            {
                let _ = start_roof_entry(ctx, u.clone(), b, slot as u8);
            }
        }
        if seconds.saturating_sub(policy.alarm_started) >= 12
            && workers.iter().all(|u| {
                ctx.db.tower_occupant().unit_id().find(u.id).is_some()
                    && ctx.db.tower_travel().unit_id().find(u.id).is_none()
            })
        {
            queue_gate_closures(ctx, b.owner);
        }
    }
    apply_gate_closures(ctx, b.owner);
    if seconds % 30 == 0 {
        let pop = ctx
            .db
            .keep_population()
            .building_id()
            .find(b.id)
            .map_or(0, |p| p.arrived);
        if let Some(mut t) = ctx.db.treasury().connection_id().find(b.owner) {
            t.gold = t
                .gold
                .saturating_add(u64::from(pop) * civic::TAX_RATES[policy.taxes as usize])
                .min(1_000_000_000_000);
            ctx.db.treasury().connection_id().update(t);
        }
    }
}
fn own_market(ctx: &ReducerContext, p: &Participant) -> Result<(), String> {
    if owner_alarm(ctx, p.connection_id) {
        return Err("Während des Alarms ist der Handel geschlossen.".into());
    }
    if !ctx
        .db
        .building()
        .owner()
        .filter(p.connection_id)
        .any(|b| b.kind == "market" && building_open(ctx, b.id))
    {
        return Err("Baue zuerst einen eigenen geöffneten Marktplatz.".into());
    }
    Ok(())
}
fn food_value(ctx: &ReducerContext, s: &GranaryStock, r: u8) -> u32 {
    match r {
        3 => s.apples,
        4 => s.bread,
        5 => s.cheese,
        6 => s.meat,
        7 => ctx
            .db
            .fish_stock()
            .building_id()
            .find(s.building_id)
            .map_or(0, |f| f.amount),
        _ => 0,
    }
}
fn write_food(ctx: &ReducerContext, s: &mut GranaryStock, r: u8, n: u32) {
    match r {
        3 => s.apples = n,
        4 => s.bread = n,
        5 => s.cheese = n,
        6 => s.meat = n,
        7 => {
            let row = FishStock {
                building_id: s.building_id,
                game_id: s.game_id,
                amount: n,
            };
            if ctx
                .db
                .fish_stock()
                .building_id()
                .find(s.building_id)
                .is_some()
            {
                ctx.db.fish_stock().building_id().update(row);
            } else {
                ctx.db.fish_stock().insert(row);
            }
        }
        _ => {}
    }
    ctx.db.granary_stock().building_id().update(s.clone());
}
fn exchange_goods(
    ctx: &ReducerContext,
    p: &Participant,
    r: u8,
    quantity: u32,
    add: bool,
) -> Result<(), String> {
    if r > 7 || quantity > 10000 {
        return Err("Ungültige Handelsmenge.".into());
    }
    if quantity == 0 {
        return Ok(());
    }
    if r <= 2 {
        if !add {
            return debit_cost(
                ctx,
                p,
                map::Cost::new(
                    if r == 0 { quantity } else { 0 },
                    if r == 1 { quantity } else { 0 },
                    if r == 2 { quantity } else { 0 },
                ),
            );
        }
        let mut balance = ensure_resources(ctx, p);
        let mut stores = own_warehouses(ctx, p.game_id, p.connection_id);
        let mut left = quantity;
        if stores.is_empty() {
            match r {
                0 => {
                    balance.wood = balance
                        .wood
                        .checked_add(quantity)
                        .ok_or("Holzbestand zu groß.")?
                }
                1 => {
                    balance.stone = balance
                        .stone
                        .checked_add(quantity)
                        .ok_or("Steinbestand zu groß.")?
                }
                _ => {
                    balance.iron = balance
                        .iron
                        .checked_add(quantity)
                        .ok_or("Eisenbestand zu groß.")?
                }
            };
            ctx.db.player_resources().connection_id().update(balance);
            return Ok(());
        }
        let commodity = Commodity::from_u8(r).unwrap();
        for s in &mut stores {
            while left > 0 && economy::can_store(s.amounts(), commodity, 1) {
                s.add(commodity, 1);
                left -= 1;
            }
            ctx.db.warehouse_stock().building_id().update(s.clone());
        }
        if left > 0 {
            return Err("Im Vorratslager fehlt Platz.".into());
        }
        match r {
            0 => balance.wood += quantity,
            1 => balance.stone += quantity,
            _ => balance.iron += quantity,
        };
        ctx.db.player_resources().connection_id().update(balance);
        return Ok(());
    }
    let mut stores = ctx
        .db
        .granary_stock()
        .game_id()
        .filter(p.game_id)
        .filter(|s| s.owner == p.connection_id)
        .collect::<Vec<_>>();
    stores.sort_by_key(|s| s.building_id);
    let mut left = quantity;
    for s in &mut stores {
        let old = food_value(ctx, s, r);
        let amount = left.min(if add {
            economy::FOOD_CAPACITY_PER_KIND.saturating_sub(old)
        } else {
            old
        });
        if amount > 0 {
            write_food(ctx, s, r, if add { old + amount } else { old - amount });
            left -= amount;
        }
    }
    if left > 0 {
        Err(if add {
            "Im Kornspeicher fehlt Platz."
        } else {
            "Nicht genug Waren vorhanden."
        }
        .into())
    } else {
        Ok(())
    }
}
#[spacetimedb::reducer]
pub fn market_trade(
    ctx: &ReducerContext,
    resource: u8,
    amount: u32,
    buy: bool,
) -> Result<(), String> {
    let p = participant(ctx)?;
    own_market(ctx, &p)?;
    if resource > 7 || amount == 0 || amount > 200 {
        return Err("Handle zwischen 1 und 200 Waren.".into());
    }
    let old = ctx
        .db
        .market_pressure()
        .game_id()
        .filter(p.game_id)
        .find(|m| m.resource == resource);
    let pressure = old.as_ref().map_or(0, |m| m.pressure);
    let price = civic::quote(resource, pressure, amount, buy);
    let mut gold = ctx
        .db
        .treasury()
        .connection_id()
        .find(p.connection_id)
        .ok_or("Die Schatzkammer wird noch eingerichtet.")?;
    if buy && gold.gold < price {
        return Err(format!("Du brauchst {price} Sesterzen."));
    }
    exchange_goods(ctx, &p, resource, amount, buy)?;
    gold.gold = if buy {
        gold.gold - price
    } else {
        gold.gold.checked_add(price).ok_or("Schatzkammer voll.")?
    };
    ctx.db.treasury().connection_id().update(gold);
    let row = MarketPressure {
        id: old.as_ref().map_or(0, |m| m.id),
        game_id: p.game_id,
        resource,
        pressure: (pressure + if buy { amount as i32 } else { -(amount as i32) }).clamp(-60, 300),
    };
    if old.is_some() {
        ctx.db.market_pressure().id().update(row);
    } else {
        ctx.db.market_pressure().insert(row);
    }
    Ok(())
}
fn pressure_for(ctx: &ReducerContext, game: u64, resource: u8) -> i32 {
    ctx.db
        .market_pressure()
        .game_id()
        .filter(game)
        .find(|m| m.resource == resource)
        .map_or(0, |m| m.pressure)
}
fn shift_pressure(ctx: &ReducerContext, game: u64, resource: u8, delta: i32) {
    if let Some(mut row) = ctx
        .db
        .market_pressure()
        .game_id()
        .filter(game)
        .find(|m| m.resource == resource)
    {
        row.pressure = (row.pressure + delta).clamp(-60, 300);
        ctx.db.market_pressure().id().update(row);
    } else {
        ctx.db.market_pressure().insert(MarketPressure {
            id: 0,
            game_id: game,
            resource,
            pressure: delta.clamp(-60, 300),
        });
    }
}
#[spacetimedb::reducer]
pub fn market_exchange(ctx: &ReducerContext, sell: u8, amount: u32, buy: u8) -> Result<(), String> {
    let p = participant(ctx)?;
    own_market(ctx, &p)?;
    let (received, change) = civic::barter(
        sell,
        pressure_for(ctx, p.game_id, sell),
        amount,
        buy,
        pressure_for(ctx, p.game_id, buy),
    );
    if received == 0 {
        return Err(
            "Wähle verschiedene Waren und eine ausreichende Tauschmenge (1 bis 200).".into(),
        );
    }
    exchange_goods(ctx, &p, sell, amount, false)?;
    exchange_goods(ctx, &p, buy, received, true)?;
    let mut gold = ctx
        .db
        .treasury()
        .connection_id()
        .find(p.connection_id)
        .ok_or("Schatzkammer fehlt.")?;
    gold.gold = gold.gold.checked_add(change).ok_or("Schatzkammer voll.")?;
    ctx.db.treasury().connection_id().update(gold);
    shift_pressure(ctx, p.game_id, sell, -(amount as i32));
    shift_pressure(ctx, p.game_id, buy, received as i32);
    Ok(())
}
#[spacetimedb::reducer]
pub fn request_goods(
    ctx: &ReducerContext,
    target_color: u32,
    resource: u8,
    amount: u32,
    offered_gold: u64,
    note: String,
) -> Result<(), String> {
    let p = participant(ctx)?;
    if resource > 7
        || amount == 0
        || amount > 1000
        || offered_gold > 1_000_000
        || note.chars().count() > 200
    {
        return Err("Ungültige Anfrage (maximal 1000 Waren und 200 Zeichen).".into());
    }
    let target = ctx
        .db
        .participant()
        .game_id()
        .filter(p.game_id)
        .find(|t| t.color == target_color && t.connection_id != p.connection_id)
        .ok_or("Dieser Mitspieler ist nicht im Spiel.")?;
    if ctx
        .db
        .diplomatic_request()
        .game_id()
        .filter(p.game_id)
        .filter(|r| r.from == p.connection_id && r.status == 0)
        .count()
        >= 10
    {
        return Err("Es sind bereits zehn Anfragen offen.".into());
    }
    ctx.db.diplomatic_request().insert(DiplomaticRequest {
        id: 0,
        game_id: p.game_id,
        from: p.connection_id,
        to: target.connection_id,
        from_color: p.color,
        to_color: target.color,
        resource,
        amount,
        offered_gold,
        note: note.trim().into(),
        status: 0,
    });
    Ok(())
}
#[spacetimedb::reducer]
pub fn respond_request(ctx: &ReducerContext, request_id: u64, accept: bool) -> Result<(), String> {
    let p = participant(ctx)?;
    let mut r = ctx
        .db
        .diplomatic_request()
        .id()
        .find(request_id)
        .filter(|r| r.game_id == p.game_id && r.to == p.connection_id && r.status == 0)
        .ok_or("Diese Anfrage ist nicht für dich oder wurde bereits beantwortet.")?;
    if accept {
        let source = ctx
            .db
            .participant()
            .connection_id()
            .find(r.from)
            .filter(|s| s.game_id == p.game_id)
            .ok_or("Der Anfragende hat das Spiel verlassen.")?;
        let mut payer = ctx
            .db
            .treasury()
            .connection_id()
            .find(r.from)
            .ok_or("Schatzkammer fehlt.")?;
        let mut receiver = ctx
            .db
            .treasury()
            .connection_id()
            .find(p.connection_id)
            .ok_or("Schatzkammer fehlt.")?;
        if payer.gold < r.offered_gold {
            return Err("Der Anfragende hat nicht genug Geld.".into());
        }
        exchange_goods(ctx, &p, r.resource, r.amount, false)?;
        exchange_goods(ctx, &source, r.resource, r.amount, true)?;
        payer.gold -= r.offered_gold;
        receiver.gold = receiver
            .gold
            .checked_add(r.offered_gold)
            .ok_or("Schatzkammer voll.")?;
        ctx.db.treasury().connection_id().update(payer);
        ctx.db.treasury().connection_id().update(receiver);
        r.status = 1;
    } else {
        r.status = 2;
    }
    ctx.db.diplomatic_request().id().update(r);
    Ok(())
}
#[derive(spacetimedb::SpacetimeType)]
pub struct SavedCivicWorld {
    pub policies: Vec<CivicPolicy>,
    pub treasuries: Vec<Treasury>,
    pub prices: Vec<MarketPressure>,
    pub close_orders: Vec<CloseGateOrder>,
}
#[spacetimedb::table(name=civic_save)]
pub struct CivicSave {
    #[primary_key]
    pub id: u64,
    pub data: SavedCivicWorld,
}
fn save_civic_world(ctx: &ReducerContext, id: u64, game: u64) {
    let data = SavedCivicWorld {
        policies: ctx.db.civic_policy().game_id().filter(game).collect(),
        treasuries: ctx.db.treasury().game_id().filter(game).collect(),
        prices: ctx.db.market_pressure().game_id().filter(game).collect(),
        close_orders: ctx
            .db
            .close_gate_order()
            .iter()
            .filter(|o| {
                ctx.db
                    .building()
                    .id()
                    .find(o.building_id)
                    .is_some_and(|b| b.game_id == game)
            })
            .collect(),
    };
    let row = CivicSave { id, data };
    if ctx.db.civic_save().id().find(id).is_some() {
        ctx.db.civic_save().id().update(row);
    } else {
        ctx.db.civic_save().insert(row);
    }
}
fn restore_civic_world(
    ctx: &ReducerContext,
    id: u64,
    game: u64,
    buildings: &std::collections::HashMap<u64, u64>,
    owners: &std::collections::HashMap<spacetimedb::ConnectionId, spacetimedb::ConnectionId>,
) {
    let Some(saved) = ctx.db.civic_save().id().find(id) else {
        return;
    };
    for mut p in saved.data.policies {
        if let Some(b) = buildings.get(&p.building_id) {
            p.building_id = *b;
            p.game_id = game;
            ctx.db.civic_policy().insert(p);
        }
    }
    for mut t in saved.data.treasuries {
        if let Some(o) = owners.get(&t.connection_id) {
            t.connection_id = *o;
            t.game_id = game;
            ctx.db.treasury().insert(t);
        }
    }
    for mut p in saved.data.prices {
        p.id = 0;
        p.game_id = game;
        ctx.db.market_pressure().insert(p);
    }
    for mut o in saved.data.close_orders {
        if let (Some(b), Some(owner)) = (buildings.get(&o.building_id), owners.get(&o.owner)) {
            o.building_id = *b;
            o.owner = *owner;
            ctx.db.close_gate_order().insert(o);
        }
    }
}
fn claim_civic_owner(
    ctx: &ReducerContext,
    old: spacetimedb::ConnectionId,
    new: spacetimedb::ConnectionId,
) {
    if let Some(mut t) = ctx.db.treasury().connection_id().find(old) {
        ctx.db.treasury().connection_id().delete(old);
        t.connection_id = new;
        ctx.db.treasury().insert(t);
    }
    for mut o in ctx
        .db
        .close_gate_order()
        .iter()
        .filter(|o| o.owner == old)
        .collect::<Vec<_>>()
    {
        o.owner = new;
        ctx.db.close_gate_order().building_id().update(o);
    }
}
