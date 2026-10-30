use crate::storage_models::MODELS;
use crate::*;
use stronghold_map::logistics as economy;
#[derive(Resource)]
pub(crate) struct StorageAssets {
    scenes: [Option<Handle<WorldAsset>>; MODELS.len()],
    loading: Option<usize>,
}
#[derive(Component)]
pub(crate) struct WarehouseVisual {
    id: u64,
    stock: [u32; 3],
    rotation: u8,
}
#[derive(Component)]
pub(crate) struct GranaryVisual {
    id: u64,
    stock: [u32; 4],
    rotation: u8,
}
#[derive(Component)]
pub(crate) struct CargoVisual {
    id: u64,
    resource: u8,
    amount: u32,
}
pub(crate) fn setup_storage_assets(mut commands: Commands) {
    commands.insert_resource(StorageAssets {
        scenes: std::array::from_fn(|_| None),
        loading: None,
    });
}
/// This queue is also bounded: do not reintroduce the glTF IO stack-overflow burst.
pub(crate) fn load_storage_assets(server: Res<AssetServer>, mut assets: ResMut<StorageAssets>) {
    if let Some(i) = assets.loading {
        let id = assets.scenes[i].as_ref().unwrap().id();
        if !server.is_loaded_with_dependencies(id)
            && !matches!(
                server.get_load_state(id),
                Some(bevy::asset::LoadState::Failed(_))
            )
        {
            return;
        }
        assets.loading = None;
    }
    if let Some(i) = assets.scenes.iter().position(Option::is_none) {
        assets.scenes[i] = Some(server.load(MODELS[i].path));
        assets.loading = Some(i);
    }
}
fn stock_transform(model: usize, cell: usize, height: f32) -> Transform {
    let (x, z) = economy::cell_position(cell);
    Transform::from_xyz(x, 0.221 + height - MODELS[model].base * 0.0085, z)
        .with_scale(Vec3::splat(0.0085))
}
fn food_transform(food: &economy::FoodVisual) -> Transform {
    let model = &MODELS[food.model];
    let scale = 0.01;
    Transform::from_xyz(
        food.position.0 - model.center[0] * scale,
        0.23 + food.layer as f32 * (model.height * scale + 0.02) - model.base * scale,
        food.position.1 - model.center[1] * scale,
    )
    .with_scale(Vec3::splat(scale))
}
pub(crate) fn sync_storage_visuals(
    mut commands: Commands,
    multiplayer: Res<Multiplayer>,
    assets: Res<StorageAssets>,
    warehouses: Query<(Entity, &WarehouseVisual)>,
    granaries: Query<(Entity, &GranaryVisual)>,
    mut cargo: Query<(Entity, &CargoVisual, &mut Transform)>,
) {
    let snapshot = &multiplayer.snapshot;
    let active = snapshot.connected && snapshot.game_id.is_some();
    let mut existing = std::collections::HashSet::new();
    for (entity, visual) in &warehouses {
        let row = active
            .then(|| {
                snapshot
                    .warehouses
                    .iter()
                    .find(|w| w.building_id == visual.id)
            })
            .flatten();
        let building = snapshot.buildings.iter().find(|b| b.id == visual.id);
        if row.zip(building).is_some_and(|(w, b)| {
            visual.stock == [w.wood, w.stone, w.iron] && visual.rotation == b.rotation
        }) {
            existing.insert(visual.id);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if active {
        for row in &snapshot.warehouses {
            if existing.contains(&row.building_id) {
                continue;
            }
            let Some(b) = snapshot.buildings.iter().find(|b| b.id == row.building_id) else {
                continue;
            };
            let stock = [row.wood, row.stone, row.iron];
            if economy::storage_cells(stock) > economy::STORAGE_CELLS as u64 {
                continue;
            }
            let models = economy::stock_visuals(stock);
            if models.iter().any(|p| assets.scenes[p.model].is_none()) {
                continue;
            }
            commands
                .spawn((
                    WarehouseVisual {
                        id: b.id,
                        stock,
                        rotation: b.rotation,
                    },
                    Transform::from_xyz(b.x, b.y, b.z).with_rotation(Quat::from_rotation_y(
                        b.rotation as f32 * std::f32::consts::FRAC_PI_2,
                    )),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for p in models {
                        parent.spawn((
                            WorldAssetRoot(assets.scenes[p.model].as_ref().unwrap().clone()),
                            stock_transform(p.model, p.cell, p.height),
                        ));
                    }
                });
        }
    }
    let mut existing_granaries = std::collections::HashSet::new();
    for (entity, visual) in &granaries {
        let row = snapshot
            .granaries
            .iter()
            .find(|s| s.building_id == visual.id);
        let building = snapshot.buildings.iter().find(|b| b.id == visual.id);
        if active
            && row.zip(building).is_some_and(|(s, b)| {
                visual.stock == [s.apples, s.bread, s.cheese, s.meat]
                    && visual.rotation == b.rotation
            })
        {
            existing_granaries.insert(visual.id);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if active {
        for row in &snapshot.granaries {
            if existing_granaries.contains(&row.building_id) {
                continue;
            }
            let Some(b) = snapshot.buildings.iter().find(|b| b.id == row.building_id) else {
                continue;
            };
            let stock = [row.apples, row.bread, row.cheese, row.meat];
            let models = economy::food_visuals(stock);
            if models.iter().any(|p| assets.scenes[p.model].is_none()) {
                continue;
            }
            commands
                .spawn((
                    GranaryVisual {
                        id: b.id,
                        stock,
                        rotation: b.rotation,
                    },
                    Transform::from_xyz(b.x, b.y, b.z).with_rotation(Quat::from_rotation_y(
                        b.rotation as f32 * std::f32::consts::FRAC_PI_2,
                    )),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for p in models {
                        parent.spawn((
                            WorldAssetRoot(assets.scenes[p.model].as_ref().unwrap().clone()),
                            food_transform(&p),
                        ));
                    }
                });
        }
    }
    let mut existing_cargo = std::collections::HashSet::new();
    for (entity, visual, mut transform) in &mut cargo {
        let worker = active
            .then(|| {
                snapshot.workers.iter().find(|w| {
                    w.unit_id == visual.id
                        && w.cargo == visual.amount
                        && w.resource == visual.resource
                        && w.cargo > 0
                })
            })
            .flatten();
        if let Some((_, unit)) = worker.zip(snapshot.units.iter().find(|u| u.id == visual.id)) {
            transform.translation = Vec3::new(unit.x, unit.y + 0.2, unit.z + 0.3);
            existing_cargo.insert(visual.id);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if active {
        for worker in snapshot
            .workers
            .iter()
            .filter(|w| w.cargo > 0 && !existing_cargo.contains(&w.unit_id))
        {
            let Some(unit) = snapshot.units.iter().find(|u| u.id == worker.unit_id) else {
                continue;
            };
            let model = match worker.resource {
                0 => 0,
                1 => 2,
                5 => 24,
                3 => 22,
                6 | 7 => 25,
                _ => 11,
            };
            let Some(scene) = &assets.scenes[model] else {
                continue;
            };
            let scale = if worker.resource == 0 { 0.003 } else { 0.008 };
            let m = &MODELS[model];
            commands
                .spawn((
                    CargoVisual {
                        id: unit.id,
                        resource: worker.resource,
                        amount: worker.cargo,
                    },
                    Transform::from_xyz(unit.x, unit.y + 0.2, unit.z + 0.3),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        WorldAssetRoot(scene.clone()),
                        Transform::from_xyz(
                            -m.center[0] * scale,
                            -m.base * scale,
                            -m.center[1] * scale,
                        )
                        .with_scale(Vec3::splat(scale)),
                    ));
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin, mesh::MeshPlugin};
    use std::time::{Duration, Instant};

    #[test]
    fn granary_replication_rebuilds_exact_food_stacks_and_cleans_up() {
        let mut app = App::new();
        app.init_resource::<Multiplayer>()
            .insert_resource(StorageAssets {
                scenes: std::array::from_fn(|_| Some(default())),
                loading: None,
            })
            .add_systems(Update, sync_storage_visuals);
        {
            let mut multiplayer = app.world_mut().resource_mut::<Multiplayer>();
            let snapshot = &mut multiplayer.snapshot;
            snapshot.connected = true;
            snapshot.game_id = Some(1);
            snapshot.buildings.push(network::Building {
                id: 7,
                kind: "granary".into(),
                rotation: 1,
                color: 0,
                health: 1.,
                owned: true,
                x: 20.,
                y: 0.,
                z: 30.,
            });
            snapshot.granaries.push(network::Granary {
                building_id: 7,
                apples: 5,
                bread: 5,
                cheese: 5,
                meat: 5,
            });
        }
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&WorldAssetRoot>()
                .iter(app.world())
                .count(),
            20
        );
        let transform = app
            .world_mut()
            .query_filtered::<&Transform, With<GranaryVisual>>()
            .single(app.world())
            .unwrap();
        assert!((transform.rotation * Vec3::X).distance(Vec3::NEG_Z) < 0.001);
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .granaries[0]
            .apples = 4;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&WorldAssetRoot>()
                .iter(app.world())
                .count(),
            19
        );
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .granaries
            .clear();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&WorldAssetRoot>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn storage_models_load_sequentially_and_fit_their_two_metre_cells() {
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
        .add_systems(Startup, setup_storage_assets)
        .add_systems(Update, load_storage_assets);
        app.finish();
        app.cleanup();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let before = app
                .world()
                .get_resource::<StorageAssets>()
                .map(|a| a.scenes.iter().filter(|h| h.is_some()).count())
                .unwrap_or(0);
            app.update();
            let assets = app.world().resource::<StorageAssets>();
            let after = assets.scenes.iter().filter(|h| h.is_some()).count();
            assert!(
                after <= before + 1,
                "Only one resource model may enter the IO queue per frame"
            );
            if after == MODELS.len() && assets.loading.is_none() {
                break;
            }
            assert!(Instant::now() < deadline, "Stockpile models failed to load");
            std::thread::sleep(Duration::from_millis(10));
        }
        for (i, model) in MODELS.iter().enumerate() {
            let server = app.world().resource::<AssetServer>();
            let mesh: Handle<Mesh> = server.load(format!(
                "{}#Mesh0/Primitive0",
                model.path.split('#').next().unwrap()
            ));
            loop {
                app.update();
                if app.world().resource::<Assets<Mesh>>().get(&mesh).is_some() {
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
            let meshes = app.world().resource::<Assets<Mesh>>();
            let positions = meshes
                .get(&mesh)
                .unwrap()
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            for p in positions {
                let point = stock_transform(i, 0, 0.).transform_point(Vec3::from(*p));
                assert!(
                    (point.x + 5.).abs() <= 1. && (point.z + 3.).abs() <= 1.,
                    "{} spills outside its cell",
                    model.path
                );
                assert!(point.y >= 0.22, "Goods must sit above the storage floor");
                if i >= 22 {
                    for food in economy::food_visuals([5; 4])
                        .iter()
                        .filter(|v| v.model == i)
                    {
                        let point = food_transform(food).transform_point(Vec3::from(*p));
                        assert!(
                            point.x.abs() < 2.6 && point.z.abs() < 2.6,
                            "Food must fit inside the granary walls"
                        );
                        assert!(point.y >= 0.229, "Food must sit above the granary floor");
                    }
                }
            }
        }
    }
}
