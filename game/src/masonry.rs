//! Continue the actual tower material and facade UV scale over connector geometry.
use crate::*;
use bevy::mesh::VertexAttributeValues;
#[derive(Component)]
pub(crate) struct JointTexture {
    pub a: u64,
    pub b: u64,
    pub painted: bool,
}
#[derive(Clone, Copy)]
struct Fit {
    h: Vec2,
    y: Vec2,
    offset: Vec2,
    score: f32,
}
pub(crate) fn update(
    net: Res<Multiplayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    parts: Query<(&tower_mesh::TowerPart, &GlobalTransform)>,
    mut joints: Query<(
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
        &mut JointTexture,
    )>,
) {
    for (mut mesh, mut material, transform, mut joint) in &mut joints {
        if joint.painted {
            continue;
        }
        let towers = net
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.id == joint.a || b.id == joint.b)
            .filter(|b| {
                stronghold_map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_tower())
            })
            .map(|b| b.id)
            .collect::<Vec<_>>();
        if !towers.is_empty()
            && !parts
                .iter()
                .any(|(part, _)| towers.contains(&part.building))
        {
            continue;
        }

        let mut best = None;
        for (part, world) in &parts {
            if part.building != joint.a && part.building != joint.b {
                continue;
            }
            let Some(source) = meshes.get(&part.original) else {
                continue;
            };
            let Some(mat) = materials.get(&part.material) else {
                continue;
            };
            let Some(VertexAttributeValues::Float32x3(pos)) =
                source.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };
            let height = pos.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max)
                - pos.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let tower = net
                .snapshot
                .buildings
                .iter()
                .find(|b| b.id == part.building)
                .and_then(|b| stronghold_map::BuildingKind::from_name(&b.kind))
                .is_some_and(|k| k.is_tower());
            let score = height
                * pos.len() as f32
                * if mat.base_color_texture.is_some() {
                    100.
                } else {
                    1.
                }
                * if tower { 10. } else { 1. };
            if best.as_ref().is_none_or(
                |(old, _, _, _): &(f32, Handle<Mesh>, Handle<StandardMaterial>, Mat4)| score > *old,
            ) {
                best = Some((
                    score,
                    part.original.clone(),
                    part.material.clone(),
                    world.to_matrix(),
                ));
            }
        }
        let Some((_, source_handle, source_material, world)) = best else {
            continue;
        };
        let source = meshes.get(&source_handle).unwrap();
        let mut fits: [Option<Fit>; 4] = [None; 4];
        if let (
            Some(VertexAttributeValues::Float32x3(p)),
            Some(VertexAttributeValues::Float32x2(uv)),
        ) = (
            source.attribute(Mesh::ATTRIBUTE_POSITION),
            source.attribute(Mesh::ATTRIBUTE_UV_0),
        ) {
            let indices = source
                .indices()
                .map(|v| v.iter().collect::<Vec<_>>())
                .unwrap_or_else(|| (0..p.len()).collect());
            for tri in indices.chunks_exact(3) {
                let w = tri
                    .iter()
                    .map(|&i| world.transform_point3(Vec3::from(p[i])))
                    .collect::<Vec<_>>();
                let cross = (w[1] - w[0]).cross(w[2] - w[0]);
                let normal = cross.normalize_or_zero();
                if normal.y.abs() > 0.55 {
                    continue;
                }
                let axis = if normal.x.abs() > normal.z.abs() {
                    2
                } else {
                    0
                };
                let side = if axis == 2 {
                    if normal.x > 0. {
                        0
                    } else {
                        1
                    }
                } else if normal.z > 0. {
                    2
                } else {
                    3
                };
                let a = Vec2::new(w[1][axis] - w[0][axis], w[1].y - w[0].y);
                let b = Vec2::new(w[2][axis] - w[0][axis], w[2].y - w[0].y);
                let det = a.x * b.y - a.y * b.x;
                if det.abs() < 0.00001 {
                    continue;
                }
                let u0 = Vec2::from(uv[tri[0]]);
                let u1 = Vec2::from(uv[tri[1]]) - u0;
                let u2 = Vec2::from(uv[tri[2]]) - u0;
                let h = (u1 * b.y - u2 * a.y) / det;
                let y = (u2 * a.x - u1 * b.x) / det;
                let score = cross.length();
                let fit = Fit {
                    h,
                    y,
                    offset: u0 - h * w[0][axis] - y * w[0].y,
                    score,
                };
                if fits[side].is_none_or(|f| score > f.score) {
                    fits[side] = Some(fit);
                }
            }
        }
        let mut output = Mesh::from(Cuboid::new(1., 1., 1.));
        let Some(VertexAttributeValues::Float32x3(pos)) =
            output.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            output.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            continue;
        };
        let matrix = transform.to_matrix();
        let fallback = fits.iter().flatten().next().copied();
        let uv = pos
            .iter()
            .zip(normals)
            .map(|(p, n)| {
                let w = matrix.transform_point3(Vec3::from(*p));
                let side = if n[0] > 0.5 {
                    0
                } else if n[0] < -0.5 {
                    1
                } else if n[2] > 0.5 {
                    2
                } else {
                    3
                };
                let h = if side < 2 { w.z } else { w.x };
                fits[side]
                    .or(fallback)
                    .map_or(Vec2::new(w.x * 0.2, w.y * 0.2), |f| {
                        if n[1].abs() > 0.5 {
                            Vec2::new(
                                w.x * f.h.length().max(0.1) + f.offset.x,
                                w.z * f.h.length().max(0.1) + f.offset.y,
                            )
                        } else {
                            f.h * h + f.y * w.y + f.offset
                        }
                    })
                    .to_array()
            })
            .collect::<Vec<_>>();
        output.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let _ = output.generate_tangents();
        mesh.0 = meshes.add(output);
        material.0 = source_material;
        joint.painted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connector_uses_the_actual_facade_material_and_world_uv_scale() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Multiplayer>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, update);
        let source = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(4., 10., 4.));
        let mat = app
            .world_mut()
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.6, 0.4),
                base_color_texture: Some(Handle::default()),
                ..default()
            });
        app.world_mut().spawn((
            tower_mesh::TowerPart {
                building: 1,
                original: source.clone(),
                material: mat.clone(),
                revision: String::new(),
                variant: None,
            },
            GlobalTransform::default(),
        ));
        let original = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::new(1., 1., 1.));
        let e = app
            .world_mut()
            .spawn((
                Mesh3d(original),
                MeshMaterial3d::<StandardMaterial>(Handle::default()),
                GlobalTransform::from(
                    Transform::from_xyz(4., 5., 0.).with_scale(Vec3::new(4., 10., 4.)),
                ),
                JointTexture {
                    a: 1,
                    b: 2,
                    painted: false,
                },
            ))
            .id();
        app.update();
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(e)
                .unwrap()
                .0,
            mat
        );
        assert!(app.world().get::<JointTexture>(e).unwrap().painted);
        let mesh = app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&app.world().get::<Mesh3d>(e).unwrap().0)
            .unwrap();
        let VertexAttributeValues::Float32x2(uv) = mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap()
        else {
            panic!()
        };
        let max = uv
            .iter()
            .flat_map(|u| u.iter())
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max > 1.4,
            "four metre connector repeats the tower stone scale rather than stretching one texture"
        );
        assert_eq!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&source)
                .unwrap()
                .count_vertices(),
            24,
            "source is never modified"
        );
    }
}
