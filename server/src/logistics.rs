use map::logistics::{self as economy, Commodity};
#[derive(Clone)]
#[spacetimedb::table(name = granary_stock, public)]
pub struct GranaryStock {
    #[primary_key]
    pub building_id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub owner: spacetimedb::ConnectionId,
    pub apples: u32,
    pub bread: u32,
    pub cheese: u32,
    pub meat: u32,
}
impl GranaryStock {
    fn food_amount(&self, resource: Commodity) -> Option<u32> {
        match resource {
            Commodity::Apples => Some(self.apples),
            Commodity::Cheese => Some(self.cheese),
            _ => None,
        }
    }
    fn can_store_food(&self, resource: Commodity, amount: u32) -> bool {
        self.food_amount(resource)
            .and_then(|n| n.checked_add(amount))
            .is_some_and(|n| n <= economy::FOOD_CAPACITY_PER_KIND)
    }
    fn add_food(&mut self, resource: Commodity, amount: u32) {
        match resource {
            Commodity::Apples => self.apples += amount,
            Commodity::Cheese => self.cheese += amount,
            _ => unreachable!("Not a food commodity"),
        }
    }
}
#[spacetimedb::table(name = granary_player)]
pub struct GranaryPlayer {
    #[primary_key]
    pub connection_id: spacetimedb::ConnectionId,
}
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
            Commodity::Apples | Commodity::Cheese => unreachable!("Food belongs in granaries"),
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
fn storage_targets(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    resource: Commodity,
    amount: u32,
) -> Vec<Building> {
    let ids: Vec<_> = if resource.is_food() {
        ctx.db
            .granary_stock()
            .game_id()
            .filter(u.game_id)
            .filter(|s| s.owner == u.owner && s.can_store_food(resource, amount))
            .map(|s| s.building_id)
            .collect()
    } else {
        own_warehouses(ctx, u.game_id, u.owner)
            .into_iter()
            .filter(|s| economy::can_store(s.amounts(), resource, amount))
            .map(|s| s.building_id)
            .collect()
    };
    let mut buildings = ids
        .into_iter()
        .filter_map(|id| ctx.db.building().id().find(id))
        .collect::<Vec<_>>();
    buildings.sort_by(|a, b| {
        map::distance((u.x, u.z), (a.x, a.z))
            .total_cmp(&map::distance((u.x, u.z), (b.x, b.z)))
            .then(a.id.cmp(&b.id))
    });
    buildings
}
fn deliver_resource(
    ctx: &ReducerContext,
    u: &ArmyUnit,
    id: u64,
    resource: Commodity,
    amount: u32,
) -> bool {
    if resource.is_food() {
        let Some(mut stock) = ctx.db.granary_stock().building_id().find(id).filter(|s| {
            s.owner == u.owner && s.game_id == u.game_id && s.can_store_food(resource, amount)
        }) else {
            return false;
        };
        stock.add_food(resource, amount);
        ctx.db.granary_stock().building_id().update(stock);
    } else {
        let Some(mut stock) = ctx
            .db
            .warehouse_stock()
            .building_id()
            .find(id)
            .filter(|s| economy::can_store(s.amounts(), resource, amount))
        else {
            return false;
        };
        stock.add(resource, amount);
        ctx.db.warehouse_stock().building_id().update(stock);
        let mut balance = ctx
            .db
            .player_resources()
            .connection_id()
            .find(u.owner)
            .unwrap();
        match resource {
            Commodity::Wood => balance.wood += amount,
            Commodity::Stone => balance.stone += amount,
            Commodity::Iron => balance.iron += amount,
            Commodity::Apples | Commodity::Cheese => unreachable!(),
        }
        ctx.db.player_resources().connection_id().update(balance);
    }
    true
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
    if b.kind == "granary" && ctx.db.granary_stock().building_id().find(b.id).is_none() {
        let first = ctx
            .db
            .granary_player()
            .connection_id()
            .find(b.owner)
            .is_none();
        let gift = if first {
            economy::STARTING_FOOD_AMOUNT
        } else {
            0
        };
        ctx.db.granary_stock().insert(GranaryStock {
            building_id: b.id,
            game_id: b.game_id,
            owner: b.owner,
            apples: gift,
            bread: gift,
            cheese: gift,
            meat: gift,
        });
        if first {
            ctx.db.granary_player().insert(GranaryPlayer {
                connection_id: b.owner,
            });
        }
    }
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
    ctx.db.producer().insert(Producer {
        building_id: b.id,
        game_id: b.game_id,
        resource: resource as u8,
        buffer: 0,
        progress: 0,
    });
    start_production(ctx);
    Ok(())
}
fn spawn_production_worker(
    ctx: &ReducerContext,
    b: &Building,
    point: map::Point,
) -> Result<(), String> {
    let resource = map::BuildingKind::from_name(&b.kind)
        .and_then(|k| k.produces())
        .ok_or("Kein Produktionsarbeitsplatz.")?;
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
    let removed_food_storage = ctx.db.granary_stock().building_id().delete(id);
    let removed_material_storage = ctx.db.warehouse_stock().building_id().find(id).is_some();
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
    }
    if removed_food_storage || removed_material_storage {
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
        if !building_open(ctx, p.building_id) {
            continue;
        }
        let staffed = ctx
            .db
            .worker()
            .source_building()
            .filter(p.building_id)
            .any(|w| {
                ctx.db.army_unit().id().find(w.unit_id).is_some_and(|u| {
                    ctx.db.building().id().find(p.building_id).is_some_and(|b| {
                        map::distance((u.x, u.z), building_shape(ctx, &b).closest((u.x, u.z)))
                            <= 1.2
                    })
                })
            });
        if !staffed {
            continue;
        }
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
                if deliver_resource(ctx, &u, b.id, resource, w.cargo) {
                    w.cargo = 0;
                    w.warehouse = None;
                    w.phase = 1;
                }
            }
            if w.cargo > 0 {
                for b in storage_targets(ctx, &u, resource, w.cargo) {
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
            let minimum = if resource.is_food() { 1 } else { amount };
            for target in storage_targets(ctx, &u, resource, minimum) {
                let amount = if resource.is_food() {
                    let stock = ctx
                        .db
                        .granary_stock()
                        .building_id()
                        .find(target.id)
                        .unwrap();
                    amount.min(
                        economy::FOOD_CAPACITY_PER_KIND.saturating_sub(
                            stock
                                .food_amount(resource)
                                .unwrap_or(economy::FOOD_CAPACITY_PER_KIND),
                        ),
                    )
                } else {
                    amount
                };
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
