use crate::*;
use stronghold_map::{self as map, BuildingKind};

#[derive(Resource, Default)]
pub(crate) struct BuildingInteraction {
    overlaps: Vec<(Vec3, Vec3)>,
    pub placing: Option<BuildingKind>,
    pub fort_tool: Option<u8>,
    pub fort_start: Option<map::Point>,
    pub rotation: u8,
    pub wall_start: Option<map::Point>,
    pub wall_cost: u32,
    pub category: Option<&'static str>,
    pub selected: Option<u64>,
    pub consumed: bool,
    attack_hover: Option<Vec2>,
    attack_error: Option<(Vec2, &'static str)>,
    enemy_group_preview: Vec<Vec3>,
    roof_hover: Option<(Vec2, usize, usize)>,
    roof_preview: Vec<Vec3>,
    roof_error: Option<&'static str>,
}
#[derive(Component)]
pub(crate) struct Building(pub network::Building);
#[derive(Component)]
pub(crate) struct BuildingCollider(u64);
#[derive(Component)]
pub(crate) struct BuildingOutline(u64);
struct BuildingModel {
    scene: Handle<WorldAsset>,
    collision: Handle<Mesh>,
    footprint: Handle<Mesh>,
    preview: Entity,
    preview_footprint: Entity,
}
#[derive(Resource)]
pub(crate) struct BuildingAssets {
    models: [Option<BuildingModel>; BuildingKind::ALL.len()],
    loading: Option<usize>,
    wall_previews: Vec<Entity>,
    invisible: Handle<StandardMaterial>,
    valid: Handle<StandardMaterial>,
    invalid: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    foundation_mesh: Handle<Mesh>,
    foundation_material: Handle<StandardMaterial>,
}
impl BuildingAssets {
    fn hide_previews(&self, commands: &mut Commands) {
        for entity in &self.wall_previews {
            commands.entity(*entity).insert(Visibility::Hidden);
        }
        for model in self.models.iter().flatten() {
            commands.entity(model.preview).insert(Visibility::Hidden);
            commands
                .entity(model.preview_footprint)
                .insert(Visibility::Hidden);
        }
    }
}
fn placement_transform(x: f32, y: f32, z: f32, rotation: u8) -> Transform {
    Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_y(
        (rotation % 4) as f32 * std::f32::consts::FRAC_PI_2,
    ))
}
fn model_transform(kind: BuildingKind) -> Transform {
    let center = kind.center();
    Transform::from_xyz(
        -center.0 * map::MODEL_SCALE,
        kind.ground_offset(),
        -center.1 * map::MODEL_SCALE,
    )
    .with_scale(Vec3::new(
        map::MODEL_SCALE,
        map::MODEL_SCALE
            * if kind == BuildingKind::TowerLookout {
                1.4
            } else {
                1.
            },
        map::MODEL_SCALE,
    ))
}
pub(crate) fn setup_buildings(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut material = |color| {
        materials.add(StandardMaterial {
            base_color: color,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })
    };
    let invisible = material(Color::srgba(0.0, 0.0, 0.0, 0.0));
    let valid = material(Color::srgba(0.1, 0.9, 0.3, 0.45));
    let invalid = material(Color::srgba(1.0, 0.1, 0.1, 0.45));
    let selected = material(Color::srgba(1.0, 0.85, 0.15, 0.45));
    commands.insert_resource(BuildingAssets {
        models: std::array::from_fn(|_| None),
        loading: None,
        wall_previews: Vec::new(),
        invisible,
        valid,
        invalid,
        selected,
        foundation_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        foundation_material: materials.add(Color::srgb(0.46, 0.43, 0.35)),
    });
}
/// Bevy's glTF texture scopes can re-enter other IO loads. Keep at most one
/// building's visual/collision pair active so nested scopes cannot grow with the catalog.
pub(crate) fn load_building_models(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut assets: ResMut<BuildingAssets>,
    interaction: Res<BuildingInteraction>,
    multiplayer: Res<Multiplayer>,
) {
    if let Some(index) = assets.loading {
        let model = assets.models[index].as_ref().unwrap();
        let loaded = server.is_loaded_with_dependencies(model.scene.id())
            && server.is_loaded_with_dependencies(model.collision.id());
        let failed = matches!(
            server.get_load_state(model.scene.id()),
            Some(bevy::asset::LoadState::Failed(_))
        ) || matches!(
            server.get_load_state(model.collision.id()),
            Some(bevy::asset::LoadState::Failed(_))
        );
        if !loaded && !failed {
            return;
        }
        assets.loading = None;
    }
    let index = interaction
        .placing
        .map(BuildingKind::index)
        .filter(|index| assets.models[*index].is_none())
        .or_else(|| {
            multiplayer
                .snapshot
                .buildings
                .iter()
                .filter_map(|building| BuildingKind::from_name(&building.kind))
                .map(BuildingKind::index)
                .find(|index| assets.models[*index].is_none())
        })
        .or_else(|| assets.models.iter().position(Option::is_none));
    let Some(index) = index else {
        return;
    };
    let kind = BuildingKind::ALL[index];
    let scene = server.load(kind.scene());
    let collision = server.load(kind.collision());
    let shape = kind.build_footprint(0.0, 0.0);
    let footprint = meshes.add(Cuboid::new(shape.half_x * 2.0, 0.04, shape.half_z * 2.0));
    let preview = commands
        .spawn((Transform::default(), Visibility::Hidden))
        .with_children(|parent| {
            parent.spawn((WorldAssetRoot(scene.clone()), model_transform(kind)));
        })
        .id();
    let preview_footprint = commands
        .spawn((
            Mesh3d(footprint.clone()),
            MeshMaterial3d(assets.valid.clone()),
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    assets.models[index] = Some(BuildingModel {
        scene,
        collision,
        footprint,
        preview,
        preview_footprint,
    });
    assets.loading = Some(index);
}
pub(crate) fn sync_buildings(
    mut commands: Commands,
    multiplayer: Res<Multiplayer>,
    mut buildings: Query<(Entity, &mut Building)>,
    assets: Res<BuildingAssets>,
    mut interaction: ResMut<BuildingInteraction>,
    mut previous_game: Local<Option<u64>>,
    outlines: Query<(Entity, &BuildingOutline)>,
) {
    let game = multiplayer
        .snapshot
        .game_id
        .filter(|_| multiplayer.snapshot.connected);
    if game != *previous_game {
        *interaction = default();
        *previous_game = game;
    }
    let mut remaining: std::collections::HashMap<_, _> = multiplayer
        .snapshot
        .buildings
        .iter()
        .filter(|_| game.is_some())
        .map(|b| (b.id, b))
        .collect();
    for (entity, mut building) in &mut buildings {
        if let Some(row) = remaining.remove(&building.0.id) {
            if building.0.kind != row.kind {
                commands.entity(entity).despawn();
                remaining.insert(row.id, row);
                continue;
            }
            if building.0.rotation != row.rotation {
                commands.entity(entity).insert(placement_transform(
                    row.x,
                    row.y,
                    row.z,
                    row.rotation,
                ));
            }
            building.0 = row.clone();
        } else {
            commands.entity(entity).despawn();
        }
    }
    for row in remaining.values() {
        let kind = BuildingKind::from_name(&row.kind).unwrap_or_default();
        let Some(model) = assets.models[kind.index()].as_ref() else {
            continue;
        };
        commands
            .spawn((
                Building((*row).clone()),
                placement_transform(row.x, row.y, row.z, row.rotation),
                Visibility::default(),
            ))
            .with_children(|parent| {
                let bottom = kind
                    .rotated_build_samples(row.x, row.z, row.rotation)
                    .map(|(px, pz)| terrain_height(px, pz))
                    .fold(row.y, f32::min);
                let depth = row.y - bottom;
                if depth > 0.05 {
                    let (width, length) = kind.grid_size();
                    parent.spawn((
                        Mesh3d(assets.foundation_mesh.clone()),
                        MeshMaterial3d(assets.foundation_material.clone()),
                        Transform::from_xyz(0.0, -depth * 0.5, 0.0).with_scale(Vec3::new(
                            width as f32 * map::BUILD_GRID_SIZE,
                            depth,
                            length as f32 * map::BUILD_GRID_SIZE,
                        )),
                    ));
                }
                parent
                    .spawn((
                        WorldAssetRoot(model.scene.clone()),
                        model_transform(kind),
                        crate::tower_mesh::TowerRoot(row.id),
                    ))
                    .observe(crate::tower_mesh::ready);
                parent.spawn((
                    BuildingCollider(row.id),
                    Mesh3d(model.collision.clone()),
                    MeshMaterial3d(assets.invisible.clone()),
                    model_transform(kind),
                ));
                parent.spawn((
                    BuildingOutline(row.id),
                    Mesh3d(model.footprint.clone()),
                    MeshMaterial3d(assets.selected.clone()),
                    Transform::from_xyz(0.0, 0.04, 0.0),
                    Visibility::Hidden,
                ));
            });
    }
    if interaction
        .selected
        .is_some_and(|id| !multiplayer.snapshot.buildings.iter().any(|b| b.id == id))
    {
        interaction.selected = None;
    }
    for (entity, outline) in &outlines {
        // Parent despawn also removes its children when commands apply. Never
        // queue a later update for an outline belonging to a removed building.
        if game.is_none()
            || !multiplayer
                .snapshot
                .buildings
                .iter()
                .any(|b| b.id == outline.0)
        {
            continue;
        }
        commands
            .entity(entity)
            .insert(if interaction.selected == Some(outline.0) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
    }
    if interaction.placing.is_none() {
        assets.hide_previews(&mut commands);
    }
}
fn placement_valid(
    kind: BuildingKind,
    x: f32,
    z: f32,
    _y: f32,
    rotation: u8,
    multiplayer: &Multiplayer,
) -> bool {
    let required = match kind {
        BuildingKind::Quarry => Some(1),
        BuildingKind::IronMine => Some(2),
        BuildingKind::Fishpond => Some(7),
        _ => None,
    };
    if required.is_some_and(|k| {
        !multiplayer.snapshot.resource_nodes.iter().any(|n| {
            n.kind == k
                && n.remaining > 0
                && map::distance((x, z), (n.x, n.z)) < if k == 7 { 34. } else { 17. }
        })
    }) {
        return false;
    }
    let shape = kind.construction_bounds(x, z, rotation);
    let candidate = crate::fortifications::candidate(&multiplayer.snapshot, kind, x, z, rotation);
    let refund = crate::fortifications::refund(&multiplayer.snapshot, candidate);
    let mut planned = crate::fortifications::pieces(&multiplayer.snapshot);
    planned.push(candidate);
    let extensions = map::fortifications::joins(&planned)
        .into_iter()
        .filter(|j| j.a == 0 || j.b == 0)
        .collect::<Vec<_>>();
    shape.in_build_area()
        && (kind == BuildingKind::DrawbridgeGate
            || !multiplayer.snapshot.moats.iter().any(|m| {
                map::Obstacle {
                    x: m.x,
                    z: m.z,
                    half_x: 2.,
                    half_z: 2.,
                }
                .overlaps(shape)
            }))
        && !multiplayer.snapshot.stairs.iter().any(|s| {
            map::fortifications::stair_bounds((s.base_x, s.base_z), (s.top_x, s.top_z))
                .overlaps(shape)
        })
        && !(kind == BuildingKind::Keep
            && multiplayer
                .snapshot
                .buildings
                .iter()
                .any(|b| b.owned && b.kind == "keep"))
        && multiplayer.snapshot.resources.is_some_and(|s| {
            kind.cost().affordable(
                s.wood.saturating_add(refund.wood),
                s.stone.saturating_add(refund.stone),
                s.iron.saturating_add(refund.iron),
            )
        })
        && !multiplayer.snapshot.buildings.iter().any(|b| {
            shape.overlaps(
                BuildingKind::from_name(&b.kind)
                    .unwrap_or_default()
                    .construction_bounds(b.x, b.z, b.rotation),
            ) && !(b.owned
                && (map::fortifications::replaced_wall(candidate, crate::fortifications::piece(b))
                    || map::fortifications::may_join(candidate, crate::fortifications::piece(b))))
        })
        && !multiplayer.snapshot.units.iter().any(|u| {
            ((u.y - terrain_height(u.x, u.z) - map::building_roof::UNIT_CENTER_HEIGHT).abs() < 0.1
                && (kind.rotated_blocks_construction_unit(x, z, rotation, (u.x, u.z))
                    || extensions
                        .iter()
                        .any(|j| j.shape.contains((u.x, u.z), map::CLEARANCE))))
                || multiplayer.snapshot.buildings.iter().any(|b| {
                    map::fortifications::replaced_wall(candidate, crate::fortifications::piece(b))
                        && BuildingKind::from_name(&b.kind)
                            .and_then(|k| k.roof_profile())
                            .is_some_and(|p| {
                                (u.y - b.y - p.height - map::building_roof::UNIT_CENTER_HEIGHT)
                                    .abs()
                                    < 0.12
                                    && BuildingKind::from_name(&b.kind)
                                        .unwrap()
                                        .rotated_build_footprint(b.x, b.z, b.rotation)
                                        .contains((u.x, u.z), 0.)
                            })
                })
        })
        && map::terrain::construction_height(kind, x, z, rotation).is_some()
}

// Keep this identical to the server's terrain surface for the placement preview.
pub(crate) fn terrain_height(x: f32, z: f32) -> f32 {
    stronghold_map::terrain::height(x, z)
}
pub(crate) fn building_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mut ray_cast: MeshRayCast,
    units: Query<(Entity, &Unit)>,
    colliders: Query<&BuildingCollider>,
    ground: Query<(), With<Ground>>,
    mut multiplayer: ResMut<Multiplayer>,
    mut selected: ResMut<SelectedUnits>,
    mut drag: ResMut<SelectionDrag>,
    panel: Res<SelectionPanel>,
    mut contexts: EguiContexts,
    mut state: ResMut<BuildingInteraction>,
    mut assets: ResMut<BuildingAssets>,
) {
    state.overlaps.clear();
    state.consumed = state.fort_tool.is_some();
    if state.fort_tool.is_some() {
        return;
    }
    state.attack_hover = None;
    state.attack_error = None;
    state.enemy_group_preview.clear();
    state.roof_hover = None;
    state.roof_error = None;
    state.roof_preview.clear();
    assets.hide_previews(&mut commands);
    if multiplayer.snapshot.game_id.is_none() || !multiplayer.snapshot.connected {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.placing = None;
        state.wall_start = None;
        state.rotation = 0;
        state.selected = None;
        state.consumed = true;
        *drag = default();
        return;
    }
    if state.placing.is_some() && mouse.just_pressed(MouseButton::Right) {
        state.placing = None;
        state.wall_start = None;
        state.consumed = true;
        *drag = default();
        return;
    }
    if !window.focused {
        return;
    }
    if keys.just_pressed(KeyCode::Delete)
        && state.placing.is_none()
        && !contexts
            .ctx_mut()
            .is_ok_and(|ctx| ctx.egui_wants_keyboard_input())
    {
        if let Some(id) = state.selected.filter(|id| {
            multiplayer
                .snapshot
                .buildings
                .iter()
                .any(|b| b.id == *id && b.owned)
        }) {
            multiplayer.send(Action::RemoveBuilding { building_id: id });
            state.selected = None;
            state.consumed = true;
            return;
        }
    }
    if state.placing.is_some() {
        *drag = default();
        if keys.just_pressed(KeyCode::KeyR)
            && !contexts
                .ctx_mut()
                .is_ok_and(|ctx| ctx.egui_wants_keyboard_input())
        {
            state.rotation = (state.rotation + 1) % 4;
        }
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if contexts.ctx_mut().is_ok_and(|ctx| {
        let pos = egui::pos2(cursor.x, cursor.y) * (window.scale_factor() / ctx.pixels_per_point());
        ctx.egui_wants_pointer_input()
            || panel.0.is_some_and(|r| r.contains(pos))
            || panel.1.is_some_and(|r| r.contains(pos))
    }) {
        return;
    }
    if state.placing.is_some() && mouse.just_pressed(MouseButton::Right) {
        state.placing = None;
        state.wall_start = None;
        state.consumed = true;
        return;
    }
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    // The keep's full-height proxy box encloses its open terrace. Let visible
    // roof occupants receive the left click before selecting that proxy building.
    if state.placing.is_none() && mouse.just_pressed(MouseButton::Left) {
        let unit_filter = |e| units.contains(e) || ground.contains(e);
        let occupant = ray_cast
            .cast_ray(
                ray,
                &MeshRayCastSettings::default().with_filter(&unit_filter),
            )
            .first()
            .map(|(e, _)| *e)
            .filter(|e| {
                units.get(*e).is_ok_and(|(_, u)| {
                    multiplayer
                        .snapshot
                        .roof_occupants
                        .iter()
                        .any(|o| o.unit_id == u.id)
                })
            });
        if occupant.is_some() {
            state.selected = None;
            return;
        }
    }
    let filter = |e| {
        ground.contains(e)
            || (state.placing.is_none() && (colliders.contains(e) || units.contains(e)))
    };
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let Some((entity, hit)) = ray_cast.cast_ray(ray, &settings).first() else {
        return;
    };
    if let Some(kind) = state.placing {
        if kind.is_wall() {
            let end = kind.snap_position(hit.point.x, hit.point.z);
            let start = state.wall_start.unwrap_or(end);
            let line = map::walls::wall_line(start, end);
            let existing = |x: f32, z: f32| {
                multiplayer.snapshot.buildings.iter().any(|b| {
                    b.owned
                        && map::fortifications::covered_wall(
                            (x, z),
                            crate::fortifications::piece(b),
                        )
                })
            };
            let wall_count = multiplayer
                .snapshot
                .buildings
                .iter()
                .filter(|b| BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_wall()))
                .count();
            state.wall_cost = line.iter().filter(|p| !existing(p.0, p.1)).count() as u32;
            let valid = !line.is_empty()
                && resources::population(&multiplayer).can_build(
                    kind,
                    line.iter().filter(|p| !existing(p.0, p.1)).count() as u32,
                )
                && multiplayer.snapshot.resources.is_some_and(|s| {
                    let c = kind.cost();
                    state.wall_cost == 0
                        || c.affordable(
                            s.wood / state.wall_cost.max(1),
                            s.stone / state.wall_cost.max(1),
                            s.iron / state.wall_cost.max(1),
                        )
                })
                && wall_count + line.iter().filter(|p| !existing(p.0, p.1)).count()
                    <= map::walls::MAX_WALL_SEGMENTS
                && line.iter().all(|&(x, z)| {
                    existing(x, z)
                        || placement_valid(kind, x, z, terrain_height(x, z), 0, &multiplayer)
                });
            let material = if valid {
                assets.valid.clone()
            } else {
                assets.invalid.clone()
            };
            let Some(model) = assets.models[kind.index()].as_ref() else {
                return;
            };
            let collision = model.collision.clone();
            for (index, &(x, z)) in line.iter().filter(|p| !existing(p.0, p.1)).enumerate() {
                state.overlaps.extend(overlap_regions(
                    kind,
                    x,
                    z,
                    terrain_height(x, z),
                    0,
                    &multiplayer,
                ));
                if index >= assets.wall_previews.len() {
                    let entity = commands
                        .spawn((
                            Mesh3d(collision.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::default(),
                            Visibility::Hidden,
                        ))
                        .id();
                    assets.wall_previews.push(entity);
                }
                let mut preview_model = model_transform(kind);
                if kind.is_crenellated() {
                    let p = map::fortifications::Structure {
                        id: u64::MAX,
                        kind,
                        x,
                        y: terrain_height(x, z),
                        z,
                        rotation: 0,
                        owner: multiplayer
                            .snapshot
                            .buildings
                            .iter()
                            .find(|b| b.owned)
                            .map_or(0, |b| b.color),
                    };
                    let pieces = crate::fortifications::pieces(&multiplayer.snapshot);
                    let actual = map::fortifications::crenellation_kind(p, &pieces);
                    preview_model.scale.y *= actual.height() / kind.height();
                }
                commands.entity(assets.wall_previews[index]).insert((
                    Visibility::Visible,
                    placement_transform(
                        x,
                        map::terrain::construction_height(kind, x, z, 0)
                            .unwrap_or_else(|| terrain_height(x, z))
                            + 0.01,
                        z,
                        0,
                    )
                    .mul_transform(preview_model),
                    MeshMaterial3d(material.clone()),
                ));
            }
            if mouse.just_pressed(MouseButton::Left) {
                state.consumed = true;
                if state.wall_start.is_none() {
                    if valid {
                        state.wall_start = Some(end);
                    }
                } else if valid
                    && multiplayer.send(Action::BuildWallKind {
                        kind: kind.name().into(),
                        start_x: start.0,
                        start_z: start.1,
                        end_x: end.0,
                        end_z: end.1,
                    })
                {
                    state.wall_start = None;
                }
            }
            return;
        }
        let Some(model) = assets.models[kind.index()].as_ref() else {
            return;
        };
        let neighbours = multiplayer
            .snapshot
            .buildings
            .iter()
            .filter_map(|b| Some((BuildingKind::from_name(&b.kind)?, b.x, b.z, b.rotation)))
            .collect::<Vec<_>>();
        let (x, z) = map::placement::snap(
            kind,
            (hit.point.x, hit.point.z),
            state.rotation,
            &neighbours,
        );
        let y = map::terrain::construction_height(kind, x, z, state.rotation)
            .unwrap_or_else(|| terrain_height(x, z));
        let valid = placement_valid(kind, x, z, y, state.rotation, &multiplayer);
        state.overlaps = overlap_regions(kind, x, z, y, state.rotation, &multiplayer);
        commands.entity(model.preview).insert((
            Visibility::Visible,
            placement_transform(x, y, z, state.rotation),
        ));
        commands.entity(model.preview_footprint).insert((
            Visibility::Visible,
            placement_transform(x, y + 0.05, z, state.rotation),
            MeshMaterial3d(if valid {
                assets.valid.clone()
            } else {
                assets.invalid.clone()
            }),
        ));
        if mouse.just_pressed(MouseButton::Left) {
            state.consumed = true;
            let action = Action::Build {
                x,
                z,
                building: kind.name().into(),
                rotation: state.rotation,
            };
            if valid
                && multiplayer.send(action)
                && kind.category() != "supply"
                && kind != BuildingKind::OxTether
            {
                state.placing = None;
            }
        }
        return;
    }
    // Combat units take precedence over the roof or ground underneath them.
    if let Ok((_, target)) = units.get(*entity) {
        if !target.owned && !target.worker && !selected.0.is_empty() {
            let attack_ids = units
                .iter()
                .filter(|(e, u)| u.owned && selected.0.contains(e))
                .map(|(_, u)| u.id)
                .collect::<Vec<_>>();
            if !enemy_surface_accessible(&multiplayer.snapshot, &attack_ids, target.id) {
                state.attack_error = Some((
                    cursor,
                    "Kein freier oder erreichbarer Turmzugang zur Besatzung",
                ));
                if mouse.just_pressed(MouseButton::Right) {
                    state.consumed = true;
                    *drag = default();
                }
                return;
            }
            let members = multiplayer
                .snapshot
                .units
                .iter()
                .filter(|u| !u.worker)
                .map(|u| map::combat::GroupMember {
                    id: u.id,
                    side: u.color,
                    point: (u.x, u.z),
                    height: u.y,
                })
                .collect::<Vec<_>>();
            let group = map::combat::enemy_group(target.id, &members);
            state.attack_hover = Some(cursor);
            state.enemy_group_preview = multiplayer
                .snapshot
                .units
                .iter()
                .filter(|u| group.contains(&u.id))
                .map(|u| Vec3::new(u.x, u.y - 0.71, u.z))
                .collect();
            if mouse.just_pressed(MouseButton::Right) {
                let unit_ids = units
                    .iter()
                    .filter(|(e, u)| u.owned && selected.0.contains(e))
                    .map(|(_, u)| u.id)
                    .collect::<Vec<_>>();
                if !unit_ids.is_empty() {
                    state.consumed = true;
                    *drag = default();
                    multiplayer.send(Action::AttackUnits {
                        unit_ids,
                        target_unit_id: target.id,
                    });
                }
            }
            return;
        }
    }
    // Pick the actual roof plane rather than the solid ground-level collision box.
    let roof_units = units
        .iter()
        .filter(|(e, u)| u.owned && selected.0.contains(e))
        .map(|(_, u)| u.id)
        .collect::<Vec<_>>();
    if !roof_units.is_empty() {
        let target = multiplayer
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.owned)
            .filter_map(|b| roof_ray_hit(ray, b).map(|(distance, point)| (b, distance, point)))
            .filter(|(building, distance, _)| {
                *distance <= hit.distance + 0.08
                    || units.contains(*entity)
                    || colliders.get(*entity).is_ok_and(|c| c.0 == building.id)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((building, _, point)) = target {
            let id = building.id;
            let tower = roof_geometry(building).unwrap();
            let accessible = !BuildingKind::from_name(&building.kind).is_some_and(|k| k.is_wall())
                || multiplayer.snapshot.stairs.iter().any(|s| {
                    map::walls::wall_component(id, &owned_roof_pieces(&multiplayer.snapshot))
                        .iter()
                        .any(|p| p.id == s.wall_id)
                })
                || !map::walls::tower_routes(id, &owned_roof_pieces(&multiplayer.snapshot))
                    .is_empty();
            if !accessible {
                state.roof_error = Some("Mauer zuerst mit einem eigenen Turm verbinden");
            }
            let pieces = owned_roof_pieces(&multiplayer.snapshot);
            let surfaces = if BuildingKind::from_name(&building.kind).is_some_and(|k| k.is_wall()) {
                map::walls::wall_component(id, &pieces)
                    .into_iter()
                    .map(|p| (p.id, p.roof()))
                    .collect::<Vec<_>>()
            } else {
                vec![(id, tower)]
            };
            let mut slots = surfaces
                .iter()
                .flat_map(|(id, roof)| (0..roof.capacity()).map(move |slot| (*id, *roof, slot)))
                .collect::<Vec<_>>();
            let capacity = slots.len();
            slots.sort_by(|a, b| {
                map::distance(a.1.slot(a.2), (point.x, point.z))
                    .total_cmp(&map::distance(b.1.slot(b.2), (point.x, point.z)))
                    .then(a.0.cmp(&b.0))
                    .then(a.2.cmp(&b.2))
            });
            let free = slots
                .into_iter()
                .filter(|(id, _, slot)| {
                    !multiplayer.snapshot.roof_occupants.iter().any(|o| {
                        !roof_units.contains(&o.unit_id)
                            && ((o.building_id == *id && o.slot as usize == *slot)
                                || (o.next_building == Some(*id) && o.next_slot as usize == *slot))
                    })
                })
                .take(if accessible { roof_units.len() } else { 0 })
                .collect::<Vec<_>>();
            state.roof_hover = Some((cursor, free.len(), capacity));
            state.roof_preview = free
                .iter()
                .map(|(_, roof, slot)| {
                    let p = roof.slot(*slot);
                    Vec3::new(p.0, roof.roof_y() + 0.08, p.1)
                })
                .collect();
            if mouse.just_pressed(MouseButton::Right) {
                state.consumed = true;
                *drag = default();
                if accessible {
                    multiplayer.send(Action::MoveToRoof {
                        unit_ids: roof_units,
                        building_id: id,
                        x: point.x,
                        z: point.z,
                    });
                }
                return;
            }
        }
    }
    if let Ok(collider) = colliders.get(*entity) {
        let Some(building) = multiplayer
            .snapshot
            .buildings
            .iter()
            .find(|b| b.id == collider.0)
        else {
            return;
        };
        let (id, owned) = (building.id, building.owned);
        if !owned
            && !drag.dragging
            && !mouse.pressed(MouseButton::Left)
            && units
                .iter()
                .any(|(entity, unit)| unit.owned && selected.0.contains(&entity))
        {
            state.attack_hover = Some(cursor);
        }
        let select_building = mouse.just_pressed(MouseButton::Left);
        let attack_building = mouse.just_pressed(MouseButton::Right);
        if select_building || attack_building {
            state.consumed = true;
            *drag = default();
            if select_building {
                state.selected = Some(id);
                selected.0.clear();
            } else if !owned {
                let unit_ids = units
                    .iter()
                    .filter(|(e, u)| u.owned && selected.0.contains(e))
                    .map(|(_, u)| u.id)
                    .collect::<Vec<_>>();
                if !unit_ids.is_empty() {
                    multiplayer.send(Action::AttackBuilding {
                        unit_ids,
                        building_id: id,
                    });
                }
            }
        }
    } else if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
        state.selected = None;
    }
}

fn owned_roof_pieces(snapshot: &network::Snapshot) -> Vec<map::walls::RoofPiece> {
    snapshot
        .buildings
        .iter()
        .filter(|b| b.owned)
        .filter_map(|b| {
            Some(map::walls::RoofPiece {
                id: b.id,
                kind: BuildingKind::from_name(&b.kind).filter(|k| k.roof_profile().is_some())?,
                x: b.x,
                y: b.y,
                z: b.z,
                rotation: b.rotation,
            })
        })
        .collect()
}
pub(crate) fn roof_geometry(
    building: &network::Building,
) -> Option<map::building_roof::BuildingRoof> {
    map::building_roof::BuildingRoof::new(
        BuildingKind::from_name(&building.kind)?,
        building.x,
        building.y,
        building.z,
        building.rotation,
    )
}
fn roof_ray_hit(ray: Ray3d, building: &network::Building) -> Option<(f32, Vec3)> {
    if ray.direction.y.abs() < 0.00001 {
        return None;
    }
    let tower = roof_geometry(building)?;
    let distance = (tower.roof_y() - ray.origin.y) / ray.direction.y;
    if distance < 0.0 {
        return None;
    }
    let point = ray.get_point(distance);
    tower
        .roof_contains((point.x, point.z))
        .then_some((distance, point))
}
pub(crate) fn roof_preview(interaction: Res<BuildingInteraction>, mut gizmos: Gizmos) {
    for point in &interaction.enemy_group_preview {
        gizmos.circle(
            Isometry3d::new(*point, Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            0.36,
            Color::srgb(1., 0.2, 0.1),
        );
    }
    for point in &interaction.roof_preview {
        gizmos.circle(
            Isometry3d::new(*point, Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            0.22,
            Color::srgb(0.3, 1.0, 0.5),
        );
    }
}

pub(crate) fn attack_hover_ui(
    mut contexts: EguiContexts,
    window: Single<&Window, With<PrimaryWindow>>,
    interaction: Res<BuildingInteraction>,
    selected: Res<SelectedUnits>,
) -> Result {
    if let Some((cursor, error)) = interaction.attack_error {
        let ctx = contexts.ctx_mut()?;
        let scale = window.scale_factor() / ctx.pixels_per_point();
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("unit_attack_error"),
        ))
        .text(
            egui::pos2(cursor.x * scale + 20., cursor.y * scale - 25.),
            egui::Align2::LEFT_BOTTOM,
            error,
            egui::FontId::proportional(16.),
            egui::Color32::LIGHT_RED,
        );
        return Ok(());
    }
    if let Some((cursor, count, capacity)) = interaction.roof_hover {
        let ctx = contexts.ctx_mut()?;
        let cursor = cursor * (window.scale_factor() / ctx.pixels_per_point());
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("roof_hover"),
        ));
        painter.text(
            egui::pos2(cursor.x + 20., cursor.y - 25.),
            egui::Align2::LEFT_BOTTOM,
            if let Some(error) = interaction.roof_error {
                error.into()
            } else if count == 0 {
                "Dach voll".into()
            } else {
                format!("Rechtsklick: {count} aufs Dach (max. {capacity})")
            },
            egui::FontId::proportional(16.),
            egui::Color32::LIGHT_GREEN,
        );
        return Ok(());
    }
    let Some(cursor) = interaction.attack_hover.filter(|_| !selected.0.is_empty()) else {
        return Ok(());
    };
    let ctx = contexts.ctx_mut()?;
    let scale = window.scale_factor() / ctx.pixels_per_point();
    let viewport = ctx.viewport_rect();
    let cursor = egui::pos2(cursor.x, cursor.y) * scale;
    let center = egui::pos2(
        (cursor.x + 32.0).clamp(
            viewport.left() + 90.0,
            viewport.right().max(viewport.left() + 180.0) - 90.0,
        ),
        (cursor.y - 32.0).max(viewport.top() + 20.0),
    );
    // A painter overlay cannot intercept the right click that issues the order.
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("building_attack_hover"),
    ));
    painter.circle_filled(center, 19.0, egui::Color32::from_rgb(70, 18, 18));
    painter.circle_stroke(
        center,
        19.0,
        egui::Stroke::new(2.0, egui::Color32::LIGHT_RED),
    );
    if !interaction.enemy_group_preview.is_empty() {
        painter.text(
            center + egui::vec2(0., 24.),
            egui::Align2::CENTER_TOP,
            format!("{} Gegner", interaction.enemy_group_preview.len()),
            egui::FontId::proportional(14.),
            egui::Color32::LIGHT_RED,
        );
    }
    // Crossed swords, drawn as geometry so no platform-specific emoji font is needed.
    for direction in [egui::vec2(1.0, -1.0), egui::vec2(-1.0, -1.0)] {
        let direction = direction.normalized();
        let side = egui::vec2(-direction.y, direction.x);
        painter.add(egui::Shape::convex_polygon(
            vec![
                center + direction * 14.0,
                center - direction * 4.0 + side * 2.0,
                center - direction * 4.0 - side * 2.0,
            ],
            egui::Color32::WHITE,
            egui::Stroke::NONE,
        ));
        painter.line_segment(
            [center - direction * 6.0, center - direction * 13.0],
            egui::Stroke::new(3.0, egui::Color32::GOLD),
        );
        painter.line_segment(
            [
                center - direction * 5.0 - side * 5.0,
                center - direction * 5.0 + side * 5.0,
            ],
            egui::Stroke::new(2.5, egui::Color32::GOLD),
        );
    }
    Ok(())
}
pub(crate) fn building_button(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    kind: BuildingKind,
    active: bool,
) -> bool {
    menu_ui::building_button(ui, texture, kind, active)
        .on_hover_ui(|ui| {
            ui.strong(kind.label());
            resources::cost_icons(ui, texture, kind.cost());
            ui.small(if kind.is_wall() {
                "Linksklick: Start und Ende · Rechtsklick / Esc: abbrechen"
            } else {
                "Linksklick: bauen · R: drehen · Rechtsklick / Esc: abbrechen"
            });
        })
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin, mesh::MeshPlugin};
    use std::time::{Duration, Instant};

    #[test]
    fn roof_click_uses_elevated_plane_for_every_tower_rotation() {
        for (kind, height) in [
            ("tower_stairwell", 6.663),
            ("tower_lookout", 21.7182),
            ("tower_square", 14.399),
        ] {
            for rotation in 0..4 {
                let row = network::Building {
                    id: 1,
                    kind: kind.into(),
                    rotation,
                    color: 0,
                    health: 1.,
                    owned: true,
                    x: 10.,
                    y: 3.,
                    z: 20.,
                };
                let ray = Ray3d::new(Vec3::new(10., 50., 20.), Dir3::NEG_Y);
                let (_, point) = roof_ray_hit(ray, &row).unwrap();
                assert!((point.y - (height + 3.)).abs() < 0.001);
                assert!(
                    roof_ray_hit(Ray3d::new(Vec3::new(15., 50., 20.), Dir3::NEG_Y), &row).is_none()
                );
                assert!(
                    roof_ray_hit(Ray3d::new(Vec3::new(10., 50., 20.), Dir3::X), &row).is_none()
                );
            }
        }
    }

    #[test]
    fn replicated_destruction_despawns_building_children_without_stale_commands() {
        let mut app = App::new();
        let models = BuildingKind::ALL.map(|_| {
            Some(BuildingModel {
                scene: default(),
                collision: default(),
                footprint: default(),
                preview: app.world_mut().spawn_empty().id(),
                preview_footprint: app.world_mut().spawn_empty().id(),
            })
        });
        app.insert_resource(BuildingAssets {
            models,
            loading: None,
            wall_previews: Vec::new(),
            invisible: default(),
            valid: default(),
            invalid: default(),
            selected: default(),
            foundation_mesh: default(),
            foundation_material: default(),
        })
        .init_resource::<Multiplayer>()
        .init_resource::<BuildingInteraction>()
        .add_systems(Update, sync_buildings);
        let row = network::Building {
            kind: "granary".into(),
            rotation: 0,
            id: 1,
            color: 0,
            health: 1.0,
            owned: true,
            x: 0.0,
            y: 0.0,
            z: 9.0,
        };
        let parent = app
            .world_mut()
            .spawn(Building(row.clone()))
            .with_children(|parent| {
                parent.spawn(BuildingOutline(1));
            })
            .id();
        {
            let mut network = app.world_mut().resource_mut::<Multiplayer>();
            network.snapshot.connected = true;
            network.snapshot.game_id = Some(1);
            network.snapshot.buildings.push(row);
        }
        app.update();
        // Orientation can arrive after the building row through its separate subscription.
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings[0]
            .rotation = 1;
        app.update();
        let transform = app.world().get::<Transform>(parent).unwrap();
        assert!((transform.rotation * Vec3::X).distance(Vec3::NEG_Z) < 0.0001);
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings
            .clear();
        app.update();
        assert!(app.world().get_entity(parent).is_err());
        assert_eq!(
            app.world_mut()
                .query::<&BuildingOutline>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn startup_loads_full_catalog_with_only_one_building_pair_in_flight() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            },
            MeshPlugin,
            ImagePlugin::default(),
            GltfPlugin::default(),
        ))
        .init_asset::<WorldAsset>()
        .init_asset::<StandardMaterial>()
        .init_resource::<Multiplayer>()
        .insert_resource(BuildingInteraction {
            placing: Some(BuildingKind::Treasury),
            ..default()
        })
        .add_systems(Startup, setup_buildings)
        .add_systems(Update, load_building_models);
        app.finish();
        app.cleanup();
        app.update();
        {
            let models = app.world().resource::<BuildingAssets>();
            assert_eq!(
                models.loading,
                Some(BuildingKind::Treasury.index()),
                "The selected building loads first"
            );
            assert_eq!(
                models.models.iter().flatten().count(),
                1,
                "Startup must not enqueue every GLB at once"
            );
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            app.update();
            let models = app.world().resource::<BuildingAssets>();
            let server = app.world().resource::<AssetServer>();
            let pending = models
                .models
                .iter()
                .flatten()
                .filter(|model| {
                    !server.is_loaded_with_dependencies(model.scene.id())
                        || !server.is_loaded_with_dependencies(model.collision.id())
                })
                .count();
            assert!(
                pending <= 1,
                "Only one building's visual/collision pair may be loading"
            );
            if models.loading.is_none() && models.models.iter().all(Option::is_some) {
                assert_eq!(
                    pending, 0,
                    "Every model, collision and embedded texture loaded"
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Startup asset queue did not finish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            app.world_mut()
                .query::<&WorldAssetRoot>()
                .iter(app.world())
                .count(),
            BuildingKind::ALL.len()
        );
    }

    #[test]
    fn packaged_building_assets_load_and_collision_matches_navigation() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            },
            MeshPlugin,
            ImagePlugin::default(),
            GltfPlugin::default(),
        ))
        .init_asset::<WorldAsset>();
        app.finish();
        app.cleanup();
        for kind in BuildingKind::ALL {
            let assets = app.world().resource::<AssetServer>();
            let visual: Handle<WorldAsset> = assets.load(kind.scene());
            let orchard: Option<Handle<bevy::gltf::Gltf>> = (kind == BuildingKind::AppleFarm)
                .then(|| assets.load(kind.scene().split('#').next().unwrap()));
            let collision: Handle<Mesh> = assets.load(kind.collision());
            let roof_mesh: Option<Handle<Mesh>> = kind.roof_profile().map(|_| {
                let primitive = match kind {
                    BuildingKind::TowerSquare => 1,
                    BuildingKind::StoneWall => 4,
                    BuildingKind::TowerRoundShort | BuildingKind::TowerRoundTall => 1,
                    BuildingKind::GatehouseSmall => 4,
                    BuildingKind::GatehouseLarge => 4,
                    _ => 0,
                };
                assets.load(format!(
                    "{}#Mesh0/Primitive{primitive}",
                    kind.scene().split('#').next().unwrap()
                ))
            });
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                app.update();
                let assets = app.world().resource::<AssetServer>();
                if assets.is_loaded_with_dependencies(visual.id())
                    && orchard
                        .as_ref()
                        .is_none_or(|h| assets.is_loaded_with_dependencies(h.id()))
                    && assets.is_loaded_with_dependencies(collision.id())
                    && roof_mesh
                        .as_ref()
                        .is_none_or(|mesh| assets.is_loaded_with_dependencies(mesh.id()))
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "{kind:?} assets failed to load: {:?}, {:?}",
                    assets.get_load_state(visual.id()),
                    assets.get_load_state(collision.id())
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            let materials = app.world().resource::<Assets<bevy::gltf::GltfMaterial>>();
            if let Some(handle) = orchard {
                let gltfs = app.world().resource::<Assets<bevy::gltf::Gltf>>();
                let gltf = gltfs.get(&handle).unwrap();
                assert_eq!(
                    gltf.named_nodes
                        .keys()
                        .filter(|name| name.starts_with("AppleTree_"))
                        .count(),
                    8,
                    "The orchard must include exactly eight apple trees"
                );
                let leaf: Handle<bevy::gltf::GltfMaterial> =
                    app.world().resource::<AssetServer>().load(format!(
                        "{}#Material7",
                        kind.scene().split('#').next().unwrap()
                    ));
                let leaf = materials.get(&leaf).unwrap();
                assert!(
                    matches!(leaf.alpha_mode, AlphaMode::Mask(_)),
                    "Leaves need transparent cutouts"
                );
                assert!(
                    leaf.base_color_texture.is_some(),
                    "Apple trees must retain their original texture"
                );
            }
            if kind == BuildingKind::Housing {
                let server = app.world().resource::<AssetServer>();
                let images = app.world().resource::<Assets<Image>>();
                for index in 0..2 {
                    let handle: Handle<bevy::gltf::GltfMaterial> = server.load(format!(
                        "{}#Material{index}",
                        kind.scene().split('#').next().unwrap()
                    ));
                    let texture = materials
                        .get(&handle)
                        .unwrap()
                        .base_color_texture
                        .as_ref()
                        .expect("Housing walls and roof must have embedded textures");
                    assert!(images.get(texture).is_some(), "Housing texture must decode");
                }
            }
            assert!(
                materials.iter().all(|(_, material)| matches!(
                    material.alpha_mode,
                    AlphaMode::Opaque | AlphaMode::Mask(_)
                )),
                "Building walls must be opaque; tree foliage may use alpha masking"
            );
            let meshes = app.world().resource::<Assets<Mesh>>();
            if let Some(handle) = roof_mesh {
                let profile = kind.roof_profile().unwrap();
                let floor = meshes
                    .get(&handle)
                    .unwrap()
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .unwrap()
                    .as_float3()
                    .unwrap();
                let mut floor_min = Vec3::splat(f32::INFINITY);
                let mut floor_max = Vec3::splat(f32::NEG_INFINITY);
                let mut floor_vertices = 0;
                for p in floor {
                    let p = model_transform(kind).transform_point(Vec3::from(*p));
                    // The keep batches walls and terrace tiles by stone material;
                    // other buildings provide a separate planar roof primitive.
                    let tolerance = if kind == BuildingKind::Keep
                        || !matches!(
                            kind,
                            BuildingKind::StoneWall
                                | BuildingKind::TowerStairwell
                                | BuildingKind::TowerLookout
                                | BuildingKind::TowerSquare
                                | BuildingKind::TowerRoundShort
                                | BuildingKind::TowerRoundTall
                                | BuildingKind::TowerGreat
                                | BuildingKind::GatehouseSmall
                        ) {
                        0.08
                    } else {
                        0.006
                    };
                    if (p.y - profile.height).abs() >= tolerance {
                        continue;
                    }
                    assert!(
                        (p.y - profile.height).abs() < tolerance,
                        "{kind:?}: roof height must match the visual mesh"
                    );
                    floor_vertices += 1;
                    floor_min = floor_min.min(p);
                    floor_max = floor_max.max(p);
                }
                assert!(floor_vertices >= 4, "{kind:?}: walkable floor must exist");
                assert!(profile.half_x <= (-floor_min.x).min(floor_max.x));
                assert!(
                    profile.half_z <= (-floor_min.z).min(floor_max.z),
                    "{kind:?}: roof extends beyond Z floor {floor_min:?} {floor_max:?}"
                );
            }
            let positions = meshes
                .get(&collision)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            for rotation in 0..4 {
                let parent = placement_transform(0.0, 0.0, 0.0, rotation);
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                for p in positions {
                    let p = parent
                        .transform_point(model_transform(kind).transform_point(Vec3::from(*p)));
                    min = min.min(p);
                    max = max.max(p);
                }
                let shape = kind.rotated_obstacle(0.0, 0.0, rotation);
                assert!(
                    (min.x + shape.half_x).abs() < 0.0001,
                    "{kind:?} at {rotation} quarter turns"
                );
                assert!((max.x - shape.half_x).abs() < 0.0001);
                assert!((min.z + shape.half_z).abs() < 0.0001);
                assert!((max.z - shape.half_z).abs() < 0.0001);
                assert!(min.y.abs() < 0.0001 && max.y > 0.0);
            }
        }
    }
}

