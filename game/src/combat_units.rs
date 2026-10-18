use crate::*;
use bevy::{gltf::Gltf, world_serialization::WorldInstanceReady};
use std::time::Duration;
#[derive(Resource, Default)]
pub(crate) struct CombatAssets {
    loading: Option<Handle<Gltf>>,
    models: Vec<CombatModel>,
}
struct CombatModel {
    scene: Handle<WorldAsset>,
    graph: Handle<AnimationGraph>,
    clips: Vec<AnimationNodeIndex>,
}
#[derive(Component)]
pub(crate) struct CombatVisual {
    owner: Entity,
    kind: usize,
}
#[derive(Component)]
pub(crate) struct RigPlayer {
    owner: Entity,
    kind: usize,
    activity: u8,
}
const PATHS: [&str; 2] = [
    "models/units/lord/mesh.glb",
    "models/units/armed_peasant/mesh.glb",
];
fn kind_index(kind: &str) -> usize {
    usize::from(kind == "armed_peasant")
}
pub(crate) fn load_combat_models(
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut assets: ResMut<CombatAssets>,
) {
    if let Some(handle) = &assets.loading {
        let Some(gltf) = gltfs
            .get(handle)
            .filter(|_| server.is_loaded_with_dependencies(handle.id()))
        else {
            return;
        };
        let (graph, clips) = AnimationGraph::from_clips(gltf.animations.iter().cloned());
        assets.models.push(CombatModel {
            scene: gltf.scenes[0].clone(),
            graph: graphs.add(graph),
            clips,
        });
        assets.loading = None;
    }
    if assets.models.len() < PATHS.len() && assets.loading.is_none() {
        assets.loading = Some(server.load(PATHS[assets.models.len()]));
    }
}
pub(crate) fn sync_combat_visuals(
    mut commands: Commands,
    assets: Res<CombatAssets>,
    units: Query<(Entity, &Unit)>,
    mut visuals: Query<(Entity, &CombatVisual, &mut Transform)>,
) {
    let mut existing = std::collections::HashSet::new();
    for (entity, v, mut transform) in &mut visuals {
        if let Some((_, u)) = units
            .get(v.owner)
            .ok()
            .filter(|(_, u)| !u.worker && kind_index(&u.kind) == v.kind)
        {
            existing.insert(v.owner);
            if u.facing.length_squared() > 0.01 {
                // The imported rigs face +Z, including their attached weapons.
                transform.rotation = Quat::from_rotation_y(u.facing.x.atan2(u.facing.y));
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (owner, u) in units
        .iter()
        .filter(|(e, u)| !u.worker && !existing.contains(e))
    {
        let kind = kind_index(&u.kind);
        let Some(model) = assets.models.get(kind) else {
            continue;
        };
        let (scale, base) = if kind == 0 {
            (0.007653524, -0.12237549)
        } else {
            (0.00802919, -1.5127563)
        };
        commands.entity(owner).with_children(|parent| {
            parent
                .spawn((
                    CombatVisual { owner, kind },
                    WorldAssetRoot(model.scene.clone()),
                    Transform::from_xyz(0., -0.75 - base * scale, 0.)
                        .with_rotation(Quat::from_rotation_y(u.facing.x.atan2(u.facing.y)))
                        .with_scale(Vec3::splat(scale)),
                ))
                .observe(rig_ready);
        });
    }
}
fn rig_ready(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    visuals: Query<&CombatVisual>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    assets: Res<CombatAssets>,
) {
    let Ok(v) = visuals.get(ready.entity) else {
        return;
    };
    for entity in children.iter_descendants(ready.entity) {
        if players.contains(entity) {
            commands.entity(entity).insert((
                AnimationGraphHandle(assets.models[v.kind].graph.clone()),
                AnimationTransitions::new(),
                RigPlayer {
                    owner: v.owner,
                    kind: v.kind,
                    activity: 255,
                },
            ));
        }
    }
}
pub(crate) fn animate_combat_units(
    assets: Res<CombatAssets>,
    units: Query<&Unit>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut RigPlayer,
    )>,
) {
    for (mut player, mut transitions, mut rig) in &mut players {
        let Ok(u) = units.get(rig.owner) else {
            continue;
        };
        let activity = u.activity.min(2);
        if rig.activity != activity {
            transitions
                .play(
                    &mut player,
                    assets.models[rig.kind].clips[activity as usize],
                    Duration::from_millis(140),
                )
                .repeat();
            rig.activity = activity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        animation::AnimationPlugin, asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin,
        mesh::MeshPlugin, world_serialization::WorldSerializationPlugin,
    };
    use std::time::Instant;
    #[test]
    fn both_rigs_have_three_playable_states_and_weapons_follow_animated_hands() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            },
            TransformPlugin,
            MeshPlugin,
            ImagePlugin::default(),
            AnimationPlugin,
            WorldSerializationPlugin,
            GltfPlugin::default(),
        ))
        .init_resource::<CombatAssets>()
        .add_systems(
            Update,
            (
                load_combat_models,
                sync_combat_visuals,
                animate_combat_units,
            )
                .chain(),
        );
        app.finish();
        app.cleanup();
        let lord = app
            .world_mut()
            .spawn((
                Unit {
                    kind: "lord".into(),
                    health: 1.,
                    ..default()
                },
                Transform::default(),
            ))
            .id();
        let peasant = app
            .world_mut()
            .spawn((
                Unit {
                    kind: "armed_peasant".into(),
                    health: 1.,
                    ..default()
                },
                Transform::default(),
            ))
            .id();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            app.update();
            if app
                .world_mut()
                .query::<&RigPlayer>()
                .iter(app.world())
                .count()
                == 2
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Both animated rig instances should load"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(1. / 60.),
        ));
        for owner in [lord, peasant] {
            let mut descendants = vec![owner];
            let mut index = 0;
            while index < descendants.len() {
                let e = descendants[index];
                if let Some(children) = app.world().get::<Children>(e) {
                    descendants.extend(children.iter());
                }
                index += 1;
            }
            let (weapon, hand) = app
                .world_mut()
                .query::<(Entity, &Name, &ChildOf)>()
                .iter(app.world())
                .find(|(e, n, _)| descendants.contains(e) && n.as_str().starts_with("Weapon_"))
                .map(|(e, _, p)| (e, p.parent()))
                .expect("Weapon attached to each rig");
            assert_eq!(
                app.world().get::<Name>(hand).unwrap().as_str(),
                "Bip01 R Hand"
            );
            for state in 0..=2 {
                app.world_mut().get_mut::<Unit>(owner).unwrap().activity = state;
                app.update();
                let before = *app.world().get::<GlobalTransform>(weapon).unwrap();
                for _ in 0..15 {
                    std::thread::sleep(Duration::from_millis(10));
                    app.update();
                }
                let after = app.world().get::<GlobalTransform>(weapon).unwrap();
                assert_ne!(
                    before.to_matrix(),
                    after.to_matrix(),
                    "Weapon must move with the animated hand in state {state}"
                );
                // Sample more than a complete cycle, after the transition:
                // the sword blade and fork tines must lean toward model +Z.
                for _ in 0..180 {
                    app.update();
                    let weapon_transform = app.world().get::<GlobalTransform>(weapon).unwrap();
                    let blade_direction = weapon_transform.rotation() * Vec3::Y;
                    assert!(
                        blade_direction.z > 0.05,
                        "Weapon points backward in state {state}: {blade_direction:?}"
                    );
                }
            }
        }
    }
}

pub(crate) fn unit_health_ui(
    mut contexts: EguiContexts,
    units: Query<(&Unit, &Transform)>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let scale = window.scale_factor() / ctx.pixels_per_point();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("combat_health"),
    ));
    let (camera, transform) = *camera;
    for (unit, position) in &units {
        if unit.worker || (unit.health >= 0.999 && unit.activity != 2) {
            continue;
        }
        let Ok(p) = camera.world_to_viewport(transform, position.translation + Vec3::Y * 1.25)
        else {
            continue;
        };
        let rect =
            egui::Rect::from_center_size(egui::pos2(p.x * scale, p.y * scale), egui::vec2(34., 5.));
        painter.rect_filled(rect, 1., egui::Color32::from_black_alpha(220));
        let mut fill = rect.shrink(1.);
        fill.max.x = fill.min.x + fill.width() * unit.health.clamp(0., 1.);
        painter.rect_filled(
            fill,
            0.,
            if unit.owned {
                egui::Color32::LIGHT_GREEN
            } else {
                egui::Color32::LIGHT_RED
            },
        );
    }
    Ok(())
}
