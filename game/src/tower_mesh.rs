//! Instance-local mesh clipping: remove facing battlements, never whole tower models.
use crate::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, VertexAttributeValues},
    world_serialization::WorldInstanceReady,
};
use stronghold_map::fortifications::ParapetBlock;
#[derive(Component)]
pub(crate) struct TowerRoot(pub u64);
#[derive(Component)]
pub(crate) struct TowerPart {
    pub(crate) building: u64,
    pub(crate) original: Handle<Mesh>,
    pub(crate) material: Handle<StandardMaterial>,
    pub(crate) revision: String,
    pub(crate) variant: Option<Handle<Mesh>>,
}
pub(crate) fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<&TowerRoot>,
    children: Query<&Children>,
    meshes: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Ok(root) = roots.get(event.entity) else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if let Ok((mesh, material)) = meshes.get(entity) {
            commands.entity(entity).insert(TowerPart {
                building: root.0,
                original: mesh.0.clone(),
                material: material.0.clone(),
                revision: String::new(),
                variant: None,
            });
        }
    }
}
pub(crate) fn update(
    net: Res<Multiplayer>,
    mut assets: ResMut<Assets<Mesh>>,
    mut parts: Query<(&mut Mesh3d, &GlobalTransform, &mut TowerPart)>,
    mut cache: Local<(
        String,
        std::collections::HashMap<u64, (String, Vec<ParapetBlock>)>,
    )>,
) {
    let revision = format!(
        "{:?}:{:?}",
        net.snapshot.game_id,
        net.snapshot
            .buildings
            .iter()
            .map(|b| (
                b.id,
                &b.kind,
                b.x.to_bits(),
                b.y.to_bits(),
                b.z.to_bits(),
                b.rotation
            ))
            .collect::<Vec<_>>()
    );
    if cache.0 != revision {
        let pieces = crate::fortifications::pieces(&net.snapshot);
        let connections = stronghold_map::fortifications::joins(&pieces);
        cache.0 = revision;
        cache.1.clear();
        for piece in pieces.iter().filter(|p| p.kind.is_tower()) {
            let cuts =
                stronghold_map::fortifications::tower_openings_from(*piece, &pieces, &connections);
            let key = format!(
                "{:?}",
                cuts.iter()
                    .map(|c| (
                        c.shape.x.to_bits(),
                        c.shape.z.to_bits(),
                        c.shape.half_x.to_bits(),
                        c.shape.half_z.to_bits(),
                        c.bottom.to_bits(),
                        c.top.to_bits()
                    ))
                    .collect::<Vec<_>>()
            );
            cache.1.insert(piece.id, (key, cuts));
        }
    }
    for (mut mesh, transform, mut part) in &mut parts {
        let Some((key, cuts)) = cache.1.get(&part.building) else {
            continue;
        };
        if *key == part.revision {
            continue;
        }
        let Some(source) = assets.get(&part.original) else {
            continue;
        };
        let result = if cuts.is_empty() {
            None
        } else {
            Some(trim_mesh(source, transform.to_matrix(), cuts))
        };
        if let Some(previous) = part.variant.take() {
            assets.remove(previous.id());
        }
        if let Some(result) = result {
            let handle = assets.add(result);
            mesh.0 = handle.clone();
            part.variant = Some(handle);
        } else {
            mesh.0 = part.original.clone();
        }
        part.revision = key.clone();
    }
}

