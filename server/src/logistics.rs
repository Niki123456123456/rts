use map::logistics::{self as economy, Commodity};
#[derive(Clone)]
#[spacetimedb::table(name = warehouse_stock, public)]
pub struct WarehouseStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
}
impl WarehouseStock {
    fn amounts(&self) -> [u32; 3] {
        [self.wood, self.stone, self.iron]
    }
    fn add(&mut self, r: Commodity, n: u32) {
        match r {
            Commodity::Wood => self.wood += n,
            Commodity::Stone => self.stone += n,
            Commodity::Iron => self.iron += n,
        }
    }
}
#[spacetimedb::table(name = logistics_player)]
pub struct LogisticsPlayer {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
}
#[derive(Clone)]
#[spacetimedb::table(name = producer, public)]
pub struct Producer {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub resource: u8,
    pub buffer: u32,
    pub progress: u32,
}
#[derive(Clone)]
#[spacetimedb::table(name = worker, public)]
pub struct Worker {
    #[primary_key]
    pub unit_id: u64,
    #[index(btree)]
    pub game_id: u64,
    #[index(btree)]
    pub source_building: u64,
    pub resource: u8,
    pub cargo: u32,
    pub warehouse: Option<u64>,
    pub phase: u8,
    pub last_x: f32,
    pub last_z: f32,
    pub stalled: u32,
}
#[spacetimedb::table(name = production_tick, scheduled(produce_resources))]
pub struct ProductionTick {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}
fn start_production(ctx: &ReducerContext) {
    if ctx.db.producer().iter().next().is_some() && ctx.db.production_tick().iter().next().is_none()
    {
        ctx.db.production_tick().insert(ProductionTick {
            scheduled_id: 0,
            scheduled_at: Duration::from_secs(1).into(),
        });
    }
}
fn own_warehouses(
    ctx: &ReducerContext,
    game: u64,
    owner: spacetimedb::ConnectionId,
) -> Vec<WarehouseStock> {
    let mut stocks = ctx
        .db
        .warehouse_stock()
        .game_id()
        .filter(game)
        .filter(|s| {
            ctx.db
                .building()
                .id()
                .find(s.building_id)
                .is_some_and(|b| b.owner == owner)
        })
        .collect::<Vec<_>>();
    stocks.sort_by_key(|s| s.building_id);
    stocks
}
fn debit_stone_stock(
    ctx: &ReducerContext,
    player: &Participant,
    amount: u32,
) -> Result<(), String> {
    let mut stocks = own_warehouses(ctx, player.game_id, player.connection_id);
    if stocks.iter().map(|s| u64::from(s.stone)).sum::<u64>() < u64::from(amount) {
        return Err("Nicht genug Stein im Vorratslager.".into());
    }
    let mut left = amount;
    for stock in &mut stocks {
        let take = left.min(stock.stone);
        if take > 0 {
            stock.stone -= take;
            left -= take;
            ctx.db.warehouse_stock().building_id().update(stock.clone());
        }
        if left == 0 {
            break;
        }
    }
    Ok(())
}
fn access_route(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    b: &Building,
) -> Option<(map::Point, Vec<map::Point>)> {
    let obstacles = obstacles(ctx, u.game_id);
    let peers = ctx
        .db
        .army_unit()
        .game_id()
        .filter(u.game_id)
        .filter(|p| p.id != u.id)
        .flat_map(|p| ground_reservations(ctx, &p))
        .collect::<Vec<_>>();
    let mut points = map::attack_positions(building_shape(ctx, b))
        .into_iter()
        .filter(|p| p.ring == 0)
        .map(|p| p.point)
        .filter(|p| {
            map::free(*p, &obstacles)
                && peers
                    .iter()
                    .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
        })
        .collect::<Vec<_>>();
    points.sort_by(|a, b| map::distance((u.x, u.z), *a).total_cmp(&map::distance((u.x, u.z), *b)));
    points
        .into_iter()
        .find_map(|p| map::shortest_path((u.x, u.z), p, &obstacles).map(|path| (p, path)))
}
fn route_worker(ctx: &ReducerContext, u: &mut ArmyUnit, b: &Building) -> bool {
    let Some((point, path)) = access_route(ctx, u, b) else {
        return false;
    };
    u.target_x = point.0;
    u.target_z = point.1;
    save_order(ctx, u, None, path);
    ctx.db.army_unit().id().update(u.clone());
    start_tick(ctx);
    true
}
fn register_logistics_building(ctx: &ReducerContext, b: &Building) -> Result<(), String> {
    if b.kind == "stockpile" && ctx.db.warehouse_stock().building_id().find(b.id).is_none() {
        let first = ctx
            .db
            .logistics_player()
            .connection_id()
            .find(b.owner)
            .is_none();
        let gift = if first {
            map::STARTING_RESOURCE_AMOUNT
        } else {
            0
        };
        ctx.db.warehouse_stock().insert(WarehouseStock {
            building_id: b.id,
            game_id: b.game_id,
            wood: gift,
            stone: gift,
            iron: gift,
        });
        if first {
            // This marker survives demolition, so replacement/additional stockpiles
            // never grant the starting gift again during the same participation.
            ctx.db.logistics_player().insert(LogisticsPlayer {
                connection_id: b.owner,
            });
            let player = ctx
                .db
                .participant()
                .connection_id()
                .find(b.owner)
                .ok_or("Unknown warehouse owner.")?;
            let mut stock = ensure_resources(ctx, &player);
            stock.wood += gift;
            stock.stone += gift;
            stock.iron += gift;
            ctx.db.player_resources().connection_id().update(stock);
        }
    }
    let Some(resource) = map::BuildingKind::from_name(&b.kind).and_then(|k| k.produces()) else {
        return Ok(());
    };
    if ctx.db.producer().building_id().find(b.id).is_some() {
        return Ok(());
    }
    let shape = building_shape(ctx, b);
    let obs = obstacles(ctx, b.game_id);
    let reserved = ctx
        .db
        .army_unit()
        .game_id()
        .filter(b.game_id)
        .filter(|u| on_ground(ctx, u))
        .map(|u| (u.x, u.z))
        .collect::<Vec<_>>();
    let point = map::attack_positions(shape)
        .into_iter()
        .filter(|p| p.ring == 0)
        .map(|p| p.point)
        .find(|p| {
            map::free(*p, &obs)
                && reserved
                    .iter()
                    .all(|q| map::distance(*p, *q) >= map::UNIT_SPACING)
        })
        .ok_or("Kein Platz für den Transportbauern.")?;
    let u = ctx.db.army_unit().insert(ArmyUnit {
        id: 0,
        game_id: b.game_id,
        owner: b.owner,
        color: b.color,
        number: 0,
        health: 1.,
        x: point.0,
        y: terrain_height(point.0, point.1) + 0.75,
        z: point.1,
        target_x: point.0,
        target_z: point.1,
    });
    ctx.db.producer().insert(Producer {
        building_id: b.id,
        game_id: b.game_id,
        resource: resource as u8,
        buffer: 0,
        progress: 0,
    });
    ctx.db.worker().insert(Worker {
        unit_id: u.id,
        game_id: b.game_id,
        source_building: b.id,
        resource: resource as u8,
        cargo: 0,
        warehouse: None,
        phase: 0,
        last_x: u.x,
        last_z: u.z,
        stalled: 0,
    });
    start_production(ctx);
    Ok(())
}
fn remove_logistics_building(ctx: &ReducerContext, id: u64) {
    if let Some(stock) = ctx.db.warehouse_stock().building_id().find(id) {
        if let Some(b) = ctx.db.building().id().find(id) {
            if let Some(mut r) = ctx.db.player_resources().connection_id().find(b.owner) {
                r.wood = r.wood.saturating_sub(stock.wood);
                r.stone = r.stone.saturating_sub(stock.stone);
                r.iron = r.iron.saturating_sub(stock.iron);
                ctx.db.player_resources().connection_id().update(r);
            }
        }
        ctx.db.warehouse_stock().building_id().delete(id);
        for mut worker in ctx
            .db
            .worker()
            .iter()
            .filter(|w| w.warehouse == Some(id))
            .collect::<Vec<_>>()
        {
            worker.warehouse = None;
            ctx.db.unit_order().unit_id().delete(worker.unit_id);
            if let Some(u) = ctx.db.army_unit().id().find(worker.unit_id) {
                stop_unit(ctx, u);
            }
            ctx.db.worker().unit_id().update(worker);
        }
    }
    ctx.db.producer().building_id().delete(id);
    for worker in ctx
        .db
        .worker()
        .source_building()
        .filter(id)
        .collect::<Vec<_>>()
    {
        ctx.db.unit_order().unit_id().delete(worker.unit_id);
        ctx.db.army_unit().id().delete(worker.unit_id);
        ctx.db.worker().unit_id().delete(worker.unit_id);
    }
}
#[spacetimedb::reducer]
pub fn produce_resources(ctx: &ReducerContext, tick: ProductionTick) -> Result<(), String> {
    if ctx.sender != ctx.identity() {
        return Err("Production is server controlled.".into());
    }
    for mut p in ctx.db.producer().iter().collect::<Vec<_>>() {
        if p.buffer < economy::PRODUCTION_BUFFER {
            p.progress += 1;
            if p.progress >= economy::PRODUCTION_SECONDS {
                p.buffer += 1;
                p.progress = 0;
            }
            ctx.db.producer().building_id().update(p);
        }
    }
    for mut w in ctx.db.worker().iter().collect::<Vec<_>>() {
        let Some(mut u) = ctx.db.army_unit().id().find(w.unit_id) else {
            ctx.db.worker().unit_id().delete(w.unit_id);
            continue;
        };
        let Some(source) = ctx.db.building().id().find(w.source_building) else {
            continue;
        };
        let resource = Commodity::from_u8(w.resource).unwrap();
        let moving = ctx.db.unit_order().unit_id().find(u.id).is_some();
        if moving {
            if map::distance((u.x, u.z), (w.last_x, w.last_z)) < 0.02 {
                w.stalled += 1;
            } else {
                w.stalled = 0;
            }
            w.last_x = u.x;
            w.last_z = u.z;
            if w.stalled < 5 {
                ctx.db.worker().unit_id().update(w);
                continue;
            }
            stop_unit(ctx, u.clone());
            w.stalled = 0;
        }
        if w.cargo > 0 {
            let delivered = w
                .warehouse
                .and_then(|id| ctx.db.building().id().find(id))
                .filter(|b| b.owner == u.owner && b.game_id == u.game_id)
                .filter(|b| {
                    map::distance((u.x, u.z), building_shape(ctx, b).closest((u.x, u.z))) <= 1.2
                });
            if let Some(b) = delivered {
                if let Some(mut stock) = ctx
                    .db
                    .warehouse_stock()
                    .building_id()
                    .find(b.id)
                    .filter(|s| economy::can_store(s.amounts(), resource, w.cargo))
                {
                    stock.add(resource, w.cargo);
                    ctx.db.warehouse_stock().building_id().update(stock);
                    let mut balance = ctx
                        .db
                        .player_resources()
                        .connection_id()
                        .find(u.owner)
                        .unwrap();
                    match resource {
                        Commodity::Wood => balance.wood += w.cargo,
                        Commodity::Stone => balance.stone += w.cargo,
                        Commodity::Iron => balance.iron += w.cargo,
                    };
                    ctx.db.player_resources().connection_id().update(balance);
                    w.cargo = 0;
                    w.warehouse = None;
                    w.phase = 1;
                }
            }
            if w.cargo > 0 {
                let mut warehouses = own_warehouses(ctx, u.game_id, u.owner);
                warehouses.sort_by(|a, b| {
                    let a = ctx.db.building().id().find(a.building_id).unwrap();
                    let b = ctx.db.building().id().find(b.building_id).unwrap();
                    map::distance((u.x, u.z), (a.x, a.z))
                        .total_cmp(&map::distance((u.x, u.z), (b.x, b.z)))
                });
                for stock in warehouses
                    .into_iter()
                    .filter(|s| economy::can_store(s.amounts(), resource, w.cargo))
                {
                    let b = ctx.db.building().id().find(stock.building_id).unwrap();
                    if route_worker(ctx, &mut u, &b) {
                        w.warehouse = Some(b.id);
                        w.phase = 2;
                        break;
                    }
                }
                ctx.db.worker().unit_id().update(w);
                continue;
            }
        }
        if map::distance((u.x, u.z), building_shape(ctx, &source).closest((u.x, u.z))) > 1.2 {
            route_worker(ctx, &mut u, &source);
            w.phase = 1;
            ctx.db.worker().unit_id().update(w);
            continue;
        }
        w.phase = 0;
        if let Some(mut p) = ctx
            .db
            .producer()
            .building_id()
            .find(source.id)
            .filter(|p| p.buffer > 0)
        {
            let amount = p.buffer.min(economy::CARRY_CAPACITY);
            for stock in own_warehouses(ctx, u.game_id, u.owner)
                .into_iter()
                .filter(|s| economy::can_store(s.amounts(), resource, amount))
            {
                let target = ctx.db.building().id().find(stock.building_id).unwrap();
                if route_worker(ctx, &mut u, &target) {
                    p.buffer -= amount;
                    ctx.db.producer().building_id().update(p);
                    w.cargo = amount;
                    w.warehouse = Some(target.id);
                    w.phase = 2;
                    break;
                }
            }
        }
        ctx.db.worker().unit_id().update(w);
    }
    if ctx.db.producer().iter().next().is_none() {
        ctx.db
            .production_tick()
            .scheduled_id()
            .delete(tick.scheduled_id);
    }
    Ok(())
}
