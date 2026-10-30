use crate::*;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use stronghold_map::terrain::{self as land, Biome};

#[derive(Component)]
pub(crate) struct TerrainTree {
    pub(crate) x: f32,
    pub(crate) z: f32,
    radius: f32,
    pub(crate) site: u32,
    pub(crate) crown: bool,
    pub(crate) original: Transform,
}

pub(crate) fn sync_clearings(
    multiplayer: Res<Multiplayer>,
    mut trees: Query<(&TerrainTree, &mut Visibility)>,
    mut previous: Local<Option<(Option<u64>, Vec<network::TerrainClearing>)>>,
) {
    let game = multiplayer
        .snapshot
        .game_id
        .filter(|_| multiplayer.snapshot.connected);
    let clearings: &[network::TerrainClearing] = if game.is_some() {
        &multiplayer.snapshot.clearings
    } else {
        &[]
    };
    if previous
        .as_ref()
        .is_some_and(|(old_game, old)| *old_game == game && old.as_slice() == clearings)
    {
        return;
    }
    for (tree, mut visibility) in &mut trees {
        let cleared = clearings.iter().any(|c| {
            stronghold_map::Obstacle {
                x: c.x,
                z: c.z,
                half_x: c.half_x,
                half_z: c.half_z,
            }
            .contains((tree.x, tree.z), tree.radius)
        });
        *visibility = if cleared {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    *previous = Some((game, clearings.to_vec()));
}

fn colored_mesh(positions: Vec<[f32; 3]>, colors: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let normals: Vec<[f32; 3]> = positions
        .iter()
        .map(|p| {
            Vec3::new(
                land::height(p[0] - 1.0, p[2]) - land::height(p[0] + 1.0, p[2]),
                2.0,
                land::height(p[0], p[2] - 1.0) - land::height(p[0], p[2] + 1.0),
            )
            .normalize()
            .to_array()
        })
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn tint(rgb: [u8; 3]) -> [f32; 4] {
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
        .to_linear()
        .to_f32_array()
}

#[derive(Component)]
pub(crate) struct TerrainChunk {
    x: usize,
    z: usize,
}

pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.85, 0.92, 1.0),
        brightness: 300.0,
        ..default()
    });
    let ground = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.95,
        ..default()
    });
    // Small terrain tiles keep ground ray casting and view culling inexpensive.
    const CELLS: usize = 40;
    for tile_z in 0..10 {
        for tile_x in 0..10 {
            let mut positions = Vec::new();
            let mut colors = Vec::new();
            let mut indices = Vec::new();
            for z in 0..=CELLS {
                for x in 0..=CELLS {
                    let wx = -400.0 + (tile_x * CELLS + x) as f32 * land::GRID;
                    let wz = -400.0 + (tile_z * CELLS + z) as f32 * land::GRID;
                    positions.push([wx, land::height(wx, wz), wz]);
                    let mut color = land::color(wx, wz);
                    // Water is a separate surface above a sandy riverbed.
                    if land::biome(wx, wz) == Biome::River {
                        color = [128, 117, 81];
                    }
                    colors.push(tint(color));
                }
            }
            for z in 0..CELLS {
                for x in 0..CELLS {
                    let a = (z * (CELLS + 1) + x) as u32;
                    let b = a + 1;
                    let c = a + (CELLS + 1) as u32;
                    indices.extend_from_slice(&[a, c, b, b, c, c + 1]);
                }
            }
            commands.spawn((
                TerrainChunk {
                    x: tile_x,
                    z: tile_z,
                },
                Mesh3d(meshes.add(colored_mesh(positions, colors, indices))),
                MeshMaterial3d(ground.clone()),
                Ground,
            ));
        }
    }
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for row in 0..=400 {
        let z = -400.0 + row as f32 * 2.0;
        let center = land::river_center(z);
        positions.push([center - 11.0, -0.65, z]);
        positions.push([center + 11.0, -0.65, z]);
        if row < 400 {
            let a = (row * 2) as u32;
            indices.extend_from_slice(&[a, a + 2, a + 1, a + 1, a + 2, a + 3]);
        }
    }
    let water_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 1.0, 0.0]; positions.len()],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices));
    commands.spawn((
        Mesh3d(meshes.add(water_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.40, 0.59),
            perceptual_roughness: 0.5,
            reflectance: 0.12,
            metallic: 0.0,
            ..default()
        })),
    ));

    let trunk = meshes.add(Cylinder::new(0.32, 3.6).mesh().resolution(6));
    let pine = meshes.add(Cone::new(2.4, 5.6).mesh().resolution(7));
    let crown = meshes.add(Sphere::new(2.5).mesh().ico(1).unwrap());
    let bark = materials.add(Color::srgb(0.30, 0.19, 0.10));
    let foliage = [
        materials.add(Color::srgb(0.16, 0.36, 0.13)),
        materials.add(Color::srgb(0.24, 0.44, 0.15)),
        materials.add(Color::srgb(0.11, 0.29, 0.18)),
    ];
    for row in 0..88_i32 {
        for column in 0..88_i32 {
            let hash = (column as u32).wrapping_mul(73856093) ^ (row as u32).wrapping_mul(19349663);
            let x = -392.0 + column as f32 * 9.0 + (hash % 51) as f32 * 0.1;
            let z = -392.0 + row as f32 * 9.0 + ((hash >> 8) % 51) as f32 * 0.1;
            if land::biome(x, z) != Biome::Forest || hash % 5 == 0 {
                continue;
            }
            let y = land::height(x, z);
            let scale = 0.8 + (hash % 50) as f32 * 0.012;
            commands.spawn((
                TerrainTree {
                    x,
                    z,
                    radius: 2.5 * scale,
                    site: row as u32 * 88 + column as u32 + 1,
                    crown: false,
                    original: Transform::from_xyz(x, y + 1.8 * scale, z)
                        .with_scale(Vec3::splat(scale)),
                },
                Mesh3d(trunk.clone()),
                MeshMaterial3d(bark.clone()),
                Transform::from_xyz(x, y + 1.8 * scale, z).with_scale(Vec3::splat(scale)),
            ));
            let evergreen = hash % 3 == 0;
            commands.spawn((
                TerrainTree {
                    x,
                    z,
                    radius: 2.5 * scale,
                    site: row as u32 * 88 + column as u32 + 1,
                    crown: true,
                    original: Transform::from_xyz(
                        x,
                        y + if evergreen { 4.7 } else { 4.2 } * scale,
                        z,
                    )
                    .with_scale(Vec3::splat(scale)),
                },
                Mesh3d(if evergreen {
                    pine.clone()
                } else {
                    crown.clone()
                }),
                MeshMaterial3d(foliage[(hash % 3) as usize].clone()),
                Transform::from_xyz(x, y + if evergreen { 4.7 } else { 4.2 } * scale, z)
                    .with_scale(Vec3::splat(scale)),
            ));
        }
    }
    let rock = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.57, 0.58, 0.53),
        perceptual_roughness: 0.95,
        ..default()
    });
    for (site, &(x, z)) in land::STONE.iter().enumerate() {
        for index in 0..(13. * (land::STONE_RADII[site] / 16.).powi(2)).round() as usize {
            let angle = index as f32 * 2.4;
            let radius = (index % 5) as f32 * 2.3 * land::STONE_RADII[site] / 16.;
            let px = x + angle.cos() * radius;
            let pz = z + angle.sin() * radius;
            commands.spawn((
                Mesh3d(rock.clone()),
                MeshMaterial3d(stone.clone()),
                Transform::from_xyz(px, land::height(px, pz) + 0.8, pz)
                    .with_scale(Vec3::new(2.7, 1.3 + (index % 3) as f32 * 0.5, 2.0))
                    .with_rotation(Quat::from_rotation_y(angle)),
                ecology::DepositVisual {
                    site: 10000 + site as u32,
                    original: Transform::from_xyz(px, land::height(px, pz) + 0.8, pz)
                        .with_scale(Vec3::new(2.7, 1.3 + (index % 3) as f32 * 0.5, 2.))
                        .with_rotation(Quat::from_rotation_y(angle)),
                },
            ));
        }
    }
    let ore = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.27, 0.29),
        metallic: 0.7,
        perceptual_roughness: 0.55,
        ..default()
    });
    let seam = materials.add(StandardMaterial {
        base_color: Color::srgb(0.64, 0.30, 0.13),
        metallic: 0.35,
        ..default()
    });
    for (site, &(x, z)) in land::IRON.iter().enumerate() {
        for index in 0..(11. * (land::IRON_RADII[site] / 13.).powi(2)).round() as usize {
            let angle = index as f32 * 2.4;
            let radius = (index % 4) as f32 * 2.3 * land::IRON_RADII[site] / 13.;
            let px = x + angle.cos() * radius;
            let pz = z + angle.sin() * radius;
            let y = land::height(px, pz);
            commands.spawn((
                Mesh3d(rock.clone()),
                MeshMaterial3d(ore.clone()),
                Transform::from_xyz(px, y + 0.7, pz)
                    .with_scale(Vec3::new(2.2, 1.4 + (index % 3) as f32 * 0.5, 1.6))
                    .with_rotation(Quat::from_rotation_y(angle)),
                ecology::DepositVisual {
                    site: 20000 + site as u32,
                    original: Transform::from_xyz(px, y + 0.7, pz)
                        .with_scale(Vec3::new(2.2, 1.4 + (index % 3) as f32 * 0.5, 1.6))
                        .with_rotation(Quat::from_rotation_y(angle)),
                },
            ));
            commands.spawn((
                Mesh3d(rock.clone()),
                MeshMaterial3d(seam.clone()),
                Transform::from_xyz(px + 0.4, y + 1.6, pz + 0.3)
                    .with_scale(Vec3::new(1.0, 0.3, 0.3))
                    .with_rotation(Quat::from_rotation_y(angle)),
                ecology::DepositVisual {
                    site: 20000 + site as u32,
                    original: Transform::from_xyz(px + 0.4, y + 1.6, pz + 0.3)
                        .with_scale(Vec3::new(1., 0.3, 0.3))
                        .with_rotation(Quat::from_rotation_y(angle)),
                },
            ));
        }
    }
}

