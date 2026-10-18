use crate::*;
use stronghold_map::{self as map, BuildingKind};

#[derive(Resource, Default)]
pub(crate) struct BuildingInteraction {
    pub placing: Option<BuildingKind>,
    pub rotation: u8,
    pub category: Option<&'static str>,
    pub selected: Option<u64>,
    pub consumed: bool,
    attack_hover: Option<Vec2>,
    roof_hover: Option<(Vec2, usize)>,
    roof_preview: Vec<Vec3>,
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
    models: [BuildingModel; BuildingKind::ALL.len()],
    invisible: Handle<StandardMaterial>,
    valid: Handle<StandardMaterial>,
    invalid: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
}
impl BuildingAssets {
    fn hide_previews(&self, commands: &mut Commands) {
        for model in &self.models {
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
    .with_scale(Vec3::splat(map::MODEL_SCALE))
}
pub(crate) fn setup_buildings(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    let models = BuildingKind::ALL.map(|kind| {
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
                MeshMaterial3d(valid.clone()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        BuildingModel {
            scene,
            collision,
            footprint,
            preview,
            preview_footprint,
        }
    });
    commands.insert_resource(BuildingAssets {
        models,
        invisible,
        valid,
        invalid,
        selected,
    });
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
        let model = &assets.models[kind.index()];
        commands
            .spawn((
                Building((*row).clone()),
                placement_transform(row.x, row.y, row.z, row.rotation),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((WorldAssetRoot(model.scene.clone()), model_transform(kind)));
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
    y: f32,
    rotation: u8,
    multiplayer: &Multiplayer,
) -> bool {
    let shape = kind.rotated_build_footprint(x, z, rotation);
    shape.in_build_area()
        && !multiplayer.snapshot.buildings.iter().any(|b| {
            shape.overlaps(
                BuildingKind::from_name(&b.kind)
                    .unwrap_or_default()
                    .rotated_build_footprint(b.x, b.z, b.rotation),
            )
        })
        && !multiplayer
            .snapshot
            .units
            .iter()
            .any(|u| kind.rotated_blocks_construction_unit(x, z, rotation, (u.x, u.z)))
        && kind
            .rotated_build_samples(x, z, rotation)
            .all(|(px, pz)| (terrain_height(px, pz) - y).abs() <= 0.05)
}

// Keep this identical to the server's terrain surface for the placement preview.
pub(crate) fn terrain_height(x: f32, z: f32) -> f32 {
    let mut h: f32 = 0.0;
    if (7.5..=22.5).contains(&x) && (-7.5..=-2.5).contains(&z) {
        h = 1.0;
    }
    if (-20.0..=-10.0).contains(&x) && (-7.5..=7.5).contains(&z) {
        h = 5.0;
    }
    let sphere = 12.5_f32.powi(2) - x * x - (z + 23.0).powi(2);
    if sphere >= 0.0 {
        h = h.max(sphere.sqrt());
    }
    h
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
    assets: Res<BuildingAssets>,
) {
    state.consumed = false;
    state.attack_hover = None;
    state.roof_hover = None;
    state.roof_preview.clear();
    assets.hide_previews(&mut commands);
    if multiplayer.snapshot.game_id.is_none() || !multiplayer.snapshot.connected {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.placing = None;
        state.rotation = 0;
        state.selected = None;
        state.consumed = true;
        *drag = default();
        return;
    }
    if !window.focused {
        return;
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
        ctx.egui_wants_pointer_input() || panel.0.is_some_and(|r| r.contains(pos))
    }) {
        return;
    }
    if state.placing.is_some() && mouse.just_pressed(MouseButton::Right) {
        state.placing = None;
        state.consumed = true;
        return;
    }
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let filter = |e| {
        ground.contains(e)
            || (state.placing.is_none() && (colliders.contains(e) || units.contains(e)))
    };
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let Some((entity, hit)) = ray_cast.cast_ray(ray, &settings).first() else {
        return;
    };
    if let Some(kind) = state.placing {
        let model = &assets.models[kind.index()];
        let (x, z) = kind.rotated_snap_position(hit.point.x, hit.point.z, state.rotation);
        let y = terrain_height(x, z);
        let valid = placement_valid(kind, x, z, y, state.rotation, &multiplayer);
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
            if valid && multiplayer.send(action) {
                state.placing = None;
            }
        }
        return;
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
            .filter(|(_, distance, _)| *distance <= hit.distance + 0.08 || units.contains(*entity))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((building, _, point)) = target {
            let id = building.id;
            let tower = stair_geometry(building);
            let free = tower
                .sorted_slots((point.x, point.z))
                .into_iter()
                .filter(|slot| {
                    !multiplayer.snapshot.roof_occupants.iter().any(|o| {
                        !roof_units.contains(&o.unit_id)
                            && ((o.building_id == id && o.slot as usize == *slot)
                                || (o.next_building == Some(id) && o.next_slot as usize == *slot))
                    })
                })
                .take(roof_units.len())
                .collect::<Vec<_>>();
            state.roof_hover = Some((cursor, free.len()));
            state.roof_preview = free
                .iter()
                .map(|slot| {
                    let p = tower.slot(*slot);
                    Vec3::new(p.0, tower.roof_y() + 0.08, p.1)
                })
                .collect();
            if mouse.just_pressed(MouseButton::Right) {
                state.consumed = true;
                *drag = default();
                multiplayer.send(Action::MoveToRoof {
                    unit_ids: roof_units,
                    building_id: id,
                    x: point.x,
                    z: point.z,
                });
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

fn stair_geometry(building: &network::Building) -> map::stair_tower::StairTower {
    map::stair_tower::StairTower {
        x: building.x,
        y: building.y,
        z: building.z,
        rotation: building.rotation,
    }
}
fn roof_ray_hit(ray: Ray3d, building: &network::Building) -> Option<(f32, Vec3)> {
    if !map::stair_tower::is_stair_tower(&building.kind) || ray.direction.y.abs() < 0.00001 {
        return None;
    }
    let tower = stair_geometry(building);
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
    if let Some((cursor, count)) = interaction.roof_hover {
        let ctx = contexts.ctx_mut()?;
        let cursor = cursor * (window.scale_factor() / ctx.pixels_per_point());
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("roof_hover"),
        ));
        painter.text(
            egui::pos2(cursor.x + 20., cursor.y - 25.),
            egui::Align2::LEFT_BOTTOM,
            if count == 0 {
                "Dach voll".into()
            } else {
                format!(
                    "Rechtsklick: {count} aufs Dach (max. {})",
                    map::stair_tower::ROOF_CAPACITY
                )
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
pub(crate) fn building_button(ui: &mut egui::Ui, kind: BuildingKind, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(114.0, 70.0), egui::Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        5.0,
        if active {
            egui::Color32::from_rgb(55, 95, 55)
        } else {
            ui.visuals().widgets.inactive.bg_fill
        },
    );
    let c = rect.center();
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(c.x - 21.0, rect.top() + 24.0),
            egui::pos2(c.x + 21.0, rect.top() + 49.0),
        ),
        2.0,
        if kind == BuildingKind::Barracks || kind.category() == "fortifications" {
            egui::Color32::GRAY
        } else {
            egui::Color32::from_rgb(178, 140, 79)
        },
    );
    if kind == BuildingKind::Barracks || kind.category() == "fortifications" {
        for i in -2..=2 {
            let x = c.x + i as f32 * 9.0;
            painter.rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x - 3.0, rect.top() + 15.0),
                    egui::vec2(6.0, 12.0),
                ),
                0.0,
                egui::Color32::GRAY,
            );
        }
    } else if kind == BuildingKind::Stockpile {
        for row in 0..2 {
            for column in 0..3 {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(
                            c.x - 21.0 + column as f32 * 15.0,
                            rect.top() + 14.0 + row as f32 * 16.0,
                        ),
                        egui::vec2(12.0, 13.0),
                    ),
                    1.0,
                    egui::Color32::from_rgb(125, 75, 40),
                );
            }
        }
    } else {
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(c.x - 25.0, rect.top() + 25.0),
                egui::pos2(c.x, rect.top() + 7.0),
                egui::pos2(c.x + 25.0, rect.top() + 25.0),
            ],
            egui::Color32::from_rgb(125, 75, 40),
            egui::Stroke::NONE,
        ));
    }
    for i in -1..=1 {
        let x = c.x + i as f32 * 9.0;
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 32.0),
                egui::pos2(x, rect.top() + 45.0),
            ],
            egui::Stroke::new(3.0, egui::Color32::GOLD),
        );
    }
    painter.text(
        egui::pos2(c.x, rect.bottom() - 10.0),
        egui::Align2::CENTER_CENTER,
        kind.label(),
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    let (width, depth) = kind.grid_size();
    response
        .on_hover_text(format!(
            "{}: {}×{} Felder ({}×{} m). R dreht um 90°. Linksklick platziert; Rechtsklick oder Esc bricht ab.",
            kind.label(),
            width,
            depth,
            width * 2,
            depth * 2
        ))
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin, mesh::MeshPlugin};
    use std::time::{Duration, Instant};

    #[test]
    fn roof_click_uses_elevated_plane_for_every_tower_rotation() {
        for rotation in 0..4 {
            let row = network::Building {
                id: 1,
                kind: "tower_stairwell".into(),
                rotation,
                color: 0,
                health: 1.,
                owned: true,
                x: 10.,
                y: 3.,
                z: 20.,
            };
            let ray = Ray3d::new(Vec3::new(10., 20., 20.), Dir3::NEG_Y);
            let (_, point) = roof_ray_hit(ray, &row).unwrap();
            assert!((point.y - 9.663).abs() < 0.001);
            assert!(
                roof_ray_hit(Ray3d::new(Vec3::new(15., 20., 20.), Dir3::NEG_Y), &row).is_none()
            );
            assert!(roof_ray_hit(Ray3d::new(Vec3::new(10., 20., 20.), Dir3::X), &row).is_none());
        }
    }

    #[test]
    fn replicated_destruction_despawns_building_children_without_stale_commands() {
        let mut app = App::new();
        let models = BuildingKind::ALL.map(|_| BuildingModel {
            scene: default(),
            collision: default(),
            footprint: default(),
            preview: app.world_mut().spawn_empty().id(),
            preview_footprint: app.world_mut().spawn_empty().id(),
        });
        app.insert_resource(BuildingAssets {
            models,
            invisible: default(),
            valid: default(),
            invalid: default(),
            selected: default(),
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
            let collision: Handle<Mesh> = assets.load(kind.collision());
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                app.update();
                let assets = app.world().resource::<AssetServer>();
                if assets.is_loaded_with_dependencies(visual.id())
                    && assets.is_loaded_with_dependencies(collision.id())
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
            assert!(
                materials
                    .iter()
                    .all(|(_, material)| material.alpha_mode == AlphaMode::Opaque),
                "Building walls must not use the DDS data alpha as transparency"
            );
            let meshes = app.world().resource::<Assets<Mesh>>();
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