// Physical connectivity and capacity are shared with the server. Dynamic movement
// can still change a free doorway between preview and execution.
fn enemy_surface_accessible(snapshot: &network::Snapshot, selected: &[u64], target: u64) -> bool {
    let Some(enemy) = snapshot.units.iter().find(|u| u.id == target) else {
        return false;
    };
    let pieces = snapshot
        .buildings
        .iter()
        .filter_map(|b| {
            Some(map::walls::RoofPiece {
                id: b.id,
                kind: BuildingKind::from_name(&b.kind).filter(|k| k.roof_profile().is_some())?,
                x: b.x,
                y: b.y,
                z: b.z,
                rotation: b.rotation,
            })
        })
        .collect::<Vec<_>>();
    // Archers can reach firing positions without entering the enemy roof.
    if snapshot
        .units
        .iter()
        .any(|u| selected.contains(&u.id) && u.kind == "archer")
    {
        return true;
    }
    let surface = pieces.iter().find(|p| {
        p.roof().roof_contains((enemy.x, enemy.z))
            && (enemy.y - p.roof().roof_y() - map::building_roof::UNIT_CENTER_HEIGHT).abs() < 0.2
    });
    let Some(surface) = surface else { return true };
    let component = map::walls::surface_component(surface.id, &pieces);
    let has_space = component.iter().any(|p| {
        (0..p.roof().capacity()).any(|slot| {
            !snapshot.roof_occupants.iter().any(|o| {
                !selected.contains(&o.unit_id)
                    && ((o.building_id == p.id && o.slot as usize == slot)
                        || (o.next_building == Some(p.id) && o.next_slot as usize == slot))
            })
        })
    });
    if !has_space {
        return false;
    }
    let actors = snapshot
        .units
        .iter()
        .filter(|u| selected.contains(&u.id))
        .collect::<Vec<_>>();
    if actors.iter().any(|u| {
        component.iter().any(|p| {
            p.roof().roof_contains((u.x, u.z))
                && (u.y - p.roof().roof_y() - map::building_roof::UNIT_CENTER_HEIGHT).abs() < 0.2
        })
    }) {
        return true;
    }
    let obstacles = snapshot
        .buildings
        .iter()
        .filter_map(|b| {
            Some(BuildingKind::from_name(&b.kind)?.rotated_obstacle(b.x, b.z, b.rotation))
        })
        .collect::<Vec<_>>();
    component.iter().filter(|p| !p.kind.is_wall()).any(|p| {
        let roof = p.roof();
        (0..roof.door_count()).any(|door| {
            let entrance = roof.door(door);
            map::free(entrance, &obstacles)
                && actors.iter().any(|u| {
                    let from = pieces.iter().find(|p| {
                        p.roof().roof_contains((u.x, u.z))
                            && (u.y - p.roof().roof_y() - map::building_roof::UNIT_CENTER_HEIGHT)
                                .abs()
                                < 0.2
                    });
                    if let Some(from) = from {
                        map::walls::surface_component(from.id, &pieces)
                            .iter()
                            .filter(|p| !p.kind.is_wall())
                            .any(|p| {
                                let roof = p.roof();
                                (0..roof.door_count()).any(|door| {
                                    map::free(roof.door(door), &obstacles)
                                        && map::shortest_path(roof.door(door), entrance, &obstacles)
                                            .is_some()
                                })
                            })
                    } else {
                        map::shortest_path((u.x, u.z), entrance, &obstacles).is_some()
                    }
                })
        })
    })
}

