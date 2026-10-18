//! Multiplayer terrain scene. Unit positions are authoritative SpacetimeDB state.

mod buildings;
#[cfg(not(target_arch = "wasm32"))]
mod module_bindings;
mod network;
use buildings::BuildingInteraction;

use network::{Action, Multiplayer};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext};

use bevy_rts_camera::{Ground, RtsCamera, RtsCameraControls, RtsCameraPlugin};

fn main() {
    let mut app = App::new();
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Stronghold".into(),
            ..default()
        }),
        ..default()
    }));
    #[cfg(target_arch = "wasm32")]
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Stronghold".into(),
            canvas: Some("#game".into()),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((MeshPickingPlugin, EguiPlugin::default()))
        .add_plugins(RtsCameraPlugin)
        .init_resource::<SelectedUnits>()
        .init_resource::<SelectionPanel>()
        .init_resource::<SelectionDrag>()
        .init_resource::<MovementDrag>()
        .init_resource::<Multiplayer>()
        .init_resource::<BuildingInteraction>()
        .add_systems(
            Startup,
            (setup, buildings::setup_buildings, connect_multiplayer),
        )
        .add_systems(
            Update,
            (
                sync_multiplayer,
                buildings::sync_buildings,
                buildings::building_input,
                select_unit,
                movement_input,
                formation_preview,
                expire_destination_markers,
                highlight_selection,
            )
                .chain(),
        )
        .add_systems(
            EguiPrimaryContextPass,
            (
                lobby_ui,
                selection_ui,
                selection_rectangle_ui,
                buildings::attack_hover_ui,
            )
                .chain(),
        )
        .run();
}

#[derive(Component, Default)]
struct Unit {
    id: u64,
    number: usize,
    color: u32,
    health: f32,
    owned: bool,
}

#[derive(Component)]
struct DestinationMarker(Timer);

#[derive(Resource)]
struct DestinationVisual {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct SelectedUnits(Vec<Entity>);

impl SelectedUnits {
    fn select_rectangle(
        &mut self,
        start: Vec2,
        end: Vec2,
        additive: bool,
        candidates: impl IntoIterator<Item = (Entity, bool, Vec2)>,
    ) {
        let rect = Rect::from_corners(start, end);
        if !additive {
            self.0.clear();
        }
        for (entity, owned, position) in candidates {
            if owned && rect.contains(position) && !self.0.contains(&entity) {
                self.0.push(entity);
            }
        }
    }

