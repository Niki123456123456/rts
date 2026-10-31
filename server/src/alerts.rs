// One live warning per attacked unit/building; refreshed by real damage only.
#[spacetimedb::table(name=attack_alert,public)]
pub struct AttackAlert {
    #[primary_key]
    pub id: u64,
    #[index(btree)]
    pub game_id: u64,
    pub owner: spacetimedb::ConnectionId,
    pub x: f32,
    pub z: f32,
    pub updated_at: i64,
}
fn alert_attack(
    ctx: &ReducerContext,
    game: u64,
    owner: spacetimedb::ConnectionId,
    target: u64,
    building: bool,
    x: f32,
    z: f32,
) {
    let id = target.saturating_mul(2) + u64::from(building);
    let row = AttackAlert {
        id,
        game_id: game,
        owner,
        x,
        z,
        updated_at: ctx.timestamp.to_micros_since_unix_epoch(),
    };
    if ctx.db.attack_alert().id().find(id).is_some() {
        ctx.db.attack_alert().id().update(row);
    } else {
        ctx.db.attack_alert().insert(row);
    }
}
fn clear_old_alerts(ctx: &ReducerContext) {
    let now = ctx.timestamp.to_micros_since_unix_epoch();
    for a in ctx
        .db
        .attack_alert()
        .iter()
        .filter(|a| now - a.updated_at > 3_000_000)
        .collect::<Vec<_>>()
    {
        ctx.db.attack_alert().id().delete(a.id);
    }
}