#[cfg(test)]
mod combat_preview_tests {
    use super::*;
    #[test]
    fn wall_attack_preview_requires_an_entry_or_an_attacker_already_on_the_network() {
        let mut snapshot:network::Snapshot=serde_json::from_value(serde_json::json!({
            "connected":true,"ready":true,"games":[],
            "units":[
                {"id":"101","number":1,"color":0,"health":1.,"owned":true,"x":0.,"y":0.75,"z":0.},
                {"id":"202","number":1,"color":1,"health":1.,"owned":false,"x":17.,"y":7.413,"z":19.}],
            "buildings":[
                {"id":"1","kind":"tower_stairwell","color":1,"health":1.,"owned":false,"x":10.,"y":0.,"z":18.},
                {"id":"2","kind":"stone_wall","color":1,"health":1.,"owned":false,"x":13.,"y":0.,"z":19.},
                {"id":"3","kind":"stone_wall","color":1,"health":1.,"owned":false,"x":15.,"y":0.,"z":19.},
                {"id":"4","kind":"stone_wall","color":1,"health":1.,"owned":false,"x":17.,"y":0.,"z":19.}],
            "roof_occupants":[{"unit_id":"202","building_id":"4","slot":4,"next_building":null,"next_slot":0}]
        })).unwrap();
        assert!(enemy_surface_accessible(&snapshot, &[101], 202));
        snapshot.buildings.retain(|b| b.id != 1);
        assert!(!enemy_surface_accessible(&snapshot, &[101], 202));
        snapshot.units[0].x = 13.;
        snapshot.units[0].z = 19.;
        snapshot.units[0].y = 7.413;
        assert!(enemy_surface_accessible(&snapshot, &[101], 202));
        snapshot.buildings.retain(|b| b.id != 3);
        assert!(!enemy_surface_accessible(&snapshot, &[101], 202));
    }
}

