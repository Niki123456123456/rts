//! Campfire and visible job seekers driven by the server's immigration count.
use crate::*;
use stronghold_map::BuildingKind;

/// Wait for the game subscription's own keep, then show the starting home once.
pub(crate) fn focus_home(
    multiplayer: Res<Multiplayer>,
    mut cameras: Query<&mut RtsCamera>,
    mut focused: Local<Option<(u64, u64)>>,
) {
    let Some(game) = multiplayer
        .snapshot
        .game_id
        .filter(|_| multiplayer.snapshot.connected)
    else {
        *focused = None;
        return;
    };
    let Some(keep) = multiplayer
        .snapshot
        .buildings
        .iter()
        .find(|b| b.owned && b.kind == "keep")
    else {
        return;
    };
    if *focused == Some((game, keep.id)) {
        return;
    }
    for mut camera in &mut cameras {
        camera.target_focus.translation = world(keep, Vec3::new(0., 2., 5.));
        camera.target_zoom = 0.2;
        camera.snap = true;
    }
    *focused = Some((game, keep.id));
}

#[derive(Resource)]
pub(crate) struct CampAssets {
    cube: Handle<Mesh>,
    head: Handle<Mesh>,
    flame: Handle<Mesh>,
    smoke: Handle<Mesh>,
    cloth: Vec<Handle<StandardMaterial>>,
    skin: Handle<StandardMaterial>,
    boots: Handle<StandardMaterial>,
    fire: Handle<StandardMaterial>,
    smoke_material: Handle<StandardMaterial>,
}
#[derive(Component)]
pub(crate) struct CampFire {
    building: u64,
    index: u8,
}
#[derive(Component)]
pub(crate) struct Citizen {
    building: u64,
    index: u32,
    age: f32,
    point: Option<stronghold_map::Point>,
    path: Vec<stronghold_map::Point>,
    revision: u64,
}
#[derive(Default)]
pub(crate) struct CampNavigation {
    revision: u64,
    obstacles: Vec<stronghold_map::Obstacle>,
    spots: std::collections::HashMap<u64, Vec<stronghold_map::Point>>,
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cloth = [
        (0.45, 0.26, 0.13),
        (0.26, 0.36, 0.44),
        (0.51, 0.46, 0.29),
        (0.34, 0.37, 0.20),
    ]
    .into_iter()
    .map(|(r, g, b)| materials.add(Color::srgb(r, g, b)))
    .collect();
    commands.insert_resource(CampAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        head: meshes.add(Sphere::new(1.).mesh().ico(1).unwrap()),
        flame: meshes.add(Cone::new(0.36, 1.1).mesh().resolution(7)),
        smoke: meshes.add(Sphere::new(1.).mesh().ico(1).unwrap()),
        cloth,
        skin: materials.add(Color::srgb(0.71, 0.47, 0.29)),
        boots: materials.add(Color::srgb(0.15, 0.10, 0.06)),
        fire: materials.add(StandardMaterial {
            base_color: Color::srgb(1., 0.43, 0.02),
            emissive: LinearRgba::rgb(5., 1.1, 0.02),
            unlit: true,
            ..default()
        }),
        smoke_material: materials.add(StandardMaterial {
            base_color: Color::srgba(0.39, 0.37, 0.32, 0.18),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    });
}
pub(crate) fn sync_camp(
    mut commands: Commands,
    multiplayer: Res<Multiplayer>,
    assets: Res<CampAssets>,
    fires: Query<(Entity, &CampFire)>,
    citizens: Query<(Entity, &Citizen)>,
) {
    let keeps = multiplayer
        .snapshot
        .buildings
        .iter()
        .filter(|b| b.kind == "keep" && multiplayer.snapshot.game_id.is_some())
        .collect::<Vec<_>>();
    for (entity, fire) in &fires {
        if !keeps.iter().any(|b| b.id == fire.building) {
            commands.entity(entity).despawn();
        }
    }
    for keep in &keeps {
        if !fires.iter().any(|(_, f)| f.building == keep.id) {
            for index in 0..9 {
                commands.spawn((
                    CampFire {
                        building: keep.id,
                        index,
                    },
                    Mesh3d(if index < 4 {
                        assets.flame.clone()
                    } else {
                        assets.smoke.clone()
                    }),
                    MeshMaterial3d(if index < 4 {
                        assets.fire.clone()
                    } else {
                        assets.smoke_material.clone()
                    }),
                    Transform::default(),
                    bevy::light::NotShadowCaster,
                ));
            }
        }
    }
    let waiting = |keep: &network::Building| {
        let jobs = resources::employed(&multiplayer, keep.color);
        multiplayer
            .snapshot
            .keep_population
            .iter()
            .find(|k| k.building_id == keep.id)
            .map_or(0, |k| k.arrived.saturating_sub(jobs))
            .min(16)
    };
    for (entity, citizen) in &citizens {
        if !keeps
            .iter()
            .any(|b| b.id == citizen.building && citizen.index < waiting(b))
        {
            commands.entity(entity).despawn();
        }
    }
    for keep in keeps {
        for index in 0..waiting(keep) {
            if citizens
                .iter()
                .any(|(_, c)| c.building == keep.id && c.index == index)
            {
                continue;
            }
            let state = multiplayer
                .snapshot
                .keep_population
                .iter()
                .find(|k| k.building_id == keep.id)
                .unwrap();
            // Late/reconnecting clients show old arrivals already at the fire.
            let age = if state.seconds > (index + 1) * 8 + 6 {
                8.
            } else {
                (state.seconds % 8) as f32
            };
            commands
                .spawn((
                    Citizen {
                        building: keep.id,
                        index,
                        age,
                        point: None,
                        path: Vec::new(),
                        revision: u64::MAX,
                    },
                    Transform::from_xyz(keep.x, keep.y, keep.z),
                    Visibility::default(),
                ))
                .with_children(|parent| {
                    for (position, scale, material, head) in [
                        (
                            Vec3::new(0., 1.05, 0.),
                            Vec3::new(0.42, 0.65, 0.29),
                            assets.cloth[index as usize % 4].clone(),
                            false,
                        ),
                        (
                            Vec3::new(0., 1.58, 0.),
                            Vec3::splat(0.21),
                            assets.skin.clone(),
                            true,
                        ),
                        (
                            Vec3::new(-0.13, 0.36, 0.),
                            Vec3::new(0.17, 0.72, 0.18),
                            assets.boots.clone(),
                            false,
                        ),
                        (
                            Vec3::new(0.13, 0.36, 0.),
                            Vec3::new(0.17, 0.72, 0.18),
                            assets.boots.clone(),
                            false,
                        ),
                        (
                            Vec3::new(-0.30, 0.98, 0.),
                            Vec3::new(0.16, 0.62, 0.18),
                            assets.skin.clone(),
                            false,
                        ),
                        (
                            Vec3::new(0.30, 0.98, 0.),
                            Vec3::new(0.16, 0.62, 0.18),
                            assets.skin.clone(),
                            false,
                        ),
                        (
                            Vec3::new(0., 1.77, 0.),
                            Vec3::new(0.42, 0.12, 0.40),
                            assets.cloth[index as usize % 4].clone(),
                            false,
                        ),
                    ] {
                        parent.spawn((
                            Mesh3d(if head {
                                assets.head.clone()
                            } else {
                                assets.cube.clone()
                            }),
                            MeshMaterial3d(material),
                            Transform::from_translation(position).with_scale(scale),
                        ));
                    }
                });
        }
    }
}
fn world(keep: &network::Building, local: Vec3) -> Vec3 {
    Vec3::new(keep.x, keep.y, keep.z)
        + Quat::from_rotation_y(keep.rotation as f32 * std::f32::consts::FRAC_PI_2) * local
}
pub(crate) fn animate_fire(
    time: Res<Time>,
    multiplayer: Res<Multiplayer>,
    mut query: Query<(&CampFire, &mut Transform)>,
) {
    for (fire, mut transform) in &mut query {
        let Some(keep) = multiplayer
            .snapshot
            .buildings
            .iter()
            .find(|b| b.id == fire.building)
        else {
            continue;
        };
        let t = time.elapsed_secs();
        let i = fire.index as f32;
        let local = if fire.index < 4 {
            let phase = t * 7. + i * 1.7;
            transform.scale = Vec3::new(
                0.65 + 0.12 * phase.sin(),
                0.72 + 0.22 * (phase * 0.73).sin(),
                0.65,
            );
            Vec3::new(
                0.28 * i.cos(),
                0.72 + 0.12 * phase.sin(),
                11. + 0.28 * i.sin(),
            )
        } else {
            let phase = (t * 0.3 + i * 0.23).fract();
            transform.scale = Vec3::splat(0.17 + phase * 0.44);
            Vec3::new(
                0.18 * (t + i).sin() + phase * 0.9,
                1.1 + phase * 3.6,
                11. + phase * 0.28,
            )
        };
        transform.translation = world(keep, local);
    }
}
pub(crate) fn animate_citizens(
    time: Res<Time>,
    multiplayer: Res<Multiplayer>,
    mut cache: Local<CampNavigation>,
    mut query: Query<(&mut Citizen, &mut Transform, &mut Visibility)>,
) {
    use stronghold_map::{self as map, building_roof::BuildingRoof};
    let buildings = &multiplayer.snapshot.buildings;
    let revision = buildings.iter().fold(
        multiplayer
            .snapshot
            .gates
            .iter()
            .fold(multiplayer.snapshot.moats.len() as u64 + 1, |h, g| {
                h.wrapping_mul(31) + u64::from(g.open)
            }),
        |h, b| {
            h.wrapping_mul(31).wrapping_add(b.id)
                ^ (b.x.to_bits() as u64).rotate_left(17)
                ^ (b.z.to_bits() as u64).rotate_left(33)
                ^ u64::from(b.rotation)
        },
    );
    if revision != cache.revision {
        cache.revision = revision;
        cache.obstacles = crate::fortifications::obstacles(&multiplayer.snapshot);
        cache.spots = buildings
            .iter()
            .filter(|b| b.kind == "keep")
            .map(|b| {
                let keep =
                    BuildingRoof::new(BuildingKind::Keep, b.x, b.y, b.z, b.rotation).unwrap();
                (b.id, map::camp::spots(keep, &cache.obstacles, 16))
            })
            .collect();
    }
    for (mut citizen, mut transform, mut visibility) in &mut query {
        let Some(keep) = buildings.iter().find(|b| b.id == citizen.building) else {
            continue;
        };
        if citizen.revision != revision {
            citizen.revision = revision;
            let Some(goal) = cache
                .spots
                .get(&keep.id)
                .and_then(|s| s.get(citizen.index as usize))
                .copied()
            else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let roof = BuildingRoof::new(BuildingKind::Keep, keep.x, keep.y, keep.z, keep.rotation)
                .unwrap();
            let route =
                if let Some(point) = citizen.point.filter(|p| map::free(*p, &cache.obstacles)) {
                    map::shortest_path(point, goal, &cache.obstacles).map(|path| (point, path))
                } else if citizen.age >= 6. {
                    Some((goal, Vec::new()))
                } else {
                    map::camp::arrival(roof, goal, &cache.obstacles)
                };
            let Some((point, path)) = route else {
                *visibility = Visibility::Hidden;
                continue;
            };
            citizen.point = Some(point);
            citizen.path = path;
            *visibility = Visibility::Inherited;
        }
        if *visibility == Visibility::Hidden {
            continue;
        }
        let Some(mut point) = citizen.point else {
            continue;
        };
        let old = point;
        let mut budget = time.delta_secs() * 1.6;
        while budget > 0.0001 {
            let Some(next) = citizen.path.first().copied() else {
                break;
            };
            if !map::segment_clear(point, next, &cache.obstacles) {
                break;
            }
            let distance = map::distance(point, next);
            if distance <= budget {
                point = next;
                budget -= distance;
                citizen.path.remove(0);
            } else {
                point = (
                    point.0 + (next.0 - point.0) * budget / distance,
                    point.1 + (next.1 - point.1) * budget / distance,
                );
                budget = 0.;
            }
        }
        citizen.point = Some(point);
        citizen.age += time.delta_secs();
        let walking = map::distance(old, point) > 0.00001;
        let fire = world(keep, Vec3::new(0., 0., 11.));
        let facing = if walking {
            (point.0 - old.0, point.1 - old.1)
        } else {
            (fire.x - point.0, fire.z - point.1)
        };
        transform.translation = Vec3::new(
            point.0,
            map::terrain::height(point.0, point.1)
                + if walking {
                    0.045 * (time.elapsed_secs() * 11.).sin().abs()
                } else {
                    0.
                },
            point.1,
        );
        transform.rotation = Quat::from_rotation_y(facing.0.atan2(facing.1));
    }
}