    fn select(&mut self, entity: Entity, additive: bool) {
        if !additive {
            self.0.clear();
            self.0.push(entity);
        } else if let Some(index) = self.0.iter().position(|selected| *selected == entity) {
            self.0.remove(index);
        } else {
            self.0.push(entity);
        }
    }
}

#[derive(Resource, Default)]
struct SelectionPanel(Option<egui::Rect>);

#[derive(Resource, Default)]
struct SelectionDrag {
    start: Option<Vec2>,
    current: Vec2,
    dragging: bool,
    additive: bool,
}

#[derive(Resource, Default)]
struct MovementDrag {
    start: Option<Vec3>,
    screen_start: Vec2,
    end: Vec3,
    dragging: bool,
    positions: Vec<stronghold_map::Point>,
}

#[derive(Resource)]
struct UnitMaterials {
    colors: std::collections::HashMap<u32, UnitPalette>,
    mesh: Handle<Mesh>,
}

struct UnitPalette {
    normal: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
}

fn player_color(index: u32) -> Color {
    Color::hsl((210.0 + index as f32 * 137.507_77) % 360.0, 0.8, 0.55)
}

impl UnitMaterials {
    fn palette(&mut self, index: u32, assets: &mut Assets<StandardMaterial>) -> &UnitPalette {
        self.colors.entry(index).or_insert_with(|| {
            let color = player_color(index);
            UnitPalette {
                normal: assets.add(StandardMaterial {
                    base_color: color,
                    ..default()
                }),
                selected: assets.add(StandardMaterial {
                    base_color: color,
                    emissive: LinearRgba::from(color) * 0.6,
                    ..default()
                }),
            }
        })
    }
}

fn connect_multiplayer(mut multiplayer: ResMut<Multiplayer>) {
    let uri = std::env::var("SPACETIMEDB_URI").unwrap_or_else(|_| multiplayer.uri.clone());
    let database =
        std::env::var("SPACETIMEDB_DATABASE").unwrap_or_else(|_| multiplayer.database.clone());
    multiplayer.uri = uri.clone();
    multiplayer.database = database.clone();
    multiplayer.send(Action::Connect { uri, database });
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DestinationVisual {
        mesh: meshes.add(Torus::new(0.3, 0.4)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.1),
            emissive: LinearRgba::new(1.0, 0.6, 0.0, 1.0),
            unlit: true,
            ..default()
        }),
    });
    // Ground
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(stronghold_map::MAP_SIZE, stronghold_map::MAP_SIZE),
            ),
        ),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        // Add `Ground` component to any entity you want the camera to treat as ground.
        Ground,
    ));
    // Some "terrain"
    let terrain_material = materials.add(Color::srgb(0.8, 0.7, 0.6));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(15.0, 1.0, 5.0))),
        MeshMaterial3d(terrain_material.clone()),
        Transform::from_xyz(15.0, 0.5, -5.0),
        Ground,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 5.0, 15.0))),
        MeshMaterial3d(terrain_material.clone()),
        Transform::from_xyz(-15.0, 2.5, 0.0),
        Ground,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(12.5))),
        MeshMaterial3d(terrain_material.clone()),
        Transform::from_xyz(0.0, 0.0, -23.0),
        Ground,
    ));
    commands.insert_resource(UnitMaterials {
        colors: default(),
        mesh: meshes.add(Capsule3d::new(0.25, 1.25)),
    });
    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 1000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::YXZ,
            150.0f32.to_radians(),
            -40.0f32.to_radians(),
            0.0,
        )),
    ));
    // Camera
    commands.spawn((
        Camera3d::default(),
        PrimaryEguiContext,
        RtsCamera {
            // Increase min height (decrease max zoom)
            // height_min: 10.0,
            // Increase max height (decrease min zoom)
            height_max: 50.0,
            bounds: Aabb2d::new(Vec2::ZERO, Vec2::splat(stronghold_map::MAP_LIMIT)),
            // Change the angle of the camera to 35 degrees
            min_angle: 35.0f32.to_radians(),
            // Decrease smoothing
            smoothness: 0.1,
            // Change starting position
            target_focus: Transform::from_xyz(3.0, 0.0, -3.0),
            // Change starting zoom level
            target_zoom: 0.2,
            // Disable dynamic angle (angle of camera will stay at `min_angle`)
            // dynamic_angle: false,
            ..default()
        },
        RtsCameraControls {
            // Change pan controls to WASD
            key_up: KeyCode::KeyW,
            key_down: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            // Reserve right click for movement orders; rotate the camera with Q/E.
            button_rotate: MouseButton::Other(0),
            lock_on_rotate: false,
            // Drag pan with middle click
            button_drag: Some(MouseButton::Middle),
            // Keep the mouse cursor in place when dragging
            lock_on_drag: true,
            // Change the width of the area that triggers edge pan. 0.1 is 10% of the window height.
            edge_pan_width: 0.1,
            // Increase pan speed
            pan_speed: 25.0,
            ..default()
        },
    ));
}

