use crate::*;
use stronghold_map::ranged;

#[derive(Component)]
pub(crate) struct ArrowVisual {
    id: u64,
    age: f32,
}
pub(crate) fn sync_arrows(
    mut commands: Commands,
    time: Res<Time>,
    multiplayer: Res<Multiplayer>,
    assets: Res<combat_units::CombatAssets>,
    industry: Res<crate::industry::IndustryAssets>,
    mut visuals: Query<(Entity, &mut ArrowVisual, &mut Transform)>,
) {
    let Some(scene) = &assets.arrow_scene else {
        return;
    };
    let mut existing = std::collections::HashSet::new();
    for (entity, mut visual, mut transform) in &mut visuals {
        let Some(arrow) = multiplayer
            .snapshot
            .arrows
            .iter()
            .find(|a| a.id == visual.id)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        existing.insert(arrow.id);
        visual.age = (visual.age + time.delta_secs()).max(arrow.age);
        *transform = arrow_transform(arrow, visual.age, &multiplayer.snapshot);
    }
    for arrow in multiplayer
        .snapshot
        .arrows
        .iter()
        .filter(|a| !existing.contains(&a.id))
    {
        commands
            .spawn((
                ArrowVisual {
                    id: arrow.id,
                    age: arrow.age,
                },
                arrow_transform(arrow, arrow.age, &multiplayer.snapshot),
                Visibility::default(),
            ))
            .with_children(|parent| {
                if matches!(arrow.kind.as_str(), "onager" | "stone_ballista") {
                    parent.spawn((
                        Mesh3d(industry.stone_mesh.clone()),
                        MeshMaterial3d(industry.stone_material.clone()),
                        Transform::from_scale(Vec3::splat(0.22)),
                    ));
                } else {
                    parent.spawn((
                        WorldAssetRoot(scene.clone()),
                        Transform::from_xyz(-2.416 * 0.006, -67.25 * 0.006, 0.)
                            .with_scale(Vec3::splat(0.006)),
                    ));
                }
            });
    }
}
fn arrow_transform(arrow: &network::Arrow, age: f32, snapshot: &network::Snapshot) -> Transform {
    let start = (arrow.start_x, arrow.start_y, arrow.start_z);
    let end = if arrow.building {
        (arrow.end_x, arrow.end_y, arrow.end_z)
    } else {
        snapshot
            .units
            .iter()
            .find(|u| u.id == arrow.target_id)
            .map(|u| (u.x, u.y + 0.15, u.z))
            .unwrap_or((arrow.end_x, arrow.end_y, arrow.end_z))
    };
    let progress = (age / arrow.flight_seconds).clamp(0., 1.);
    let point = ranged::projectile_point(start, end, progress, &arrow.kind);
    let before = ranged::projectile_point(start, end, (progress - 0.01).max(0.), &arrow.kind);
    let after = ranged::projectile_point(start, end, (progress + 0.01).min(1.), &arrow.kind);
    let direction =
        Vec3::new(after.0 - before.0, after.1 - before.1, after.2 - before.2).normalize_or_zero();
    Transform::from_xyz(point.0, point.1, point.2).with_rotation(
        if direction.length_squared() > 0.01 {
            Quat::from_rotation_arc(Vec3::Y, direction)
        } else {
            Quat::IDENTITY
        },
    )
}
pub(crate) fn range_preview(
    selected: Res<SelectedUnits>,
    units: Query<(Entity, &Unit, &Transform)>,
    mut gizmos: Gizmos,
) {
    for (entity, u, transform) in &units {
        if !u.owned
            || !stronghold_map::units::UnitKind::from_name(&u.kind).is_some_and(|k| k.is_ranged())
            || !selected.0.contains(&entity)
        {
            continue;
        }
        let origin = transform.translation;
        let range = stronghold_map::units::UnitKind::from_name(&u.kind)
            .unwrap()
            .ranged_range(origin.y);
        gizmos.linestrip(
            (0..=96).map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 96.;
                let x = origin.x + range * angle.cos();
                let z = origin.z + range * angle.sin();
                Vec3::new(x, buildings::terrain_height(x, z) + 0.08, z)
            }),
            Color::srgba(0.3, 0.85, 1., 0.65),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrow_visual_follows_the_arc_and_points_along_its_flight() {
        let arrow = network::Arrow {
            kind: "archer".into(),
            id: 1,
            target_id: 2,
            building: false,
            start_x: 0.,
            start_y: 1.3,
            start_z: 0.,
            end_x: 14.,
            end_y: 0.9,
            end_z: 0.,
            age: 0.,
            flight_seconds: 1.,
        };
        let mut snapshot = network::Snapshot::default();
        for age in [0., 0.25, 0.5, 0.75, 1.] {
            let transform = arrow_transform(&arrow, age, &snapshot);
            let p = ranged::arrow_point((0., 1.3, 0.), (14., 0.9, 0.), age);
            assert!((transform.translation - Vec3::new(p.0, p.1, p.2)).length() < 0.001);
            assert!((transform.rotation * Vec3::Y).x > 0.5);
        }
        snapshot.units.push(serde_json::from_value(serde_json::json!({"id":"2","number":1,"color":1,"health":1.,"owned":false,"x":12.,"y":0.75,"z":3.})).unwrap());
        assert_eq!(
            arrow_transform(&arrow, 1., &snapshot).translation,
            Vec3::new(12., 0.9, 3.)
        );
    }
}
