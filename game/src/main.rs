//! A terrain scene with stationary units and an RTS camera.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext};

use bevy_rts_camera::{Ground, RtsCamera, RtsCameraControls, RtsCameraPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((MeshPickingPlugin, EguiPlugin::default()))
        .add_plugins(RtsCameraPlugin)
        .init_resource::<SelectedUnit>()
        .init_resource::<SelectionPanel>()
        .add_systems(Startup, setup)
        .add_systems(Update, (select_unit, highlight_selection).chain())
        .add_systems(EguiPrimaryContextPass, selection_ui)
        .run();
}

#[derive(Component)]
struct Unit {
    number: usize,
}

#[derive(Resource, Default)]
struct SelectedUnit(Option<Entity>);

#[derive(Resource, Default)]
struct SelectionPanel(Option<egui::Rect>);

#[derive(Resource)]
struct UnitMaterials {
    normal: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
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
    // Some generic units that are not part of the 'Ground' (ignored for height calculation)
    let selected_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.75, 0.15),
        emissive: LinearRgba::new(0.3, 0.15, 0.0, 1.0),
        ..default()
    });
    commands.insert_resource(UnitMaterials {
        normal: terrain_material.clone(),
        selected: selected_material,
    });
    for x in -5..5 {
        for z in -5..5 {
            commands.spawn((
                Unit {
                    number: ((x + 5) * 10 + (z + 5) + 1) as usize,
                },
                Mesh3d(meshes.add(Capsule3d::new(0.25, 1.25))),
                MeshMaterial3d(terrain_material.clone()),
                Transform::from_xyz(x as f32 * 0.7, 0.75, z as f32 * 0.7),
            ));
        }
    }
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
            // Rotate the camera with right click
            button_rotate: MouseButton::Right,
            // Keep the mouse cursor in place when rotating
            lock_on_rotate: true,
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
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mut ray_cast: MeshRayCast,
    units: Query<(), With<Unit>>,
    mut selected: ResMut<SelectedUnit>,
    mut contexts: EguiContexts,
    panel: Res<SelectionPanel>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // UI clicks must not change the selection in the world behind the panel.
    if let Ok(ctx) = contexts.ctx_mut() {
        let ui_position =
            egui::pos2(cursor.x, cursor.y) * (window.scale_factor() / ctx.pixels_per_point());
        if panel.0.is_some_and(|rect| rect.contains(ui_position)) {
            return;
        }
    }
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    // Cast against the actual visible geometry, independent of egui's picking events.
    selected.0 = unit_at_ray(ray, &mut ray_cast, &units);
}

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

fn highlight_selection(
    selected: Res<SelectedUnit>,
    materials: Res<UnitMaterials>,
    mut units: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>), With<Unit>>,
) {
    if !selected.is_changed() {
        return;
    }
    for (entity, mut material) in &mut units {
        material.0 = if selected.0 == Some(entity) {
            materials.selected.clone()
        } else {
            materials.normal.clone()
        };
    }
}

fn selection_ui(
    mut contexts: EguiContexts,
    mut selected: ResMut<SelectedUnit>,
    units: Query<&Unit>,
    mut controls: Query<&mut RtsCameraControls>,
    mut panel: ResMut<SelectionPanel>,
) -> Result {
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
                ui.heading("Auswahl");
                ui.separator();
                if let Some(unit) = selected.0.and_then(|entity| units.get(entity).ok()) {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // A capsule portrait matching the units in the scene.
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(44.0, 64.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                rect.shrink2(egui::vec2(10.0, 4.0)),
                                12.0,
                                egui::Color32::from_rgb(255, 191, 38),
                            );
                            ui.vertical(|ui| {
                                ui.strong(format!("Einheit {}", unit.number));
                                ui.label("Ausgewählt");
                            });
                        });
                    });
                    if ui.button("Auswahl aufheben").clicked() {
                        selected.0 = None;
                    }
                } else {
                    ui.label("Klicke auf eine Einheit, um sie auszuwählen.");
                }
            });
        });
    panel.0 = Some(response.response.rect);

    // Prevent edge pan, zoom and rotation while interacting with the UI.
    let over_panel = ctx
        .pointer_interact_pos()
        .is_some_and(|position| response.response.rect.contains(position));
    for mut controls in &mut controls {
        controls.enabled =
            !over_panel && !ctx.egui_wants_pointer_input() && !ctx.egui_wants_keyboard_input();
    }
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
        world.entity_mut(near).insert(Unit { number: 1 });
        let far = spawn_mesh(
            &mut world,
            Capsule3d::new(0.25, 1.25).mesh().build(),
            Vec3::new(0.0, 0.75, -2.0),
        );
        world.entity_mut(far).insert(Unit { number: 2 });
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
        world.entity_mut(unit).insert(Unit { number: 1 });
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
}