fn select_unit(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mut ray_cast: MeshRayCast,
    units: Query<(Entity, &Transform, &Unit)>,
    multiplayer: Res<Multiplayer>,
    mut selected: ResMut<SelectedUnits>,
    mut contexts: EguiContexts,
    panel: Res<SelectionPanel>,
    ground: Query<(), With<Ground>>,
    mut drag: ResMut<SelectionDrag>,
    building_interaction: Res<BuildingInteraction>,
) {
    if building_interaction.placing.is_some() || building_interaction.consumed {
        *drag = default();
        return;
    }
    if multiplayer.snapshot.game_id.is_none()
        || !multiplayer.snapshot.connected
        || !window.focused
        || keys.just_pressed(KeyCode::Escape)
    {
        *drag = default();
        return;
    }
    let Some(mut cursor) = window.cursor_position() else {
        *drag = default();
        return;
    };
    let over_ui = contexts.ctx_mut().is_ok_and(|ctx| {
        let ui_position =
            egui::pos2(cursor.x, cursor.y) * (window.scale_factor() / ctx.pixels_per_point());
        ctx.egui_wants_pointer_input() || panel.0.is_some_and(|rect| rect.contains(ui_position))
    });
    let move_order = mouse.just_pressed(MouseButton::Right);
    if move_order {
        *drag = default();
    }
    if mouse.just_pressed(MouseButton::Left) && !over_ui && !move_order {
        let cmd_keys = [KeyCode::SuperLeft, KeyCode::SuperRight];
        *drag = SelectionDrag {
            start: Some(cursor),
            current: cursor,
            dragging: false,
            additive: keys.any_pressed(cmd_keys) || keys.any_just_pressed(cmd_keys),
        };
    }
    let mut select = false;
    let mut additive = false;
    if let Some(start) = drag.start {
        drag.current = cursor;
        // A small mouse movement is still a normal click.
        drag.dragging |= start.distance_squared(cursor) >= 16.0;
        if mouse.just_released(MouseButton::Left) {
            let completed = std::mem::take(&mut *drag);
            if completed.dragging {
                let (camera, camera_transform) = *camera;
                selected.select_rectangle(
                    start,
                    cursor,
                    completed.additive,
                    units.iter().filter_map(|(entity, transform, unit)| {
                        camera
                            .world_to_viewport(camera_transform, transform.translation)
                            .ok()
                            .map(|position| (entity, unit.owned, position))
                    }),
                );
                return;
            }
            cursor = start;
            select = true;
            additive = completed.additive;
        } else if mouse.pressed(MouseButton::Left) {
            return;
        } else {
            *drag = default();
        }
    }
    if !select {
        return;
    }
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    // Cast against the actual visible geometry, independent of egui's picking events.
    let scene_filter = |entity| units.contains(entity) || ground.contains(entity);
    let settings = MeshRayCastSettings::default().with_filter(&scene_filter);
    let Some((entity, _)) = ray_cast.cast_ray(ray, &settings).first() else {
        if select {
            selected.0.clear();
        }
        return;
    };
    if select && units.get(*entity).is_ok_and(|(_, _, unit)| unit.owned) {
        selected.select(*entity, additive);
    } else if select {
        selected.0.clear();
    }
}

