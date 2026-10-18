use crate::*;
use bevy::{gltf::Gltf, world_serialization::WorldInstanceReady};
use std::time::Duration;

#[derive(Resource, Default)]
pub(crate) struct DairyAssets {
    loading: Option<Handle<Gltf>>,
    cow: Option<Handle<WorldAsset>>,
    fence: Option<Handle<WorldAsset>>,
    graph: Handle<AnimationGraph>,
    clips: Vec<AnimationNodeIndex>,
}

pub(crate) fn load_assets(
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut assets: ResMut<DairyAssets>,
) {
    if assets.cow.is_none() {
        if assets.loading.is_none() {
            assets.loading = Some(server.load("models/animals/dairy/cow.glb"));
        }
        if let Some(gltf) = assets
            .loading
            .as_ref()
            .filter(|h| server.is_loaded_with_dependencies(h.id()))
            .and_then(|h| gltfs.get(h))
        {
            let (graph, clips) = AnimationGraph::from_clips(gltf.animations.iter().cloned());
            assets.cow = Some(gltf.scenes[0].clone());
            assets.graph = graphs.add(graph);
            assets.clips = clips;
            assets.loading = None;
        }
    } else if assets.fence.is_none() {
        assets.fence = Some(server.load("models/animals/dairy/fence.glb#Scene0"));
    }
}

#[derive(Component)]
pub(crate) struct DairyVisual {
    id: u64,
    rotation: u8,
}

#[derive(Component)]
pub(crate) struct Cow {
    position: Vec2,
    target: Vec2,
    wait: f32,
    random: u64,
    walking: bool,
}
impl Cow {
    fn random(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random >> 32) as u32 as f32 / u32::MAX as f32
    }
    fn choose_target(&mut self, peers: &[Vec2]) {
        for _ in 0..16 {
            let point = Vec2::new(-5.5 + 11. * self.random(), 2. + 3.7 * self.random());
            if self.position.distance(point) > 1. && peers.iter().all(|p| p.distance(point) >= 2.7)
            {
                self.target = point;
                return;
            }
        }
        self.wait = 1.;
    }
    fn step(&mut self, dt: f32, peers: &[Vec2]) -> Vec2 {
        self.walking = false;
        if self.wait > 0. {
            self.wait -= dt;
            return Vec2::ZERO;
        }
        let delta = self.target - self.position;
        if delta.length() < 0.03 {
            self.choose_target(peers);
            self.wait = 1. + self.random() * 4.;
            return Vec2::ZERO;
        }
        let step = delta.normalize() * (0.55 * dt).min(delta.length());
        let next = self.position + step;
        if peers.iter().any(|p| p.distance(next) < 2.7) {
            self.choose_target(peers);
            self.wait = 0.6;
            return Vec2::ZERO;
        }
        self.position = next;
        self.walking = true;
        step
    }
}

#[derive(Component)]
struct CowScene(Entity);
#[derive(Component)]
pub(crate) struct CowRig {
    owner: Entity,
    activity: usize,
}

pub(crate) fn sync_farms(
    mut commands: Commands,
    multiplayer: Res<Multiplayer>,
    assets: Res<DairyAssets>,
    farms: Query<(Entity, &DairyVisual)>,
) {
    let snapshot = &multiplayer.snapshot;
    let active = snapshot.connected && snapshot.game_id.is_some();
    let mut existing = std::collections::HashSet::new();
    for (entity, visual) in &farms {
        if active
            && snapshot.buildings.iter().any(|b| {
                b.id == visual.id && b.kind == "dairy_farm" && b.rotation == visual.rotation
            })
        {
            existing.insert(visual.id);
        } else {
            commands.entity(entity).despawn();
        }
    }
    let Some((cow, fence)) = assets
        .cow
        .as_ref()
        .zip(assets.fence.as_ref())
        .filter(|_| active)
    else {
        return;
    };
    for b in snapshot
        .buildings
        .iter()
        .filter(|b| b.kind == "dairy_farm" && !existing.contains(&b.id))
    {
        commands
            .spawn((
                DairyVisual {
                    id: b.id,
                    rotation: b.rotation,
                },
                Transform::from_xyz(b.x, b.y, b.z).with_rotation(Quat::from_rotation_y(
                    b.rotation as f32 * std::f32::consts::FRAC_PI_2,
                )),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((WorldAssetRoot(fence.clone()), Transform::default()));
                for (index, position) in
                    [Vec2::new(-4., 2.5), Vec2::new(0., 4.6), Vec2::new(4., 2.5)]
                        .into_iter()
                        .enumerate()
                {
                    parent
                        .spawn((
                            Cow {
                                position,
                                target: position,
                                wait: 0.5 + index as f32,
                                random: (b.id.wrapping_mul(0x9e3779b97f4a7c15)
                                    ^ (index as u64 + 1))
                                    .max(1),
                                walking: false,
                            },
                            Transform::from_xyz(position.x, 0., position.y),
                            Visibility::default(),
                        ))
                        .with_children(|cow_parent| {
                            let owner = cow_parent.target_entity();
                            cow_parent
                                .spawn((
                                    CowScene(owner),
                                    WorldAssetRoot(cow.clone()),
                                    Transform::from_xyz(0., 0.02043204, -0.2649318)
                                        .with_scale(Vec3::splat(0.01)),
                                ))
                                .observe(cow_ready);
                        });
                }
            });
    }
}

