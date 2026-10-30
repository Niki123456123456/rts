use crate::buildings::terrain_height;
use crate::*;
use stronghold_map::{self as map, fortifications as fort, BuildingKind};
#[derive(Component)]
pub(crate) struct FortVisual;
#[derive(Component)]
pub(crate) struct GateVisual {
    id: u64,
    bridge: bool,
}
#[derive(Resource)]
pub(crate) struct FortAssets {
    cube: Handle<Mesh>,
    stone: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    water: Handle<StandardMaterial>,
    soil: Handle<StandardMaterial>,
}
pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let stone = materials.add(Color::srgb(0.58, 0.54, 0.44));
    let wood = materials.add(Color::srgb(0.32, 0.18, 0.08));
    let water = materials.add(StandardMaterial {
        base_color: Color::srgb(0.13, 0.33, 0.35),
        perceptual_roughness: 0.2,
        ..default()
    });
    let soil = materials.add(Color::srgb(0.24, 0.17, 0.09));
    commands.insert_resource(FortAssets {
        cube,
        stone,
        wood,
        water,
        soil,
    });
}
/// Match server collision, including gate pillars, traversable decks and dug moats.
pub(crate) fn obstacles(s: &network::Snapshot) -> Vec<map::Obstacle> {
    let mut result = Vec::new();
    let mut decks = Vec::new();
    for b in &s.buildings {
        let k = BuildingKind::from_name(&b.kind).unwrap_or_default();
        let open = s.gates.iter().any(|g| g.building_id == b.id && g.open);
        if k.is_gate() {
            result.extend(fort::gate_solids(k, (b.x, b.z), b.rotation, open));
            if k == BuildingKind::DrawbridgeGate && open {
                decks.push(fort::bridge_span((b.x, b.z), b.rotation));
            }
        } else {
            result.push(k.rotated_obstacle(b.x, b.z, b.rotation));
        }
    }
    let mut water = map::terrain::obstacles();
    water.extend(s.moats.iter().map(|m| map::Obstacle {
        x: m.x,
        z: m.z,
        half_x: 2.,
        half_z: 2.,
    }));
    for cut in decks {
        water = water
            .into_iter()
            .flat_map(|a| fort::subtract(a, cut))
            .collect();
    }
    result.extend(water);
    result.extend(
        s.stairs
            .iter()
            .map(|a| fort::stair_bounds((a.base_x, a.base_z), (a.top_x, a.top_z))),
    );
    result
}
pub(crate) fn sync(
    mut commands: Commands,
    m: Res<Multiplayer>,
    assets: Res<FortAssets>,
    old: Query<Entity, With<FortVisual>>,
    mut previous: Local<String>,
) {
    let s = &m.snapshot;
    let revision = format!(
        "{:?}:{:?}:{:?}:{:?}",
        s.game_id,
        s.moats
            .iter()
            .map(|m| (m.id, m.x.to_bits(), m.z.to_bits()))
            .collect::<Vec<_>>(),
        s.stairs.iter().map(|s| s.id).collect::<Vec<_>>(),
        s.buildings
            .iter()
            .filter(|b| BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_gate()))
            .map(|b| b.id)
            .collect::<Vec<_>>()
    );
    if *previous == revision {
        return;
    }
    *previous = revision;
    for e in &old {
        commands.entity(e).despawn();
    }
    for m in &s.moats {
        let y = terrain_height(m.x, m.z);
        for (h, yy, material) in [
            (0.12, y - 1.4, assets.water.clone()),
            (0.12, y - 1.8, assets.soil.clone()),
        ] {
            commands.spawn((
                FortVisual,
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(m.x, yy, m.z).with_scale(Vec3::new(4., h, 4.)),
                Ground,
            ));
        }
        // Soil banks are vertical faces down to the excavation floor; inner seams remain below water.
        for (x, z, w, d) in [
            (-2., 0., 0.12, 4.),
            (2., 0., 0.12, 4.),
            (0., -2., 4., 0.12),
            (0., 2., 4., 0.12),
        ] {
            let outside = (m.x + x * 1.05, m.z + z * 1.05);
            if s.moats.iter().any(|other| {
                other.id != m.id
                    && (other.x - outside.0).abs() < 2.
                    && (other.z - outside.1).abs() < 2.
            }) {
                continue;
            }
            commands.spawn((
                FortVisual,
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.soil.clone()),
                Transform::from_xyz(m.x + x, y - 0.9, m.z + z).with_scale(Vec3::new(w, 1.8, d)),
            ));
        }
    }
    for a in &s.stairs {
        let base = Vec3::new(a.base_x, a.base_y, a.base_z);
        let top = Vec3::new(a.top_x, a.top_y, a.top_z);
        let delta = top - base;
        let length = Vec2::new(delta.x, delta.z).length();
        let n = (delta.y / 0.22).ceil().max(1.) as usize;
        let rotation = Quat::from_rotation_y(delta.x.atan2(delta.z));
        for i in 0..n {
            let t = (i as f32 + 0.5) / n as f32;
            let h = delta.y * (i + 1) as f32 / n as f32;
            commands.spawn((
                FortVisual,
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.stone.clone()),
                Transform {
                    translation: base + Vec3::new(delta.x * t, h * 0.5, delta.z * t),
                    rotation,
                    scale: Vec3::new(1.6, h.max(0.1), length / n as f32 + 0.01),
                },
            ));
        }
    }
    for stair in &s.stairs {
        if let Some(wall) = s.buildings.iter().find(|b| b.id == stair.wall_id) {
            let delta = Vec2::new(wall.x - stair.top_x, wall.z - stair.top_z);
            let centre = Vec3::new(
                (wall.x + stair.top_x) * 0.5,
                stair.top_y - 0.08,
                (wall.z + stair.top_z) * 0.5,
            );
            commands.spawn((
                FortVisual,
                Mesh3d(assets.cube.clone()),
                MeshMaterial3d(assets.stone.clone()),
                Transform::from_translation(centre)
                    .with_rotation(Quat::from_rotation_y(delta.x.atan2(delta.y)))
                    .with_scale(Vec3::new(1.6, 0.16, delta.length() + 0.3)),
            ));
        }
    }
    for b in &s.buildings {
        let Some(k) = BuildingKind::from_name(&b.kind).filter(|k| k.is_gate()) else {
            continue;
        };
        let gate_width = (k.obstacle(0., 0.).half_x * 0.96).min(3.6);
        let angle = b.rotation as f32 * std::f32::consts::FRAC_PI_2;
        commands.spawn((
            FortVisual,
            GateVisual {
                id: b.id,
                bridge: false,
            },
            Mesh3d(assets.cube.clone()),
            MeshMaterial3d(assets.wood.clone()),
            Transform::from_xyz(b.x, b.y + 2., b.z)
                .with_rotation(Quat::from_rotation_y(angle))
                .with_scale(Vec3::new(gate_width, 4., 0.24)),
        ));
        if k == BuildingKind::DrawbridgeGate {
            commands
                .spawn((
                    FortVisual,
                    GateVisual {
                        id: b.id,
                        bridge: true,
                    },
                    Transform::from_xyz(b.x, b.y + 0.05, b.z)
                        .with_rotation(Quat::from_rotation_y(angle)),
                    Visibility::Inherited,
                ))
                .with_children(|p| {
                    for i in 0..64 {
                        p.spawn((
                            Mesh3d(assets.cube.clone()),
                            MeshMaterial3d(assets.wood.clone()),
                            Transform::from_xyz(0., 0.12, 3. + (i as f32 + 0.5) * 0.5)
                                .with_scale(Vec3::new(3.8, 0.24, 0.48)),
                        ));
                    }
                    for x in [-1.7, 1.7] {
                        p.spawn((
                            Mesh3d(assets.cube.clone()),
                            MeshMaterial3d(assets.wood.clone()),
                            Transform::from_xyz(x, -0.12, 19.)
                                .with_scale(Vec3::new(0.18, 0.5, 32.)),
                        ));
                    }
                });
        }
    }
}
pub(crate) fn animate(
    m: Res<Multiplayer>,
    time: Res<Time>,
    mut gates: Query<(&GateVisual, &mut Transform)>,
) {
    for (g, mut t) in &mut gates {
        let Some(b) = m.snapshot.buildings.iter().find(|b| b.id == g.id) else {
            continue;
        };
        let open = m
            .snapshot
            .gates
            .iter()
            .any(|s| s.building_id == g.id && s.open);
        let f = (time.delta_secs() * 4.).min(1.);
        if g.bridge {
            let target = Quat::from_rotation_y(b.rotation as f32 * std::f32::consts::FRAC_PI_2)
                * Quat::from_rotation_x(if open {
                    0.
                } else {
                    -std::f32::consts::FRAC_PI_2
                });
            t.rotation = t.rotation.slerp(target, f);
        } else {
            t.translation.y += (b.y + if open { 6.4 } else { 2. } - t.translation.y) * f;
        }
    }
}
pub(crate) fn input(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<RtsCamera>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BuildingInteraction>,
    mut m: ResMut<Multiplayer>,
    mut contexts: EguiContexts,
    mut gizmos: Gizmos,
) {
    let Some(tool) = state.fort_tool else { return };
    state.consumed = true;
    if keys.just_pressed(KeyCode::Escape) || mouse.just_pressed(MouseButton::Right) {
        state.fort_tool = None;
        state.fort_start = None;
        return;
    }
    if contexts
        .ctx_mut()
        .is_ok_and(|c| c.is_pointer_over_egui() || c.egui_wants_pointer_input())
    {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.0.viewport_to_world(camera.1, cursor) else {
        return;
    };
    if ray.direction.y >= -0.001 {
        return;
    }
    let mut d = -ray.origin.y / ray.direction.y;
    for _ in 0..3 {
        let p = ray.get_point(d);
        d = (terrain_height(p.x, p.z) - ray.origin.y) / ray.direction.y;
    }
    let p = ray.get_point(d);
    let point = (p.x, p.z);
    if let Some(a) = state.fort_start {
        let y = terrain_height(a.0, a.1);
        gizmos.line(
            Vec3::new(a.0, y + 0.2, a.1),
            p + Vec3::Y * 0.2,
            Color::srgb(0.4, 0.85, 1.),
        );
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if state.fort_start.is_none() {
        state.fort_start = Some(point);
        return;
    }
    let a = state.fort_start.take().unwrap();
    if tool == 0 {
        m.send(Action::DigMoat {
            start_x: a.0,
            start_z: a.1,
            end_x: point.0,
            end_z: point.1,
        });
    } else {
        let wall = m
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.owned && BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_wall()))
            .filter_map(|b| {
                let roof = buildings::roof_geometry(b)?;
                let d = (roof.roof_y() - ray.origin.y) / ray.direction.y;
                let hit = ray.get_point(d);
                roof.roof_contains((hit.x, hit.z))
                    .then_some((b.id, (hit.x, hit.z), d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        if let Some((wall_id, top, _)) = wall {
            m.send(Action::BuildWallStair {
                wall_id,
                base_x: a.0,
                base_z: a.1,
                top_x: top.0,
                top_z: top.1,
            });
        } else {
            state.fort_start = Some(a);
        }
    }
}