fn movement_input(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mut ray_cast: MeshRayCast,
    units: Query<&Unit>,
    mut multiplayer: ResMut<Multiplayer>,
    selected: Res<SelectedUnits>,
    mut contexts: EguiContexts,
    panel: Res<SelectionPanel>,
    ground: Query<(), With<Ground>>,
    markers: Query<Entity, With<DestinationMarker>>,
    visual: Res<DestinationVisual>,
    mut drag: ResMut<MovementDrag>,
    building_interaction: Res<BuildingInteraction>,
) {
    if building_interaction.placing.is_some()
        || building_interaction.consumed
        || multiplayer.snapshot.game_id.is_none()
        || !multiplayer.snapshot.connected
        || !window.focused
        || keys.just_pressed(KeyCode::Escape)
        || mouse.just_pressed(MouseButton::Left)
    {
        *drag = default();
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        *drag = default();
        return;
    };
    let over_ui = contexts.ctx_mut().is_ok_and(|ctx| {
        let pos = egui::pos2(cursor.x, cursor.y) * (window.scale_factor() / ctx.pixels_per_point());
        ctx.egui_wants_pointer_input() || panel.0.is_some_and(|rect| rect.contains(pos))
    });
    if over_ui {
        // Releasing over a panel cancels the order.
        drag.positions.clear();
        if !mouse.pressed(MouseButton::Right) {
            *drag = default();
        }
        return;
    }
    let unit_ids: Vec<_> = selected
        .0
        .iter()
        .filter_map(|entity| {
            units
                .get(*entity)
                .ok()
                .filter(|unit| unit.owned)
                .map(|unit| unit.id)
        })
        .collect();
    if unit_ids.is_empty() {
        *drag = default();
        return;
    }
    if !mouse.just_pressed(MouseButton::Right) && drag.start.is_none() {
        return;
    }
    let (camera, transform) = *camera;
    let point = camera
        .viewport_to_world(transform, cursor)
        .ok()
        .and_then(|ray| {
            let filter = |entity| ground.contains(entity);
            ray_cast
                .cast_ray(ray, &MeshRayCastSettings::default().with_filter(&filter))
                .first()
                .map(|(_, hit)| hit.point)
        });
    if mouse.just_pressed(MouseButton::Right) {
        *drag = MovementDrag {
            start: point,
            screen_start: cursor,
            end: point.unwrap_or_default(),
            ..default()
        };
    }
    let Some(start) = drag.start else {
        return;
    };
    drag.dragging |= cursor.distance_squared(drag.screen_start) >= 16.0;
    if let Some(point) = point {
        drag.end = point;
    }
    drag.positions = if drag.dragging && point.is_some() {
        stronghold_map::formation_positions(
            (start.x, start.z),
            (drag.end.x, drag.end.z),
            unit_ids.len(),
        )
    } else {
        Vec::new()
    };
    if mouse.just_released(MouseButton::Right) {
        let completed = std::mem::take(&mut *drag);
        if point.is_none() {
            return;
        }
        let action = if completed.dragging {
            Action::FormationTargets {
                unit_ids,
                start_x: start.x,
                start_z: start.z,
                end_x: completed.end.x,
                end_z: completed.end.z,
            }
        } else {
            Action::Targets {
                unit_ids,
                x: start.x,
                z: start.z,
            }
        };
        if multiplayer.send(action) {
            for marker in &markers {
                commands.entity(marker).despawn();
            }
            let destinations = if completed.dragging {
                completed.positions
            } else {
                vec![(start.x, start.z)]
            };
            for (x, z) in destinations {
                commands.spawn((
                    DestinationMarker(Timer::from_seconds(0.8, TimerMode::Once)),
                    Mesh3d(visual.mesh.clone()),
                    MeshMaterial3d(visual.material.clone()),
                    Transform::from_xyz(x, buildings::terrain_height(x, z) + 0.06, z)
                        .with_scale(Vec3::splat(0.6)),
                ));
            }
        }
    } else if !mouse.pressed(MouseButton::Right) {
        *drag = default();
    }
}

fn formation_preview(drag: Res<MovementDrag>, mut gizmos: Gizmos) {
    for &(x, z) in &drag.positions {
        gizmos.circle(
            Isometry3d::new(
                Vec3::new(x, buildings::terrain_height(x, z) + 0.08, z),
                Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            ),
            0.2,
            Color::srgb(1.0, 0.85, 0.2),
        );
    }
}

#[cfg(test)]
fn unit_at_ray(
    ray: Ray3d,
    ray_cast: &mut MeshRayCast,
    units: &Query<(), With<Unit>>,
) -> Option<Entity> {
    ray_cast
        .cast_ray(ray, &MeshRayCastSettings::default())
        .first()
        .map(|(entity, _)| *entity)
        .filter(|entity| units.contains(*entity))
}