#[derive(Clone)]
struct Vertex {
    p: Vec3,
    data: Vec<Vec<f32>>,
}
impl Vertex {
    fn between(&self, b: &Self, t: f32) -> Self {
        Self {
            p: self.p.lerp(b.p, t),
            data: self
                .data
                .iter()
                .zip(&b.data)
                .map(|(a, b)| a.iter().zip(b).map(|(a, b)| a + (b - a) * t).collect())
                .collect(),
        }
    }
}
fn split(poly: &[Vertex], axis: usize, bound: f32, sign: f32) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for i in 0..poly.len() {
        let a = &poly[i];
        let b = &poly[(i + 1) % poly.len()];
        let da = (a.p[axis] - bound) * sign;
        let db = (b.p[axis] - bound) * sign;
        let a_in = da <= 0.;
        let b_in = db <= 0.;
        if a_in {
            inside.push(a.clone());
        } else {
            outside.push(a.clone());
        }
        if a_in != b_in {
            let v = a.between(b, da / (da - db));
            inside.push(v.clone());
            outside.push(v);
        }
    }
    (inside, outside)
}
fn subtract(poly: Vec<Vertex>, cut: &ParapetBlock) -> Vec<Vec<Vertex>> {
    let mut remaining = poly;
    let mut result = Vec::new();
    for (axis, bound, sign) in [
        (0, cut.shape.x - cut.shape.half_x, -1.),
        (0, cut.shape.x + cut.shape.half_x, 1.),
        (1, cut.bottom, -1.),
        (1, cut.top, 1.),
        (2, cut.shape.z - cut.shape.half_z, -1.),
        (2, cut.shape.z + cut.shape.half_z, 1.),
    ] {
        if remaining.len() < 3 {
            break;
        }
        let (inside, outside) = split(&remaining, axis, bound, sign);
        if outside.len() >= 3 {
            result.push(outside);
        }
        remaining = inside;
    }
    result
}
fn floats(value: &VertexAttributeValues, index: usize) -> Vec<f32> {
    match value {
        VertexAttributeValues::Float32(v) => vec![v[index]],
        VertexAttributeValues::Float32x2(v) => v[index].to_vec(),
        VertexAttributeValues::Float32x3(v) => v[index].to_vec(),
        VertexAttributeValues::Float32x4(v) => v[index].to_vec(),
        _ => Vec::new(),
    }
}
fn trim_mesh(source: &Mesh, world: Mat4, cuts: &[ParapetBlock]) -> Mesh {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        source.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return source.clone();
    };
    let attributes = source
        .attributes()
        .filter(|(a, v)| {
            a.id != Mesh::ATTRIBUTE_POSITION.id
                && matches!(
                    v,
                    VertexAttributeValues::Float32(_)
                        | VertexAttributeValues::Float32x2(_)
                        | VertexAttributeValues::Float32x3(_)
                        | VertexAttributeValues::Float32x4(_)
                )
        })
        .map(|(a, v)| (*a, v))
        .collect::<Vec<_>>();
    let indices = source
        .indices()
        .map(|v| v.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| (0..positions.len()).collect());
    let inverse = world.inverse();
    let mut vertices = Vec::<Vertex>::new();
    for triangle in indices.chunks_exact(3) {
        let mut polys = vec![triangle
            .iter()
            .map(|&i| Vertex {
                p: world.transform_point3(Vec3::from(positions[i])),
                data: attributes.iter().map(|(_, v)| floats(v, i)).collect(),
            })
            .collect()];
        for cut in cuts {
            polys = polys
                .into_iter()
                .flat_map(|poly| subtract(poly, cut))
                .collect();
        }
        for poly in polys {
            for i in 1..poly.len() - 1 {
                vertices.extend([poly[0].clone(), poly[i].clone(), poly[i + 1].clone()]);
            }
        }
    }
    let mut mesh = Mesh::new(source.primitive_topology(), RenderAssetUsages::default());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices
            .iter()
            .map(|v| inverse.transform_point3(v.p).to_array())
            .collect::<Vec<_>>(),
    );
    for (index, (attribute, values)) in attributes.iter().enumerate() {
        let data = match values {
            VertexAttributeValues::Float32(_) => {
                VertexAttributeValues::Float32(vertices.iter().map(|v| v.data[index][0]).collect())
            }
            VertexAttributeValues::Float32x2(_) => VertexAttributeValues::Float32x2(
                vertices
                    .iter()
                    .map(|v| [v.data[index][0], v.data[index][1]])
                    .collect(),
            ),
            VertexAttributeValues::Float32x3(_) => VertexAttributeValues::Float32x3(
                vertices
                    .iter()
                    .map(|v| [v.data[index][0], v.data[index][1], v.data[index][2]])
                    .collect(),
            ),
            VertexAttributeValues::Float32x4(_) => VertexAttributeValues::Float32x4(
                vertices
                    .iter()
                    .map(|v| {
                        [
                            v.data[index][0],
                            v.data[index][1],
                            v.data[index][2],
                            v.data[index][3],
                        ]
                    })
                    .collect(),
            ),
            _ => continue,
        };
        mesh.insert_attribute(*attribute, data);
    }
    mesh.insert_indices(Indices::U32((0..vertices.len() as u32).collect()));
    mesh
}
#[cfg(test)]
mod tests {
    use super::*;
    use stronghold_map::Obstacle;
    #[test]
    fn removing_the_neighbour_restores_the_original_without_touching_shared_meshes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Multiplayer>()
            .add_systems(Update, update);
        let kind = stronghold_map::BuildingKind::TowerRoundShort;
        let make = |id, x| network::Building {
            id,
            kind: kind.name().into(),
            rotation: 0,
            color: 0,
            owned: true,
            health: 1.,
            x,
            y: 0.,
            z: 0.,
        };
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings = vec![
            make(1, 0.),
            make(2, kind.obstacle(0., 0.).half_x * 2. + 0.0002),
        ];
        let original = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(4., 4., 4.));
        let entity = app
            .world_mut()
            .spawn((
                Mesh3d(original.clone()),
                GlobalTransform::from_translation(Vec3::new(4., 12., 0.)),
                TowerPart {
                    building: 1,
                    original: original.clone(),
                    material: default(),
                    revision: String::new(),
                    variant: None,
                },
            ))
            .id();
        let untouched = app.world_mut().spawn(Mesh3d(original.clone())).id();
        app.update();
        let variant = app.world().get::<Mesh3d>(entity).unwrap().0.clone();
        assert_ne!(variant, original);
        assert_eq!(app.world().get::<Mesh3d>(untouched).unwrap().0, original);
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings
            .retain(|b| b.id == 1);
        app.update();
        assert_eq!(app.world().get::<Mesh3d>(entity).unwrap().0, original);
        assert!(app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&variant)
            .is_none());
        assert!(app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&original)
            .is_some());
    }
    #[test]
    fn clipping_opens_only_the_facing_wall_and_preserves_floor_and_exterior() {
        let mesh = Mesh::from(Cuboid::new(4., 4., 4.));
        let cut = ParapetBlock {
            shape: Obstacle {
                x: 2.,
                z: 0.,
                half_x: 0.75,
                half_z: 0.8,
            },
            bottom: 0.,
            top: 3.,
        };
        let result = trim_mesh(&mesh, Mat4::IDENTITY, &[cut]);
        let Some(VertexAttributeValues::Float32x3(p)) = result.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        assert!(
            p.iter().any(|p| p[0] < -1.99 && p[1] > 1.9),
            "exterior crenels remain"
        );
        assert!(p.iter().any(|p| p[1] < -1.99), "floor remains");
        for t in p.chunks_exact(3) {
            let center = (Vec3::from(t[0]) + Vec3::from(t[1]) + Vec3::from(t[2])) / 3.;
            assert!(
                !(center.x > 1.251 && center.y > 0.001 && center.z.abs() < 0.799),
                "no remaining facing geometry"
            );
        }
        assert!(result.count_vertices() > 0);
        assert_eq!(mesh.count_vertices(), 24, "shared source unchanged");
    }
}
