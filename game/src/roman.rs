//! Roman banners, workplace equipment and manual world saves.
use crate::*;
#[derive(Resource)]
pub(crate) struct RomanAssets {
    cube: Handle<Mesh>,
    wood: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
}
#[derive(Component)]
pub(crate) struct RomanFlag {
    keep: u64,
    side: f32,
    strip: i32,
}
#[derive(Component)]
pub(crate) struct Equipment;
pub(crate) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RomanAssets {
        cube: meshes.add(Cuboid::new(1., 1., 1.)),
        wood: mats.add(Color::srgb(0.3, 0.16, 0.05)),
        iron: mats.add(Color::srgb(0.42, 0.45, 0.47)),
        red: mats.add(Color::srgb(0.65, 0.025, 0.035)),
        gold: mats.add(Color::srgb(0.88, 0.65, 0.15)),
    });
}
pub(crate) fn visuals(
    mut commands: Commands,
    net: Res<Multiplayer>,
    assets: Res<RomanAssets>,
    time: Res<Time>,
    mut flags: Query<(Entity, &RomanFlag, &mut Transform)>,
    workers: Query<(Entity, &Unit), Without<Equipment>>,
) {
    for (e, f, mut t) in &mut flags {
        let Some(b) = net.snapshot.buildings.iter().find(|b| b.id == f.keep) else {
            commands.entity(e).despawn();
            continue;
        };
        *t = flag_transform(b, f.side, f.strip, time.elapsed_secs());
    }
    for b in net.snapshot.buildings.iter().filter(|b| {
        b.kind == "keep"
            || stronghold_map::BuildingKind::from_name(&b.kind).is_some_and(|k| k.is_anchor())
    }) {
        if flags.iter().any(|(_, f, _)| f.keep == b.id) {
            continue;
        }
        for &side in if b.kind == "keep" {
            &[-1., 1.][..]
        } else {
            &[0.][..]
        } {
            for strip in -1..24 {
                commands.spawn((
                    RomanFlag {
                        keep: b.id,
                        side,
                        strip,
                    },
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(if strip < 0 || strip == 23 {
                        assets.gold.clone()
                    } else {
                        assets.red.clone()
                    }),
                    flag_transform(b, side, strip, time.elapsed_secs()),
                ));
            }
        }
    }
    for (e, u) in &workers {
        let job = net
            .snapshot
            .workers
            .iter()
            .find(|w| w.unit_id == u.id)
            .and_then(|w| {
                net.snapshot
                    .buildings
                    .iter()
                    .find(|b| b.id == w.source_building)
            })
            .map(|b| b.kind.as_str());
        let Some(job) = job else { continue };
        if !["hunters_post", "quarry", "iron_mine", "saw_pit"].contains(&job) {
            continue;
        }
        commands.entity(e).insert(Equipment).with_children(|c| {
            let mut part = |pos: Vec3, scale: Vec3, rot: Quat, metal: bool| {
                c.spawn((
                    Mesh3d(assets.cube.clone()),
                    MeshMaterial3d(if metal {
                        assets.iron.clone()
                    } else {
                        assets.wood.clone()
                    }),
                    Transform::from_translation(pos)
                        .with_scale(scale)
                        .with_rotation(rot),
                ));
            };
            if job == "hunters_post" {
                for i in 0..12 {
                    let a = -1.3 + i as f32 * 2.6 / 11.;
                    part(
                        Vec3::new(0.48 + 0.22 * a.cos(), 0.65 + a.sin() * 0.64, 0.08),
                        Vec3::new(0.045, 0.15, 0.045),
                        Quat::from_rotation_z(-a),
                        false,
                    );
                }
                part(
                    Vec3::new(0.54, 0.65, 0.08),
                    Vec3::new(0.012, 1.24, 0.012),
                    Quat::IDENTITY,
                    false,
                );
                part(
                    Vec3::new(0.58, 0.65, 0.08),
                    Vec3::new(0.035, 0.035, 0.9),
                    Quat::IDENTITY,
                    false,
                );
                part(
                    Vec3::new(0.58, 0.65, -0.4),
                    Vec3::splat(0.09),
                    Quat::IDENTITY,
                    true,
                );
            } else {
                part(
                    Vec3::new(0.5, 0.6, 0.),
                    Vec3::new(0.055, 1.2, 0.055),
                    Quat::from_rotation_z(-0.3),
                    false,
                );
                part(
                    Vec3::new(0.32, 1.14, 0.),
                    Vec3::new(0.62, 0.09, 0.09),
                    Quat::from_rotation_z(0.1),
                    true,
                );
            }
        });
    }
}
fn flag_transform(b: &network::Building, side: f32, strip: i32, t: f32) -> Transform {
    let a = b.rotation as f32 * std::f32::consts::FRAC_PI_2;
    let rot = Quat::from_rotation_y(a);
    if b.kind != "keep" {
        let k = stronghold_map::BuildingKind::from_name(&b.kind).unwrap_or_default();
        let height = k
            .height()
            .max(k.roof_profile().map_or(0., |p| p.height + 1.2));
        let offset = k.roof_profile().map_or(Vec3::ZERO, |p| {
            Vec3::new(-p.half_x * 0.5, 0., -p.half_z * 0.5)
        });
        let (pos, scale, turn) = if strip < 0 {
            (
                offset + Vec3::new(0., height + 1.75, 0.),
                Vec3::new(0.07, 3.5, 0.07),
                0.,
            )
        } else {
            let x = (strip as f32 + 0.5) * 0.07;
            let phase = t * 3.4 - x * 2.6;
            (
                offset + Vec3::new(x, height + 2.8, x / 1.7 * phase.sin() * 0.25),
                Vec3::new(0.075, 1.1, 0.03),
                -x / 1.7 * phase.cos() * 0.6,
            )
        };
        return Transform::from_translation(Vec3::new(b.x, b.y, b.z) + rot * pos)
            .with_rotation(rot * Quat::from_rotation_y(turn))
            .with_scale(scale);
    }
    let (p, s, r) = if strip < 0 {
        (
            Vec3::new(side * 6.1, 12.1, -4.8),
            Vec3::new(0.08, 5.8, 0.08),
            Quat::IDENTITY,
        )
    } else {
        let x = (strip as f32 + 0.5) * 0.105;
        let phase = t * 3.4 - x * 2.6 + side;
        let z = x / 2.5 * phase.sin() * 0.32;
        (
            Vec3::new(side * 6.1 + x, 14.8, -4.8 + z),
            Vec3::new(0.11, 1.5, 0.035),
            Quat::from_rotation_y(-x / 2.5 * phase.cos() * 0.7),
        )
    };
    Transform::from_translation(Vec3::new(b.x, b.y, b.z) + rot * p)
        .with_rotation(rot * r)
        .with_scale(s)
}
pub(crate) fn save_ui(
    mut contexts: EguiContexts,
    mut net: ResMut<Multiplayer>,
    mut open: Local<bool>,
    mut name: Local<String>,
) -> Result {
    if net.snapshot.game_id.is_none() {
        *open = false;
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    egui::Area::new(egui::Id::new("save_button"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(12., 65.))
        .show(ctx, |ui| {
            if ui.button("💾 Spiel speichern").clicked() {
                *open = true;
                if name.is_empty() {
                    *name = "Mein Spiel".into();
                }
            }
        });
    if *open {
        egui::Window::new("Spiel speichern und laden")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Name des Spielstands");
                ui.add(egui::TextEdit::singleline(&mut *name).char_limit(60));
                if ui.add_enabled(!name.trim().is_empty(), egui::Button::new("Speichern / überschreiben")).clicked() {
                    net.send(Action::SaveGame { title: name.clone() });
                }
                if let Some(error) = &net.snapshot.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                ui.separator();
                ui.small("Gespeicherte Spielstände · Laden öffnet eine Kopie");
                for save in net.snapshot.saves.clone() {
                    ui.horizontal(|ui| {
                        ui.label(&save.title);
                        let seconds=save.saved_at_micros.div_euclid(1_000_000);
                        ui.small(format!("✓ Gespeichert {:02}:{:02} UTC",seconds.rem_euclid(86400)/3600,seconds.rem_euclid(3600)/60));
                        if ui.button("Laden").clicked() {
                            net.send(Action::LoadGame { save_id: save.id });
                        }
                    });
                }
                ui.small("Gleicher Name überschreibt den vorhandenen Stand.");
                ui.small("Auf diesem Spielserver gespeichert. Behalte denselben Browser und dessen Browserdaten, um später zu laden.");
            });
    }
    Ok(())
}

#[cfg(test)]
mod flag_tests {
    use super::*;
    #[test]
    fn waving_cloth_clears_the_entire_palace_roof() {
        for rotation in 0..4 {
            let b = network::Building {
                kind: "keep".into(),
                rotation,
                id: 1,
                color: 0,
                health: 1.,
                owned: true,
                x: 12.,
                y: 3.,
                z: 24.,
            };
            for side in [-1., 1.] {
                for strip in 0..24 {
                    for time in 0..100 {
                        let t = flag_transform(&b, side, strip, time as f32 * 0.1);
                        assert!(
                            t.translation.y - t.scale.y * 0.5 > b.y + 13.,
                            "cloth must stay above every roof stone"
                        );
                    }
                }
            }
        }
    }
}