fn sync_multiplayer(
    mut commands: Commands,
    mut multiplayer: ResMut<Multiplayer>,
    mut selected: ResMut<SelectedUnits>,
    mut units: Query<(Entity, &mut Unit, &mut Transform)>,
    mut materials: ResMut<UnitMaterials>,
    mut assets: ResMut<Assets<StandardMaterial>>,
    markers: Query<Entity, With<DestinationMarker>>,
    mut previous_game: Local<Option<u64>>,
    mut drag: ResMut<SelectionDrag>,
) {
    multiplayer.poll();
    let snapshot = &multiplayer.snapshot;
    let game_id = snapshot.game_id.filter(|_| snapshot.connected);
    if game_id != *previous_game {
        selected.0.clear();
        *drag = default();
        for marker in &markers {
            commands.entity(marker).despawn();
        }
        *previous_game = game_id;
    }
    let mut remaining: std::collections::HashMap<_, _> = snapshot
        .units
        .iter()
        .filter(|_| game_id.is_some())
        .map(|unit| (unit.id, unit))
        .collect();
    for (entity, mut unit, mut transform) in &mut units {
        if let Some(row) = remaining.remove(&unit.id) {
            // Rendering only: the client never advances or predicts movement.
            transform.translation = Vec3::new(row.x, row.y, row.z);
            unit.health = row.health;
            unit.owned = row.owned;
        } else {
            commands.entity(entity).despawn();
            selected.0.retain(|selected| *selected != entity);
        }
    }
    for row in remaining.values() {
        let material = materials.palette(row.color, &mut assets).normal.clone();
        commands.spawn((
            Unit {
                id: row.id,
                number: row.number,
                color: row.color,
                health: row.health,
                owned: row.owned,
            },
            Mesh3d(materials.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(row.x, row.y, row.z),
        ));
    }
}

fn lobby_ui(
    mut contexts: EguiContexts,
    mut multiplayer: ResMut<Multiplayer>,
    mut controls: Query<&mut RtsCameraControls>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if multiplayer.snapshot.game_id.is_some() && multiplayer.snapshot.connected {
        return Ok(());
    }
    for mut control in &mut controls {
        control.enabled = false;
    }
    egui::Window::new("Stronghold")
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .resizable(false)
        .collapsible(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            let connected = multiplayer.snapshot.connected && multiplayer.snapshot.ready;
            ui.horizontal(|ui| {
                ui.heading("Stronghold");
                let (rect, light) =
                    ui.allocate_exact_size(egui::vec2(14.0, 20.0), egui::Sense::hover());
                ui.painter().circle_filled(
                    rect.center(),
                    4.0,
                    if connected {
                        egui::Color32::GREEN
                    } else {
                        egui::Color32::RED
                    },
                );
                light.on_hover_text(if connected {
                    "Connected"
                } else {
                    "Disconnected"
                });
            });
            ui.add_space(8.0);
            egui::CollapsingHeader::new("Connection settings")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Server");
                        ui.text_edit_singleline(&mut multiplayer.uri);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Database");
                        ui.text_edit_singleline(&mut multiplayer.database);
                    });
                    if ui
                        .button(if connected { "Reconnect" } else { "Connect" })
                        .clicked()
                    {
                        let uri = multiplayer.uri.clone();
                        let database = multiplayer.database.clone();
                        multiplayer.send(Action::Connect { uri, database });
                    }
                });
            ui.heading("Games");
            if let Some(error) = &multiplayer.snapshot.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            ui.separator();
            let games = multiplayer.snapshot.games.clone();
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    if connected && games.is_empty() {
                        ui.label("No games yet. Create the first one below.");
                    }
                    for game in games {
                        ui.push_id(game.id, |ui| {
                            let (rect, row) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 38.0),
                                egui::Sense::hover(),
                            );
                            let hovered = row.hovered()
                                || ctx
                                    .pointer_hover_pos()
                                    .is_some_and(|pos| rect.contains(pos));
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.label(&game.title);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if hovered
                                                && ui
                                                    .add_enabled(
                                                        connected && !multiplayer.pending,
                                                        egui::Button::new("Join"),
                                                    )
                                                    .clicked()
                                            {
                                                multiplayer.send(Action::Join { game_id: game.id });
                                            }
                                        },
                                    );
                                });
                            });
                        });
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let create = ui.add_enabled(
                        connected && !multiplayer.pending && !multiplayer.title.trim().is_empty(),
                        egui::Button::new("Create Game"),
                    );
                    ui.add_sized(
                        egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                        egui::TextEdit::singleline(&mut multiplayer.title)
                            .hint_text("Game title")
                            .char_limit(60),
                    );
                    if create.clicked() {
                        let title = multiplayer.title.clone();
                        multiplayer.send(Action::Create { title });
                    }
                });
            });
            if !connected {
                ui.small(
                    "Create Game requires a connection to the published SpacetimeDB database.",
                );
            } else if multiplayer.pending {
                ui.small("Waiting for the server …");
            } else if multiplayer.title.trim().is_empty() {
                ui.small("Enter a game title to enable Create Game.");
            }
        });
    Ok(())
}