fn cow_ready(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    scenes: Query<&CowScene>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    assets: Res<DairyAssets>,
) {
    let Ok(scene) = scenes.get(ready.entity) else {
        return;
    };
    for entity in children.iter_descendants(ready.entity) {
        if players.contains(entity) {
            commands.entity(entity).insert((
                AnimationGraphHandle(assets.graph.clone()),
                AnimationTransitions::new(),
                CowRig {
                    owner: scene.0,
                    activity: usize::MAX,
                },
            ));
        }
    }
}

pub(crate) fn wander(
    time: Res<Time>,
    mut cows: Query<(Entity, &mut Cow, &mut Transform, &ChildOf)>,
) {
    let peers: Vec<_> = cows
        .iter()
        .map(|(e, c, _, parent)| (e, parent.parent(), c.position))
        .collect();
    let dt = time.delta_secs().min(0.1);
    for (entity, mut cow, mut transform, parent) in &mut cows {
        let others: Vec<_> = peers
            .iter()
            .filter(|(e, p, _)| *e != entity && *p == parent.parent())
            .map(|(_, _, p)| *p)
            .collect();
        let step = cow.step(dt, &others);
        transform.translation.x = cow.position.x;
        transform.translation.z = cow.position.y;
        if step.length_squared() > 0.000001 {
            let facing = Quat::from_rotation_y(step.x.atan2(step.y));
            transform.rotation = transform.rotation.slerp(facing, (dt * 6.).min(1.));
        }
    }
}

pub(crate) fn animate(
    assets: Res<DairyAssets>,
    cows: Query<&Cow>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions, &mut CowRig)>,
) {
    for (mut player, mut transitions, mut rig) in &mut players {
        let Ok(cow) = cows.get(rig.owner) else {
            continue;
        };
        let activity = usize::from(cow.walking);
        if rig.activity != activity && assets.clips.len() > activity {
            transitions
                .play(
                    &mut player,
                    assets.clips[activity],
                    Duration::from_millis(180),
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
    fn textured_cows_and_fence_load_animate_rotate_and_disappear_with_the_farm() {
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
        .init_resource::<Multiplayer>()
        .init_resource::<DairyAssets>()
        .add_systems(Update, (load_assets, sync_farms, animate).chain());
        app.finish();
        app.cleanup();
        {
            let mut multiplayer = app.world_mut().resource_mut::<Multiplayer>();
            multiplayer.snapshot.connected = true;
            multiplayer.snapshot.game_id = Some(1);
            multiplayer.snapshot.buildings.push(network::Building {
                id: 7,
                kind: "dairy_farm".into(),
                rotation: 1,
                color: 0,
                health: 1.,
                owned: true,
                x: 20.,
                y: 0.,
                z: 30.,
            });
        }
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            app.update();
            let assets = app.world().resource::<DairyAssets>();
            if assets.fence.as_ref().is_some_and(|h| {
                app.world()
                    .resource::<AssetServer>()
                    .is_loaded_with_dependencies(h.id())
            }) && app.world_mut().query::<&CowRig>().iter(app.world()).count() == 3
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Cow rigs and fence must load with embedded textures"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(app.world().resource::<DairyAssets>().clips.len(), 2);
        let parent = app
            .world_mut()
            .query_filtered::<&Transform, With<DairyVisual>>()
            .single(app.world())
            .unwrap();
        assert!((parent.rotation * Vec3::X).distance(Vec3::NEG_Z) < 0.001);
        let leg = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, n)| n.as_str() == "Bip01 L Thigh")
            .unwrap()
            .0;
        let before = app.world().get::<Transform>(leg).unwrap().rotation;
        for mut cow in app
            .world_mut()
            .query::<&mut Cow>()
            .iter_mut(app.world_mut())
        {
            cow.walking = true;
        }
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs_f64(1. / 60.),
        ));
        for _ in 0..25 {
            app.update();
        }
        let after = app.world().get::<Transform>(leg).unwrap().rotation;
        assert!(
            before.angle_between(after) > 0.01,
            "Walking must animate the cow's legs"
        );
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings
            .clear();
        app.update();
        assert_eq!(app.world_mut().query::<&Cow>().iter(app.world()).count(), 0);
        assert_eq!(
            app.world_mut().query::<&CowRig>().iter(app.world()).count(),
            0
        );
    }

    #[test]
    fn wandering_stays_in_the_pasture_and_avoids_other_cows() {
        let mut cow = Cow {
            position: Vec2::new(-4., 2.5),
            target: Vec2::new(-4., 2.5),
            wait: 0.,
            random: 12345,
            walking: false,
        };
        let peers = [Vec2::new(0., 4.6), Vec2::new(4., 2.5)];
        let start = cow.position;
        let mut distance = 0.;
        let mut idle = false;
        for _ in 0..30000 {
            let before = cow.position;
            cow.step(0.05, &peers);
            distance += before.distance(cow.position);
            idle |= !cow.walking;
            assert!(cow.position.x >= -5.5 && cow.position.x <= 5.5);
            assert!(cow.position.y >= 2. && cow.position.y <= 5.7);
            assert!(peers.iter().all(|p| p.distance(cow.position) >= 2.6));
            assert!(before.distance(cow.position) <= 0.02751);
        }
        assert!(distance > 50. && cow.position.distance(start) > 0.5 && idle);
    }
}