pub(crate) fn sync_excavations(
    m: Res<Multiplayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunks: Query<(&TerrainChunk, &Mesh3d)>,
    mut previous: Local<Vec<(u64, u32, u32)>>,
) {
    let state = m
        .snapshot
        .moats
        .iter()
        .map(|m| (m.id, m.x.to_bits(), m.z.to_bits()))
        .collect::<Vec<_>>();
    if *previous == state {
        return;
    }
    *previous = state;
    for (tile, handle) in &chunks {
        let Some(mut mesh) = meshes.get_mut(&handle.0) else {
            continue;
        };
        let mut indices = Vec::new();
        for z in 0..40 {
            for x in 0..40 {
                let wx = -400. + (tile.x * 40 + x) as f32 * land::GRID + land::GRID * 0.5;
                let wz = -400. + (tile.z * 40 + z) as f32 * land::GRID + land::GRID * 0.5;
                if m.snapshot
                    .moats
                    .iter()
                    .any(|m| (m.x - wx).abs() < 2. && (m.z - wz).abs() < 2.)
                {
                    continue;
                }
                let a = (z * 41 + x) as u32;
                let b = a + 1;
                let c = a + 41;
                indices.extend([a, c, b, b, c, c + 1]);
            }
        }
        mesh.insert_indices(Indices::U32(indices));
    }
}