fn overlap_regions(
    kind: BuildingKind,
    x: f32,
    z: f32,
    y: f32,
    rotation: u8,
    net: &Multiplayer,
) -> Vec<(Vec3, Vec3)> {
    let a = kind.construction_bounds(x, z, rotation);
    net.snapshot
        .buildings
        .iter()
        .filter_map(|b| {
            let k = BuildingKind::from_name(&b.kind)?;
            let candidate = crate::fortifications::candidate(&net.snapshot, kind, x, z, rotation);
            let other_piece = crate::fortifications::piece(b);
            if b.owned
                && (map::fortifications::replaced_wall(candidate, other_piece)
                    || map::fortifications::may_join(candidate, other_piece)
                    || (kind.is_wall() && map::fortifications::covered_wall((x, z), other_piece)))
            {
                return None;
            }
            let other = k.construction_bounds(b.x, b.z, b.rotation);
            let x0 = (a.x - a.half_x).max(other.x - other.half_x);
            let x1 = (a.x + a.half_x).min(other.x + other.half_x);
            let z0 = (a.z - a.half_z).max(other.z - other.half_z);
            let z1 = (a.z + a.half_z).min(other.z + other.half_z);
            if x1 - x0 < 0.01 || z1 - z0 < 0.01 {
                return None;
            }
            let h = (k.height() + b.y).max(y + 0.2);
            Some((
                Vec3::new((x0 + x1) * 0.5, h + 0.12, (z0 + z1) * 0.5),
                Vec3::new(x1 - x0, 0.16, z1 - z0),
            ))
        })
        .collect()
}
pub(crate) fn overlap_preview(
    mut commands: Commands,
    state: Res<BuildingInteraction>,
    assets: Res<BuildingAssets>,
    mut pool: Local<Vec<Entity>>,
    mut gizmos: Gizmos,
) {
    for e in pool.iter() {
        commands.entity(*e).insert(Visibility::Hidden);
    }
    for (i, (pos, size)) in state.overlaps.iter().enumerate() {
        if i >= pool.len() {
            pool.push(
                commands
                    .spawn((
                        Mesh3d(assets.foundation_mesh.clone()),
                        MeshMaterial3d(assets.invalid.clone()),
                        Transform::default(),
                        Visibility::Hidden,
                    ))
                    .id(),
            );
        }
        commands.entity(pool[i]).insert((
            Visibility::Visible,
            Transform::from_translation(*pos).with_scale(*size),
        ));
        gizmos.cube(
            Transform::from_translation(*pos).with_scale(*size),
            Color::srgb(1., 0., 0.),
        );
    }
}