fn expire_destination_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut markers: Query<(Entity, &mut DestinationMarker)>,
) {
    for (entity, mut marker) in &mut markers {
        if marker.0.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn highlight_selection(
    selected: Res<SelectedUnits>,
    materials: Res<UnitMaterials>,
    mut units: Query<(Entity, &Unit, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !selected.is_changed() {
        return;
    }
    for (entity, unit, mut material) in &mut units {
        let Some(palette) = materials.colors.get(&unit.color) else {
            continue;
        };
        material.0 = if selected.0.contains(&entity) {
            palette.selected.clone()
        } else {
            palette.normal.clone()
        };
    }
}

fn selection_ui(
    mut contexts: EguiContexts,
    selected: Res<SelectedUnits>,
    units: Query<&Unit>,
    mut controls: Query<&mut RtsCameraControls>,
    mut panel: ResMut<SelectionPanel>,
    mut multiplayer: ResMut<Multiplayer>,
    drag: Res<SelectionDrag>,
    movement: Res<MovementDrag>,
    mut building_interaction: ResMut<BuildingInteraction>,
) -> Result {
    if multiplayer.snapshot.game_id.is_none() || !multiplayer.snapshot.connected {
        panel.0 = None;
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "selection_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let response = egui::Panel::bottom("selected_units")
        .resizable(false)
        .exact_size(110.0)
        .show(&mut viewport_ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Auswahl");
                    if ui.button("Leave game").clicked() {
                        multiplayer.send(Action::Leave);
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    let category = building_interaction.category.unwrap_or("supply");
                    let category_label = |category| match category {
                        "military" => "Militär",
                        "fortifications" => "Befestigungen",
                        _ => "Versorgung",
                    };
                    egui::ComboBox::from_id_salt("build_category")
                        .selected_text(category_label(category))
                        .show_ui(ui, |ui| {
                            for category in ["supply", "military", "fortifications"] {
                                if ui
                                    .selectable_label(
                                        building_interaction.category.unwrap_or("supply")
                                            == category,
                                        category_label(category),
                                    )
                                    .clicked()
                                {
                                    building_interaction.category = Some(category);
                                }
                            }
                        });
                    egui::ScrollArea::horizontal()
                        .id_salt("building_palette")
                        .max_width(380.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let category = building_interaction.category.unwrap_or("supply");
                                for kind in stronghold_map::BuildingKind::ALL
                                    .into_iter()
                                    .filter(|kind| kind.category() == category)
                                {
                                    let active = building_interaction.placing == Some(kind);
                                    if buildings::building_button(ui, kind, active) {
                                        building_interaction.placing =
                                            if active { None } else { Some(kind) };
                                        building_interaction.selected = None;
                                    }
                                }
                            });
                        });
                });
                ui.separator();
                if let Some(kind) = building_interaction.placing {
                    ui.label(format!(
                        "{} auf grünem Boden platzieren. Rechtsklick / Esc bricht ab.",
                        kind.label()
                    ));
                } else if let Some(building) = building_interaction
                    .selected
                    .and_then(|id| multiplayer.snapshot.buildings.iter().find(|b| b.id == id))
                    .cloned()
                {
                    ui.vertical(|ui| {
                        let color = player_color(building.color).to_srgba();
                        ui.colored_label(
                            egui::Color32::from_rgb(
                                (color.red * 255.0) as u8,
                                (color.green * 255.0) as u8,
                                (color.blue * 255.0) as u8,
                            ),
                            format!(
                                "{}: {}",
                                if building.owned {
                                    "Eigenes Gebäude"
                                } else {
                                    "Feindliches Gebäude"
                                },
                                stronghold_map::BuildingKind::from_name(&building.kind)
                                    .unwrap_or_default()
                                    .label()
                            ),
                        );
                        ui.add(
                            egui::ProgressBar::new(building.health)
                                .text(format!("Health {:.0}%", building.health * 100.0))
                                .desired_width(160.0),
                        );
                        if building.owned && ui.button("Remove building").clicked() {
                            multiplayer.send(Action::RemoveBuilding {
                                building_id: building.id,
                            });
                        }
                    });
                } else if selected.0.is_empty() {
                    ui.label("Klicke auf eine Einheit. Cmd + Klick für Mehrfachauswahl.");
                } else {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for entity in &selected.0 {
                                let Ok(unit) = units.get(*entity) else {
                                    continue;
                                };
                                ui.push_id(unit.number, |ui| {
                                    egui::Frame::group(ui.style()).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // A capsule portrait matching the units in the scene.
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(44.0, 64.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                rect.shrink2(egui::vec2(10.0, 4.0)),
                                                12.0,
                                                {
                                                    let color = player_color(unit.color).to_srgba();
                                                    egui::Color32::from_rgb(
                                                        (color.red * 255.0) as u8,
                                                        (color.green * 255.0) as u8,
                                                        (color.blue * 255.0) as u8,
                                                    )
                                                },
                                            );
                                            ui.vertical(|ui| {
                                                ui.strong(format!("Einheit {}", unit.number));
                                                ui.add(
                                                    egui::ProgressBar::new(unit.health)
                                                        .text(format!(
                                                            "Health {:.0}%",
                                                            unit.health * 100.0
                                                        ))
                                                        .desired_width(95.0),
                                                );
                                            });
                                        });
                                    });
                                });
                            }
                        });
                    });
                }
            });
            if let Some(error) = &multiplayer.snapshot.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
        });
    panel.0 = Some(response.response.rect);

    // Prevent edge pan, zoom and rotation while interacting with the UI.
    let over_panel = ctx
        .pointer_interact_pos()
        .is_some_and(|position| response.response.rect.contains(position));
    for mut controls in &mut controls {
        controls.enabled = movement.start.is_none()
            && !drag.dragging
            && !over_panel
            && !ctx.egui_wants_pointer_input()
            && !ctx.egui_wants_keyboard_input();
    }
    Ok(())
}

