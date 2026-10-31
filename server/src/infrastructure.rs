// Drag-built, nonblocking transport surfaces; bridge rows cut actual river obstacles.
fn build_transport(
    ctx: &ReducerContext,
    p: &Participant,
    k: map::BuildingKind,
    start: map::Point,
    end: map::Point,
) -> Result<(), String> {
    let line = map::walls::wall_line(start, end);
    if line.is_empty() || line.len() > 256 {
        return Err("Ziehe eine kürzere Strecke innerhalb der Karte.".into());
    }
    if ctx
        .db
        .building()
        .game_id()
        .filter(p.game_id)
        .filter(|b| map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_road()))
        .count()
        + line.len()
        > 1024
    {
        return Err("Maximal 1024 Wegabschnitte pro Karte.".into());
    }
    if k == map::BuildingKind::RomanBridge {
        let dry = |q: map::Point| map::terrain::biome(q.0, q.1) != map::terrain::Biome::River;
        if !dry(line[0]) || !dry(*line.last().unwrap()) {
            return Err("Ziehe die Brücke von einem trockenen Ufer zum anderen.".into());
        }
    }
    for (i, &(x, z)) in line.iter().enumerate() {
        let q = line
            .get(i + 1)
            .or_else(|| i.checked_sub(1).and_then(|j| line.get(j)))
            .copied()
            .unwrap_or((x + 2., z));
        let r = u8::from((q.1 - z).abs() > (q.0 - x).abs());
        let height = map::terrain::construction_height(k, x, z, r)
            .ok_or("Die Strecke ist hier nicht baubar.")?;
        let existing = ctx
            .db
            .building()
            .game_id()
            .filter(p.game_id)
            .filter(|b| {
                map::distance((b.x, b.z), (x, z)) < 0.2
                    && map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_road())
            })
            .collect::<Vec<_>>();
        if existing.iter().any(|b| b.owner != p.connection_id) {
            return Err("Fremde Straßen können nicht ersetzt werden.".into());
        }
        if existing.iter().any(|b| b.kind == k.name()) {
            continue;
        }
        let shape = k.construction_bounds(x, z, r);
        if ctx
            .db
            .building()
            .game_id()
            .filter(p.game_id)
            .filter_map(|b| map::BuildingKind::from_name(&b.kind).map(|kind| (b, kind)))
            .any(|(b, kind)| {
                !kind.is_road()
                    && kind
                        .ground_solids(b.x, b.z, rotation(ctx, b.id))
                        .iter()
                        .any(|s| s.overlaps(shape))
            })
        {
            return Err("Die Strecke überschneidet ein Gebäude oder eine Befestigung.".into());
        }
        debit_cost(ctx, p, k.cost())?;
        for b in existing {
            delete_building(ctx, b.id);
        }
        let b = ctx.db.building().insert(Building {
            id: 0,
            game_id: p.game_id,
            owner: p.connection_id,
            color: p.color,
            kind: k.name().into(),
            x,
            y: height,
            z,
            health: 1.,
        });
        ctx.db.building_rotation().insert(BuildingRotation {
            building_id: b.id,
            game_id: p.game_id,
            quarter_turns: r,
        });
        let clear = map::terrain::clearing_shape(k, x, z, r);
        clear_resource_trees(ctx, p.game_id, clear);
        ctx.db.terrain_clearing().insert(TerrainClearing {
            id: 0,
            game_id: p.game_id,
            x,
            z,
            half_x: clear.half_x,
            half_z: clear.half_z,
        });
    }
    replan(ctx, p.game_id);
    Ok(())
}
fn transport_speed(ctx: &ReducerContext, game: u64, point: map::Point) -> f32 {
    ctx.db
        .building()
        .game_id()
        .filter(game)
        .filter_map(|b| {
            map::BuildingKind::from_name(&b.kind)
                .filter(|k| k.is_road())
                .filter(|k| {
                    k.rotated_obstacle(b.x, b.z, rotation(ctx, b.id))
                        .contains(point, 0.)
                })
                .map(|k| k.travel_multiplier())
        })
        .fold(1., f32::max)
}
