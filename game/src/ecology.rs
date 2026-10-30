//! Replicated living resources, animals and working ox carts.
use crate::*;
use std::collections::{HashMap, HashSet};
use stronghold_map::{self as map, ecology as nature};
#[derive(Resource, Default)]
pub(crate) struct EcologyInteraction {
    selected: Option<u64>,
}
#[derive(Resource)]
pub(crate) struct NatureAssets {
    cube: Handle<Mesh>,
    sphere: Handle<Mesh>,
    wheel: Handle<Mesh>,
    bodies: Vec<Handle<Mesh>>,
    skins: Vec<Handle<StandardMaterial>>,
    wood: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    horn: Handle<StandardMaterial>,
    fish: Handle<StandardMaterial>,
}
#[derive(Component)]
pub(crate) struct AnimalVisual {
    pub id: u64,
    species: u8,
}
#[derive(Component)]
pub(crate) struct CartVisual {
    home: u64,
}
#[derive(Component)]
pub(crate) struct FishVisual {
    site: u32,
    index: u32,
}
#[derive(Component)]
pub(crate) struct DepositVisual {
    pub site: u32,
    pub original: Transform,
}
#[derive(Component)]
pub(crate) struct AnimalLeg {
    owner: u64,
    index: u32,
    base: Vec3,
}
pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let skins = [
        (0.53, 0.36, 0.20),
        (0.57, 0.49, 0.38),
        (0.30, 0.22, 0.14),
        (0.38, 0.40, 0.39),
        (0.30, 0.18, 0.10),
        (0.48, 0.35, 0.21),
    ]
    .into_iter()
    .map(|(r, g, b)| mats.add(Color::srgb(r, g, b)))
    .collect();
    let bodies = [
        (0.65, 0.75, 1.3),
        (0.28, 0.25, 0.5),
        (0.7, 0.65, 1.1),
        (0.5, 0.65, 1.15),
        (1.1, 1.1, 1.7),
    ]
    .into_iter()
    .map(|(x, y, z)| meshes.add(Cuboid::new(x, y, z)))
    .collect();
    commands.insert_resource(NatureAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        sphere: meshes.add(Sphere::new(1.).mesh().ico(1).unwrap()),
        wheel: meshes.add(Cylinder::new(0.28, 0.12).mesh().resolution(12)),
        bodies,
        skins,
        wood: mats.add(Color::srgb(0.29, 0.15, 0.06)),
        iron: mats.add(Color::srgb(0.13, 0.15, 0.16)),
        horn: mats.add(Color::srgb(0.82, 0.77, 0.61)),
        fish: mats.add(Color::srgb(0.28, 0.72, 0.77)),
    });
}
fn body_height(species: u8) -> f32 {
    match species {
        0 => 1.,
        1 => 0.3,
        2 | 3 => 0.65,
        _ => 0.95,
    }
}
pub(crate) fn sync_animals(
    mut commands: Commands,
    time: Res<Time>,
    network: Res<Multiplayer>,
    assets: Res<NatureAssets>,
    mut query: Query<(Entity, &AnimalVisual, &mut Transform)>,
) {
    let alive = network.snapshot.game_id.is_some();
    let rows = &network.snapshot.wildlife;
    let obs: Vec<_> = network
        .snapshot
        .buildings
        .iter()
        .filter_map(|b| {
            map::BuildingKind::from_name(&b.kind).map(|k| k.rotated_obstacle(b.x, b.z, b.rotation))
        })
        .chain(map::terrain::obstacles())
        .collect();
    let mut existing = HashSet::new();
    for (e, v, mut t) in &mut query {
        let row = alive.then(|| rows.iter().find(|a| a.id == v.id)).flatten();
        let Some(a) = row.filter(|a| a.health > 0. || a.meat > 0) else {
            commands.entity(e).despawn();
            continue;
        };
        existing.insert(a.id);
        let from = (t.translation.x, t.translation.z);
        let goal = (a.x, a.z);
        let mut next = goal;
        if map::distance(from, goal) > 0.01 {
            if !map::segment_clear(from, goal, &obs) {
                next = map::shortest_path(from, goal, &obs)
                    .and_then(|p| p.first().copied())
                    .unwrap_or(from);
            }
            let d = map::distance(from, next);
            let step = (time.delta_secs() * 6.).min(d);
            if d > 0. {
                next = (
                    from.0 + (next.0 - from.0) * step / d,
                    from.1 + (next.1 - from.1) * step / d,
                );
            }
            let direction = Vec2::new(next.0 - from.0, next.1 - from.1);
            if direction.length_squared() > 0.0001 {
                t.rotation = Quat::from_rotation_y(direction.x.atan2(direction.y));
            }
        }
        t.translation = Vec3::new(
            next.0,
            map::terrain::height(next.0, next.1) + body_height(a.species),
            next.1,
        );
        if a.health <= 0. {
            t.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
            t.translation.y = map::terrain::height(a.x, a.z) + 0.25;
        }
    }
    for a in rows
        .iter()
        .filter(|a| alive && (a.health > 0. || a.meat > 0) && !existing.contains(&a.id))
    {
        let species = a.species as usize;
        let height = body_height(a.species);
        commands
            .spawn((
                AnimalVisual {
                    id: a.id,
                    species: a.species,
                },
                Mesh3d(assets.bodies[species].clone()),
                MeshMaterial3d(assets.skins[species].clone()),
                Transform::from_xyz(a.x, map::terrain::height(a.x, a.z) + height, a.z),
            ))
            .with_children(|p| {
                let small = if species == 1 { 0.35 } else { 1. };
                let head = Vec3::new(0., 0.28 * small, 0.8 * small);
                p.spawn((
                    Mesh3d(assets.sphere.clone()),
                    MeshMaterial3d(assets.skins[species].clone()),
                    Transform::from_translation(head)
                        .with_scale(Vec3::new(0.25, 0.3, 0.38) * small),
                ));
                for side in [-1., 1.] {
                    p.spawn((
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(assets.skins[species].clone()),
                        Transform::from_translation(
                            head + Vec3::new(0.16 * side * small, 0.29 * small, 0.),
                        )
                        .with_scale(
                            Vec3::new(0.1, if species == 1 { 0.65 } else { 0.2 }, 0.12) * small,
                        ),
                    ));
                    p.spawn((
                        Mesh3d(assets.sphere.clone()),
                        MeshMaterial3d(assets.iron.clone()),
                        Transform::from_translation(
                            head + Vec3::new(0.18 * side * small, 0.08 * small, 0.27 * small),
                        )
                        .with_scale(Vec3::splat(0.035 * small)),
                    ));
                    if species == 0 {
                        for branch in 0..3 {
                            p.spawn((
                                Mesh3d(assets.cube.clone()),
                                MeshMaterial3d(assets.horn.clone()),
                                Transform::from_translation(
                                    head + Vec3::new(
                                        side * (0.2 + branch as f32 * 0.08),
                                        0.55 + branch as f32 * 0.1,
                                        -0.12,
                                    ),
                                )
                                .with_scale(Vec3::new(0.045, 0.55 - branch as f32 * 0.1, 0.045))
                                .with_rotation(Quat::from_rotation_z(side * 0.4)),
                            ));
                        }
                    }
                    if species == 2 {
                        p.spawn((
                            Mesh3d(assets.cube.clone()),
                            MeshMaterial3d(assets.horn.clone()),
                            Transform::from_translation(head + Vec3::new(side * 0.18, -0.1, 0.28))
                                .with_scale(Vec3::new(0.05, 0.2, 0.06)),
                        ));
                    }
                }
                for i in 0..4u32 {
                    let base = Vec3::new(
                        if i % 2 == 0 { -0.22 } else { 0.22 } * small,
                        -height * 0.5,
                        if i < 2 { -0.42 } else { 0.42 } * small,
                    );
                    p.spawn((
                        AnimalLeg {
                            owner: a.id,
                            index: i,
                            base,
                        },
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(assets.skins[species].clone()),
                        Transform::from_translation(base).with_scale(Vec3::new(
                            0.12 * small,
                            height,
                            0.14 * small,
                        )),
                    ));
                }
                p.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(assets.skins[species].clone()),
                    Transform::from_xyz(0., 0.05, -0.75 * small)
                        .with_scale(
                            Vec3::new(0.12, 0.14, if species == 3 { 0.6 } else { 0.25 }) * small,
                        )
                        .with_rotation(Quat::from_rotation_x(0.35)),
                ));
            });
    }
}
pub(crate) fn animate_legs(
    time: Res<Time>,
    network: Res<Multiplayer>,
    mut q: Query<(&AnimalLeg, &mut Transform)>,
) {
    for (leg, mut t) in &mut q {
        let moving = network
            .snapshot
            .wildlife
            .iter()
            .any(|a| a.id == leg.owner && a.health > 0. && a.state != 0);
        t.rotation = Quat::from_rotation_x(if moving {
            (time.elapsed_secs() * 9. + leg.index as f32 * 2.4).sin() * 0.45
        } else {
            0.
        });
        t.translation = leg.base;
    }
}
fn parked_cart(s: &network::Snapshot, home: &network::Building) -> map::Point {
    let shape = map::BuildingKind::from_name(&home.kind)
        .unwrap_or_default()
        .rotated_obstacle(home.x, home.z, home.rotation);
    let mut obs = crate::fortifications::obstacles(s);
    for a in &mut obs {
        a.half_x += 1.25;
        a.half_z += 1.25;
    }
    map::attack_positions(shape)
        .into_iter()
        .filter(|a| a.ring >= 2)
        .find(|a| map::free(a.point, &obs))
        .map_or((home.x, home.z), |a| a.point)
}
pub(crate) fn sync_carts(
    mut commands: Commands,
    network: Res<Multiplayer>,
    assets: Res<NatureAssets>,
    mut q: Query<(Entity, &CartVisual, &mut Transform)>,
    mut workers: Query<(&Unit, &mut Visibility)>,
) {
    let mut existing = HashSet::new();
    for (e, v, mut t) in &mut q {
        let cart = network
            .snapshot
            .ox_carts
            .iter()
            .find(|c| c.building_id == v.home);
        let home = network.snapshot.buildings.iter().find(|b| b.id == v.home);
        let Some((c, h)) = cart.zip(home) else {
            commands.entity(e).despawn();
            continue;
        };
        existing.insert(v.home);
        let worker = c
            .source_building
            .and_then(|s| {
                network
                    .snapshot
                    .workers
                    .iter()
                    .find(|w| w.source_building == s)
            })
            .and_then(|w| network.snapshot.units.iter().find(|u| u.id == w.unit_id));
        let point = worker.map_or_else(|| parked_cart(&network.snapshot, h), |u| (u.x, u.z));
        let d = Vec2::new(point.0 - t.translation.x, point.1 - t.translation.z);
        if d.length_squared() > 0.0001 {
            t.rotation = Quat::from_rotation_y(d.x.atan2(d.y));
        }
        t.translation = Vec3::new(
            point.0,
            worker.map_or_else(|| map::terrain::height(point.0, point.1), |u| u.y - 0.75),
            point.1,
        );
    }
    for c in network
        .snapshot
        .ox_carts
        .iter()
        .filter(|c| !existing.contains(&c.building_id))
    {
        let Some(home) = network
            .snapshot
            .buildings
            .iter()
            .find(|b| b.id == c.building_id)
        else {
            continue;
        };
        commands
            .spawn((
                CartVisual {
                    home: c.building_id,
                },
                Transform::from_xyz(
                    home.x + 2.,
                    map::terrain::height(home.x + 2., home.z + 2.),
                    home.z + 2.,
                ),
                Visibility::default(),
            ))
            .with_children(|p| {
                // Wagon, high sides, axle, two visible wheels and the ox in its yoke.
                for (pos, size) in [
                    (Vec3::new(0., 0.55, -0.65), Vec3::new(0.95, 0.14, 1.15)),
                    (Vec3::new(-0.46, 0.8, -0.65), Vec3::new(0.08, 0.5, 1.15)),
                    (Vec3::new(0.46, 0.8, -0.65), Vec3::new(0.08, 0.5, 1.15)),
                    (Vec3::new(0., 0.8, -1.18), Vec3::new(0.9, 0.5, 0.08)),
                    (Vec3::new(0., 0.4, -0.6), Vec3::new(1.2, 0.12, 0.12)),
                    (Vec3::new(0., 0.7, 0.2), Vec3::new(1., 0.08, 0.08)),
                ] {
                    p.spawn((
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(assets.wood.clone()),
                        Transform::from_translation(pos).with_scale(size),
                    ));
                }
                for side in [-1., 1.] {
                    p.spawn((
                        Mesh3d(assets.wheel.clone()),
                        MeshMaterial3d(assets.wood.clone()),
                        Transform::from_xyz(side * 0.58, 0.3, -0.65)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    ));
                }
                for (pos, size) in [
                    (Vec3::new(0., 0.73, 0.75), Vec3::new(0.65, 0.7, 0.85)),
                    (Vec3::new(0., 0.85, 1.25), Vec3::new(0.45, 0.45, 0.4)),
                    (Vec3::new(-0.23, 0.3, 0.55), Vec3::new(0.13, 0.6, 0.13)),
                    (Vec3::new(0.23, 0.3, 0.55), Vec3::new(0.13, 0.6, 0.13)),
                    (Vec3::new(-0.23, 0.3, 1.), Vec3::new(0.13, 0.6, 0.13)),
                    (Vec3::new(0.23, 0.3, 1.), Vec3::new(0.13, 0.6, 0.13)),
                ] {
                    p.spawn((
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(assets.skins[5].clone()),
                        Transform::from_translation(pos).with_scale(size),
                    ));
                }
                for side in [-1., 1.] {
                    p.spawn((
                        Mesh3d(assets.cube.clone()),
                        MeshMaterial3d(assets.horn.clone()),
                        Transform::from_xyz(side * 0.3, 1.08, 1.25)
                            .with_scale(Vec3::new(0.35, 0.07, 0.07))
                            .with_rotation(Quat::from_rotation_z(side * 0.4)),
                    ));
                }
            });
    }
    for (u, mut v) in &mut workers {
        if u.worker {
            let cart = network
                .snapshot
                .workers
                .iter()
                .find(|w| w.unit_id == u.id)
                .is_some_and(|w| {
                    network
                        .snapshot
                        .ox_carts
                        .iter()
                        .any(|c| c.source_building == Some(w.source_building))
                });
            *v = if cart {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
}
pub(crate) fn sync_fish(
    mut commands: Commands,
    time: Res<Time>,
    network: Res<Multiplayer>,
    assets: Res<NatureAssets>,
    mut q: Query<(Entity, &FishVisual, &mut Transform)>,
) {
    let mut existing = HashSet::new();
    for (e, v, mut t) in &mut q {
        let Some(n) = network
            .snapshot
            .resource_nodes
            .iter()
            .find(|n| n.site == v.site && n.kind == 7 && n.remaining.div_ceil(5) > v.index)
        else {
            commands.entity(e).despawn();
            continue;
        };
        existing.insert((v.site, v.index));
        let phase = time.elapsed_secs() * 0.45 + v.index as f32 * 2.4;
        let z = n.z + phase.sin() * 9.;
        let x = map::terrain::river_center(z) + (phase * 0.7).cos() * 5.;
        t.translation = Vec3::new(x, -0.46, z);
        t.rotation = Quat::from_rotation_y(phase);
    }
    for n in network
        .snapshot
        .resource_nodes
        .iter()
        .filter(|n| n.kind == 7)
    {
        for i in 0..(n.remaining.div_ceil(5)).min(16) {
            if existing.contains(&(n.site, i)) {
                continue;
            }
            commands.spawn((
                FishVisual {
                    site: n.site,
                    index: i,
                },
                Mesh3d(assets.sphere.clone()),
                MeshMaterial3d(assets.fish.clone()),
                Transform::from_xyz(n.x, -0.46, n.z).with_scale(Vec3::new(0.18, 0.12, 0.48)),
            ));
        }
    }
}
pub(crate) fn sync_terrain(
    time: Res<Time>,
    network: Res<Multiplayer>,
    mut trees: Query<
        (&terrain::TerrainTree, &mut Transform, &mut Visibility),
        Without<DepositVisual>,
    >,
    mut rocks: Query<
        (&DepositVisual, &mut Transform, &mut Visibility),
        Without<terrain::TerrainTree>,
    >,
    mut fall_started: Local<HashMap<u32, f32>>,
) {
    let nodes: HashMap<_, _> = network
        .snapshot
        .resource_nodes
        .iter()
        .map(|n| (n.site, n))
        .collect();
    for (t, mut transform, mut visible) in &mut trees {
        let Some(n) = nodes.get(&t.site) else {
            *transform = t.original;
            continue;
        };
        *transform = t.original;
        if n.remaining == 0 {
            *visible = Visibility::Hidden;
            continue;
        }
        if n.phase == 1 || n.phase == 2 {
            let began = *fall_started
                .entry(n.site)
                .or_insert(time.elapsed_secs() - n.phase_ticks as f32);
            let fraction = if n.phase == 2 {
                1.
            } else {
                ((time.elapsed_secs() - began) / 2.).clamp(0., 1.)
            };
            if t.crown && n.phase == 2 {
                *visible = Visibility::Hidden;
                continue;
            }
            let rotation = Quat::from_rotation_z(fraction * std::f32::consts::FRAC_PI_2);
            let base = Vec3::new(t.x, map::terrain::height(t.x, t.z), t.z);
            transform.translation = base + rotation * (t.original.translation - base);
            transform.rotation = rotation;
            if !t.crown && n.phase == 2 {
                transform.scale.y *= n.remaining as f32 / n.capacity as f32;
            }
        }
    }
    for (v, mut t, mut visible) in &mut rocks {
        let ratio = nodes
            .get(&v.site)
            .map_or(1., |n| n.remaining as f32 / n.capacity.max(1) as f32);
        *t = v.original;
        t.scale *= ratio.cbrt();
        *visible = if ratio == 0. {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
pub(crate) fn input(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mut raycast: MeshRayCast,
    animals: Query<&AnimalVisual>,
    ground: Query<(), With<Ground>>,
    colliders: Query<(), With<buildings::BuildingCollider>>,
    units: Query<&Unit>,
    selected: Res<SelectedUnits>,
    mut network: ResMut<Multiplayer>,
    mut state: ResMut<EcologyInteraction>,
    mut interaction: ResMut<BuildingInteraction>,
    mut contexts: EguiContexts,
) {
    if interaction.placing.is_some() || network.snapshot.game_id.is_none() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) && !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if contexts
        .ctx_mut()
        .is_ok_and(|c| c.egui_wants_pointer_input())
    {
        return;
    }
    let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor) else {
        return;
    };
    let filter = |e| animals.contains(e) || ground.contains(e) || colliders.contains(e);
    let target = raycast
        .cast_ray(ray, &MeshRayCastSettings::default().with_filter(&filter))
        .first()
        .and_then(|(e, _)| animals.get(*e).ok())
        .map(|v| v.id);
    if let Some(id) = target {
        state.selected = Some(id);
        interaction.consumed = true;
        if mouse.just_pressed(MouseButton::Right) {
            let ids = selected
                .0
                .iter()
                .filter_map(|e| units.get(*e).ok())
                .filter(|u| u.owned && !u.worker)
                .map(|u| u.id)
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                network.send(Action::HuntAnimal {
                    unit_ids: ids,
                    animal_id: id,
                });
            }
        }
    } else if mouse.just_pressed(MouseButton::Left) {
        state.selected = None;
    }
}
pub(crate) fn animal_ui(
    mut contexts: EguiContexts,
    mut network: ResMut<Multiplayer>,
    mut state: ResMut<EcologyInteraction>,
    units: Query<&Unit>,
    selected: Res<SelectedUnits>,
) -> Result {
    let Some(a) = state
        .selected
        .and_then(|id| network.snapshot.wildlife.iter().find(|a| a.id == id))
        .cloned()
    else {
        return Ok(());
    };
    let mut open = true;
    egui::Window::new(nature::species_name(a.species)).open(&mut open).default_pos(egui::pos2(20.,450.)).resizable(false).show(contexts.ctx_mut()?,|ui|{
  let pack=network.snapshot.wildlife.iter().filter(|b|b.group_id==a.group_id&&b.health>0.).count();ui.label(format!("Gruppe: {pack} Tiere · Gesundheit {:0.0}%",a.health/nature::animal_health(a.species)*100.));
  if a.health<=0.{ui.label(format!("Wildfleisch am Kadaver: {}",a.meat));}else{ui.label(if nature::predator(a.species){"Vorsicht: Verteidigt sein Revier. Offene Siedlungen können Ziel eines Streifzugs sein."}else{"Friedliches Wild · Jägerhütten jagen in ihrer Umgebung."});
   let ids=selected.0.iter().filter_map(|e|units.get(*e).ok()).filter(|u|u.owned&&!u.worker).map(|u|u.id).collect::<Vec<_>>();if ui.add_enabled(!ids.is_empty(),egui::Button::new("Mit ausgewählten Truppen jagen")).clicked(){network.send(Action::HuntAnimal{unit_ids:ids,animal_id:a.id});}
  }
 });
    if !open {
        state.selected = None;
    }
    Ok(())
}
pub(crate) fn production_details(
    ui: &mut egui::Ui,
    b: &network::Building,
    network: &mut Multiplayer,
) {
    let kind = match b.kind.as_str() {
        "saw_pit" => Some(0),
        "quarry" => Some(1),
        "iron_mine" => Some(2),
        "fishpond" => Some(7),
        _ => None,
    };
    if let Some(k) = kind {
        let node = network
            .snapshot
            .resource_nodes
            .iter()
            .filter(|n| n.kind == k && (n.remaining > 0 || k != 0))
            .filter(|n| map::distance((b.x, b.z), (n.x, n.z)) < if k == 0 { 40. } else { 35. })
            .min_by(|a, c| {
                map::distance((b.x, b.z), (a.x, a.z))
                    .total_cmp(&map::distance((b.x, b.z), (c.x, c.z)))
            });
        ui.small(node.map_or("Bestand erschöpft".into(), |n| {
            format!(
                "Bestand {} / {}{}",
                n.remaining,
                n.capacity,
                if k == 7 {
                    " · langsame Regeneration"
                } else if n.phase == 1 {
                    " · Baum fällt"
                } else if n.phase == 2 {
                    " · Stamm wird zerhackt"
                } else {
                    ""
                }
            )
        }));
    }
    if matches!(b.kind.as_str(), "quarry" | "iron_mine" | "saw_pit") {
        let cart = network
            .snapshot
            .ox_carts
            .iter()
            .any(|c| c.source_building == Some(b.id));
        ui.small(if cart {
            "Ochsenkarren: bis zu 20 Einheiten je Fahrt"
        } else if b.kind == "saw_pit" {
            "Zu Fuß: 5 Holz je Fahrt"
        } else {
            "Transport benötigt einen Ochsenkarren"
        });
        if b.owned {
            let available = network.snapshot.ox_carts.iter().any(|c| {
                c.source_building.is_none()
                    && network
                        .snapshot
                        .buildings
                        .iter()
                        .any(|h| h.id == c.building_id && h.owned)
            });
            if ui.add_enabled(cart||available,egui::Button::new(if cart{"Karren freigeben"}else{"Karren zuweisen"})).on_hover_text("Jeder Ochsenplatz stellt einen Karren. Ein Karren kann genau einen Betrieb versorgen.").clicked(){network.send(if cart{Action::ReleaseOxCart{building_id:b.id}}else{Action::AssignOxCart{building_id:b.id}});}
        }
    }
    if b.kind == "hunters_post" {
        ui.small("Jagt Wild im Umkreis von 80 m. Auch Raubtiere können gejagt werden.");
    }
}