fn selection_rectangle_ui(
    mut contexts: EguiContexts,
    window: Single<&Window, With<PrimaryWindow>>,
    drag: Res<SelectionDrag>,
) -> Result {
    let Some(start) = drag.start.filter(|_| drag.dragging) else {
        return Ok(());
    };
    let ctx = contexts.ctx_mut()?;
    let scale = window.scale_factor() / ctx.pixels_per_point();
    let rect = egui::Rect::from_two_pos(
        egui::pos2(start.x, start.y) * scale,
        egui::pos2(drag.current.x, drag.current.y) * scale,
    );
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("selection_rectangle"),
    ));
    painter.rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(80, 180, 255, 30),
    );
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 180, 255)),
        egui::StrokeKind::Inside,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::primitives::MeshAabb;
    use bevy::camera::visibility::{InheritedVisibility, ViewVisibility};
    use bevy::ecs::system::SystemState;

    fn spawn_mesh(world: &mut World, mesh: Mesh, position: Vec3) -> Entity {
        let aabb = mesh.get_aabb().unwrap();
        let handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
        world
            .spawn((
                Mesh3d(handle),
                aabb,
                GlobalTransform::from_translation(position),
                InheritedVisibility::VISIBLE,
                ViewVisibility::VISIBLE,
            ))
            .id()
    }

    fn pick(world: &mut World, ray: Ray3d) -> Option<Entity> {
        bevy::tasks::ComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        let mut state = SystemState::<(MeshRayCast, Query<(), With<Unit>>)>::new(world);
        let (mut ray_cast, units) = state.get(world).unwrap();
        unit_at_ray(ray, &mut ray_cast, &units)
    }

    #[test]
    fn clicking_capsule_geometry_selects_the_nearest_unit() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        let near = spawn_mesh(
            &mut world,
            Capsule3d::new(0.25, 1.25).mesh().build(),
            Vec3::new(0.0, 0.75, 0.0),
        );
        world.entity_mut(near).insert(Unit {
            id: 1,
            number: 1,
            ..default()
        });
        let far = spawn_mesh(
            &mut world,
            Capsule3d::new(0.25, 1.25).mesh().build(),
            Vec3::new(0.0, 0.75, -2.0),
        );
        world.entity_mut(far).insert(Unit {
            id: 2,
            number: 2,
            ..default()
        });
        assert_eq!(
            pick(
                &mut world,
                Ray3d::new(Vec3::new(0.0, 0.75, 5.0), Dir3::NEG_Z)
            ),
            Some(near)
        );
        assert_eq!(
            pick(
                &mut world,
                Ray3d::new(Vec3::new(5.0, 0.75, 5.0), Dir3::NEG_Z)
            ),
            None
        );
    }

    #[test]
    fn terrain_blocks_selection_of_units_behind_it() {
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        let unit = spawn_mesh(
            &mut world,
            Capsule3d::new(0.25, 1.25).mesh().build(),
            Vec3::new(0.0, 0.75, 0.0),
        );
        world.entity_mut(unit).insert(Unit {
            id: 1,
            number: 1,
            ..default()
        });
        spawn_mesh(
            &mut world,
            Cuboid::new(2.0, 2.0, 1.0).mesh().build(),
            Vec3::new(0.0, 0.75, 2.0),
        );
        assert_eq!(
            pick(
                &mut world,
                Ray3d::new(Vec3::new(0.0, 0.75, 5.0), Dir3::NEG_Z)
            ),
            None
        );
    }

    #[test]
    fn cmd_selection_adds_and_toggles_units_and_normal_click_replaces_it() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let third = world.spawn_empty().id();
        let mut selected = SelectedUnits::default();
        selected.select(first, false);
        selected.select(second, true);
        assert_eq!(selected.0, vec![first, second]);
        selected.select(first, true);
        assert_eq!(selected.0, vec![second]);
        selected.select(third, false);
        assert_eq!(selected.0, vec![third]);
    }

    #[test]
    fn rectangle_selects_only_owned_units_in_all_drag_directions() {
        let mut world = World::new();
        let inside = world.spawn_empty().id();
        let enemy = world.spawn_empty().id();
        let outside = world.spawn_empty().id();
        let boundary = world.spawn_empty().id();
        for (start, end) in [
            (Vec2::ZERO, Vec2::splat(10.0)),
            (Vec2::splat(10.0), Vec2::ZERO),
            (Vec2::new(0.0, 10.0), Vec2::new(10.0, 0.0)),
            (Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0)),
        ] {
            let mut selected = SelectedUnits(vec![outside]);
            selected.select_rectangle(
                start,
                end,
                false,
                [
                    (inside, true, Vec2::splat(5.0)),
                    (enemy, false, Vec2::splat(5.0)),
                    (outside, true, Vec2::new(20.0, 5.0)),
                    (boundary, true, Vec2::new(0.0, 10.0)),
                ],
            );
            assert_eq!(selected.0, vec![inside, boundary]);
        }
    }

    #[test]
    fn cmd_rectangle_adds_without_toggling_or_duplicating_selected_units() {
        let mut world = World::new();
        let previous = world.spawn_empty().id();
        let inside = world.spawn_empty().id();
        let mut selected = SelectedUnits(vec![previous, inside]);
        selected.select_rectangle(
            Vec2::ZERO,
            Vec2::splat(10.0),
            true,
            [(inside, true, Vec2::splat(5.0))],
        );
        assert_eq!(selected.0, vec![previous, inside]);
        selected.select_rectangle(Vec2::ZERO, Vec2::splat(10.0), false, []);
        assert!(selected.0.is_empty());
    }
}
