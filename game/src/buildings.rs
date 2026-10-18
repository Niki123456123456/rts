use crate::*;
use stronghold_map::{self as map, Obstacle};

#[derive(Resource, Default)]
pub(crate) struct BuildingInteraction {
    pub placing: bool,
    pub selected: Option<u64>,
    pub consumed: bool,
    attack_hover: Option<Vec2>,
}
#[derive(Component)]
pub(crate) struct Building(pub network::Building);
#[derive(Component)]
pub(crate) struct BuildingCollider(u64);
#[derive(Component)]
pub(crate) struct BuildingOutline(u64);
#[derive(Resource)]
pub(crate) struct BuildingAssets {
    scene: Handle<WorldAsset>,
    collision: Handle<Mesh>,
    invisible: Handle<StandardMaterial>,
    footprint: Handle<Mesh>,
    valid: Handle<StandardMaterial>,
    invalid: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    preview: Entity,
    preview_footprint: Entity,
}
fn model_transform() -> Transform {
    Transform::from_xyz(
        -map::MODEL_CENTER.0 * map::MODEL_SCALE,
        0.0,
        -map::MODEL_CENTER.1 * map::MODEL_SCALE,
    )
    .with_scale(Vec3::splat(map::MODEL_SCALE))
}
pub(crate) fn setup_buildings(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scene = server.load("models/buildings/granary_closed.glb#Scene0");
    // Pick the supplied collision mesh, rather than the roof/decoration geometry.
    let collision = server.load("models/buildings/granary_collision.glb#Mesh0/Primitive0");
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
    let footprint = meshes.add(Cuboid::new(map::HALF_X * 2.0, 0.04, map::HALF_Z * 2.0));
    let preview = commands
        .spawn((Transform::default(), Visibility::Hidden))
        .with_children(|parent| {
            parent.spawn((WorldAssetRoot(scene.clone()), model_transform()));
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
    commands.insert_resource(BuildingAssets {
        scene,
        collision,
        invisible,
        footprint,
        valid,
        invalid,
        selected,
        preview,
        preview_footprint,
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
            building.0 = row.clone();
        } else {
            commands.entity(entity).despawn();
        }
    }
    for row in remaining.values() {
        commands
            .spawn((
                Building((*row).clone()),
                Transform::from_xyz(row.x, row.y, row.z),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((WorldAssetRoot(assets.scene.clone()), model_transform()));
                parent.spawn((
                    BuildingCollider(row.id),
                    Mesh3d(assets.collision.clone()),
                    MeshMaterial3d(assets.invisible.clone()),
                    model_transform(),
                ));
                parent.spawn((
                    BuildingOutline(row.id),
                    Mesh3d(assets.footprint.clone()),
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
    if !interaction.placing {
        commands.entity(assets.preview).insert(Visibility::Hidden);
        commands
            .entity(assets.preview_footprint)
            .insert(Visibility::Hidden);
    }
}
fn placement_valid(x: f32, z: f32, y: f32, multiplayer: &Multiplayer) -> bool {
    let shape = Obstacle { x, z };
    shape.in_map()
        && !multiplayer
            .snapshot
            .buildings
            .iter()
            .any(|b| shape.overlaps(Obstacle { x: b.x, z: b.z }))
        && !multiplayer
            .snapshot
            .units
            .iter()
            .any(|u| shape.contains((u.x, u.z), map::CLEARANCE))
        && [-map::HALF_X, 0.0, map::HALF_X].into_iter().all(|dx| {
            [-map::HALF_Z, 0.0, map::HALF_Z]
                .into_iter()
                .all(|dz| (terrain_height(x + dx, z + dz) - y).abs() <= 0.05)
        })
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
    commands.entity(assets.preview).insert(Visibility::Hidden);
    commands
        .entity(assets.preview_footprint)
        .insert(Visibility::Hidden);
    if multiplayer.snapshot.game_id.is_none() || !multiplayer.snapshot.connected {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.placing = false;
        state.selected = None;
        state.consumed = true;
        *drag = default();
        return;
    }
    if !window.focused {
        return;
    }
    if state.placing {
        *drag = default();
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
    if state.placing && mouse.just_pressed(MouseButton::Right) {
        state.placing = false;
        state.consumed = true;
        return;
    }
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let filter =
        |e| ground.contains(e) || (!state.placing && (colliders.contains(e) || units.contains(e)));
    let settings = MeshRayCastSettings::default().with_filter(&filter);
    let Some((entity, hit)) = ray_cast.cast_ray(ray, &settings).first() else {
        return;
    };
    if state.placing {
        let (x, z) = (hit.point.x, hit.point.z);
        let y = terrain_height(x, z);
        let valid = placement_valid(x, z, y, &multiplayer);
        commands
            .entity(assets.preview)
            .insert((Visibility::Visible, Transform::from_xyz(x, y, z)));
        commands.entity(assets.preview_footprint).insert((
            Visibility::Visible,
            Transform::from_xyz(x, y + 0.05, z),
            MeshMaterial3d(if valid {
                assets.valid.clone()
            } else {
                assets.invalid.clone()
            }),
        ));
        if mouse.just_pressed(MouseButton::Left) {
            state.consumed = true;
            if valid && multiplayer.send(Action::BuildGranary { x, z }) {
                state.placing = false;
            }
        }
        return;
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

pub(crate) fn attack_hover_ui(
    mut contexts: EguiContexts,
    window: Single<&Window, With<PrimaryWindow>>,
    interaction: Res<BuildingInteraction>,
    selected: Res<SelectedUnits>,
) -> Result {
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
pub(crate) fn granary_button(ui: &mut egui::Ui, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(66.0, 70.0), egui::Sense::click());
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
        egui::Color32::from_rgb(178, 140, 79),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(c.x - 25.0, rect.top() + 25.0),
            egui::pos2(c.x, rect.top() + 7.0),
            egui::pos2(c.x + 25.0, rect.top() + 25.0),
        ],
        egui::Color32::from_rgb(125, 75, 40),
        egui::Stroke::NONE,
    ));
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
        "Granary",
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    response
        .on_hover_text("Build a granary. Left click to place; right click or Esc to cancel.")
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin, mesh::MeshPlugin};
    use std::time::{Duration, Instant};

    #[test]
    fn replicated_destruction_despawns_building_children_without_stale_commands() {
        let mut app = App::new();
        let preview = app.world_mut().spawn_empty().id();
        let preview_footprint = app.world_mut().spawn_empty().id();
        app.insert_resource(BuildingAssets {
            scene: default(),
            collision: default(),
            invisible: default(),
            footprint: default(),
            valid: default(),
            invalid: default(),
            selected: default(),
            preview,
            preview_footprint,
        })
        .init_resource::<Multiplayer>()
        .init_resource::<BuildingInteraction>()
        .add_systems(Update, sync_buildings);
        let row = network::Building {
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
    fn packaged_granary_assets_load_and_collision_matches_navigation() {
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
        let assets = app.world().resource::<AssetServer>();
        let visual: Handle<WorldAsset> = assets.load("models/buildings/granary_closed.glb#Scene0");
        let collision: Handle<Mesh> =
            assets.load("models/buildings/granary_collision.glb#Mesh0/Primitive0");
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
                "Granary assets failed to load: {:?}, {:?}",
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
            "Granary roof and walls must not use the DDS data alpha as transparency"
        );
        let meshes = app.world().resource::<Assets<Mesh>>();
        let positions = meshes
            .get(&collision)
            .unwrap()
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for p in positions {
            let p = model_transform().transform_point(Vec3::from(*p));
            min = min.min(p);
            max = max.max(p);
        }
        assert!((min.x + map::HALF_X).abs() < 0.0001);
        assert!((max.x - map::HALF_X).abs() < 0.0001);
        assert!((min.z + map::HALF_Z).abs() < 0.0001);
        assert!((max.z - map::HALF_Z).abs() < 0.0001);
        assert!(min.y.abs() < 0.0001 && max.y > 6.0);
    }
}
