//! Horse breeding visuals, Roman manufacture and real intermediate inventories.
use crate::*;
use bevy::{gltf::Gltf, world_serialization::WorldInstanceReady};
#[derive(Resource, Default)]
pub(crate) struct IndustryAssets {
    loading: Option<Handle<Gltf>>,
    models: Vec<Model>,
    pub(crate) stone_mesh: Handle<Mesh>,
    pub(crate) stone_material: Handle<StandardMaterial>,
}
struct Model {
    scene: Handle<WorldAsset>,
    graph: Option<Handle<AnimationGraph>>,
    clips: Vec<AnimationNodeIndex>,
}
const PATHS: [&str; 11] = [
    "models/animals/horse/mesh.glb",
    "models/units/ballista/mesh.glb",
    "models/units/onager/mesh.glb",
    "models/industry/wheat_patch/mesh.glb",
    "models/units/stone_ballista/mesh.glb",
    "models/units/scorpion/mesh.glb",
    "models/units/ram/mesh.glb",
    "models/units/covered_ram/mesh.glb",
    "models/units/siege_tower/mesh.glb",
    "models/units/assault_ladder/mesh.glb",
    "models/units/pluteus/mesh.glb",
];
pub(crate) fn setup(
    mut assets: ResMut<IndustryAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    assets.stone_mesh = meshes.add(Sphere::new(1.).mesh().ico(2).unwrap());
    assets.stone_material = materials.add(Color::srgb(0.54, 0.50, 0.40));
}
pub(crate) fn load_assets(
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut assets: ResMut<IndustryAssets>,
) {
    if let Some(handle) = &assets.loading {
        let Some(gltf) = gltfs
            .get(handle)
            .filter(|_| server.is_loaded_with_dependencies(handle.id()))
        else {
            return;
        };
        let (graph, clips) = if gltf.animations.is_empty() {
            (None, Vec::new())
        } else {
            let (g, c) = AnimationGraph::from_clips(gltf.animations.iter().cloned());
            (Some(graphs.add(g)), c)
        };
        assets.models.push(Model {
            scene: gltf.scenes[0].clone(),
            graph,
            clips,
        });
        assets.loading = None;
    }
    if assets.models.len() < PATHS.len() && assets.loading.is_none() {
        let index = assets.models.len();
        assets.loading = Some(server.load(PATHS[index]));
    }
}
#[derive(Component)]
pub(crate) struct HorseVisual {
    id: u64,
    stable: u64,
    breeder: u8,
    visible_age: f32,
}
#[derive(Component)]
pub(crate) struct HorseRig;
#[derive(Component)]
pub(crate) struct SiegeVisual {
    owner: Entity,
}
#[derive(Component)]
pub(crate) struct FieldVisual {
    building: u64,
}
fn horse_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    assets: Res<IndustryAssets>,
) {
    let Some(model) = assets.models.first() else {
        return;
    };
    let Some(graph) = &model.graph else {
        return;
    };
    for entity in children.iter_descendants(event.entity) {
        if players.contains(entity) {
            let mut player = AnimationPlayer::default();
            player.play(model.clips[1]).repeat();
            commands
                .entity(entity)
                .insert((AnimationGraphHandle(graph.clone()), player));
        }
    }
}
pub(crate) fn visuals(
    mut commands: Commands,
    time: Res<Time>,
    net: Res<Multiplayer>,
    assets: Res<IndustryAssets>,
    units: Query<(Entity, &Unit)>,
    mut horses: Query<(Entity, &mut HorseVisual, &mut Transform)>,
    mut siege: Query<(Entity, &SiegeVisual, &mut Transform), Without<HorseVisual>>,
    fields: Query<(Entity, &FieldVisual)>,
) {
    let mut expected = Vec::new();
    for b in net.snapshot.buildings.iter().filter(|b| b.kind == "stable") {
        expected.push((0, b.id, 1, 90));
        expected.push((0, b.id, 2, 90));
    }
    for h in &net.snapshot.horses {
        expected.push((h.id, h.stable_id, 0, h.age));
    }
    let mut existing = std::collections::HashSet::new();
    for (entity, mut horse, mut t) in &mut horses {
        let key = (horse.id, horse.stable, horse.breeder);
        let Some(&(_, _, _, age)) = expected.iter().find(|&&(id, s, b, _)| (id, s, b) == key)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        let Some(stable) = net.snapshot.buildings.iter().find(|b| b.id == horse.stable) else {
            commands.entity(entity).despawn();
            continue;
        };
        existing.insert(key);
        horse.visible_age = if age as f32 - horse.visible_age > 3. {
            age as f32
        } else {
            (horse.visible_age + time.delta_secs()).min(age as f32)
        };
        let fraction = (horse.visible_age / 90.).clamp(0., 1.);
        let phase =
            time.elapsed_secs() * 0.12 + horse.id as f32 * 1.73 + horse.breeder as f32 * 3.4;
        let local = (3.0 + phase.cos() * 1.5, phase.sin() * 3.2);
        let p = stronghold_map::fortifications::world((stable.x, stable.z), stable.rotation, local);
        let a = stable.rotation as f32 * std::f32::consts::FRAC_PI_2;
        *t = Transform::from_xyz(p.0, stable.y + 0.15, p.1)
            .with_rotation(Quat::from_rotation_y(
                a + (-phase.sin() * 1.5).atan2(phase.cos() * 3.2),
            ))
            .with_scale(Vec3::splat(0.007653524 * (0.42 + 0.58 * fraction)));
    }
    if let Some(model) = assets.models.first() {
        for &(id, stable, breeder, age) in &expected {
            if !existing.contains(&(id, stable, breeder)) {
                commands
                    .spawn((
                        HorseVisual {
                            id,
                            stable,
                            breeder,
                            visible_age: age as f32,
                        },
                        HorseRig,
                        WorldAssetRoot(model.scene.clone()),
                        Transform::from_scale(Vec3::splat(0.0035)),
                    ))
                    .observe(horse_ready);
            }
        }
    }
    let mut shown = std::collections::HashSet::new();
    for (entity, v, mut t) in &mut siege {
        let Ok((_, u)) = units.get(v.owner) else {
            commands.entity(entity).despawn();
            continue;
        };
        if !stronghold_map::units::UnitKind::from_name(&u.kind).is_some_and(|k| k.is_siege()) {
            commands.entity(entity).despawn();
            continue;
        }
        shown.insert(v.owner);
        let yaw = Quat::from_rotation_y(u.facing.x.atan2(u.facing.y));
        t.rotation = yaw;
        t.translation = Vec3::new(0., -0.75, 0.);
        t.scale = Vec3::splat(0.01);
        if u.kind == "siege_tower" {
            if let Some(b) = net
                .snapshot
                .siege_access
                .iter()
                .find(|a| a.unit_id == u.id)
                .and_then(|a| {
                    net.snapshot
                        .buildings
                        .iter()
                        .find(|b| b.id == a.building_id)
                })
            {
                if let Some(row) = net.snapshot.units.iter().find(|r| r.id == u.id) {
                    let height = b.y
                        + stronghold_map::BuildingKind::from_name(&b.kind)
                            .and_then(|k| k.roof_profile())
                            .map_or(9.7, |p| p.height)
                        - (row.y - 0.75);
                    t.scale.y = 0.01 * height.max(1.0) / 9.7;
                }
            }
        }
        if u.kind == "assault_ladder" {
            if let Some(b) = net
                .snapshot
                .siege_access
                .iter()
                .find(|a| a.unit_id == u.id)
                .and_then(|a| {
                    net.snapshot
                        .buildings
                        .iter()
                        .find(|b| b.id == a.building_id)
                })
            {
                let row = net.snapshot.units.iter().find(|r| r.id == u.id);
                if let Some(row) = row {
                    let height = b.y
                        + stronghold_map::BuildingKind::from_name(&b.kind)
                            .and_then(|k| k.roof_profile())
                            .map_or(0., |p| p.height)
                        - (row.y - 0.75);
                    let distance = Vec2::new(b.x - row.x, b.z - row.z)
                        .length()
                        .min(4.5)
                        .max(1.);
                    let angle = height.atan2(distance);
                    t.rotation = yaw * Quat::from_rotation_x(-angle);
                    t.scale.z = height.hypot(distance) / 8. * 0.01;
                    t.translation = yaw * Vec3::new(0., height * 0.5 - 0.75, distance * 0.5);
                }
            }
        }
    }
    for (owner, u) in &units {
        if !stronghold_map::units::UnitKind::from_name(&u.kind).is_some_and(|k| k.is_siege())
            || shown.contains(&owner)
        {
            continue;
        }
        let index = match u.kind.as_str() {
            "ballista" => 1,
            "onager" => 2,
            "stone_ballista" => 4,
            "scorpion" => 5,
            "ram" => 6,
            "covered_ram" => 7,
            "siege_tower" => 8,
            "assault_ladder" => 9,
            _ => 10,
        };
        if let Some(model) = assets.models.get(index) {
            commands.entity(owner).with_children(|p| {
                p.spawn((
                    SiegeVisual { owner },
                    WorldAssetRoot(model.scene.clone()),
                    Transform::from_xyz(0., -0.75, 0.).with_scale(Vec3::splat(0.01)),
                ));
            });
        }
    }
    let mut planted = std::collections::HashSet::new();
    for (e, f) in &fields {
        if net
            .snapshot
            .buildings
            .iter()
            .any(|b| b.id == f.building && b.kind == "wheat_farm")
        {
            planted.insert(f.building);
        } else {
            commands.entity(e).despawn();
        }
    }
    if let Some(model) = assets.models.get(3) {
        for b in net
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.kind == "wheat_farm" && !planted.contains(&b.id))
        {
            commands.spawn((
                FieldVisual { building: b.id },
                WorldAssetRoot(model.scene.clone()),
                Transform::from_xyz(b.x, b.y, b.z)
                    .with_rotation(Quat::from_rotation_y(
                        b.rotation as f32 * std::f32::consts::FRAC_PI_2,
                    ))
                    .with_scale(Vec3::splat(0.01)),
            ));
        }
    }
}
pub(crate) fn ui(
    mut contexts: EguiContexts,
    mut net: ResMut<Multiplayer>,
    selection: Res<BuildingInteraction>,
    atlas: Res<resources::ResourceAtlas>,
) -> Result {
    if net.snapshot.game_id.is_none() {
        return Ok(());
    }
    let texture = atlas.texture(&mut contexts);
    let ctx = contexts.ctx_mut()?;
    let own = |id| net.snapshot.buildings.iter().any(|b| b.id == id && b.owned);
    let wheat = net
        .snapshot
        .industry_stocks
        .iter()
        .filter(|s| own(s.building_id))
        .map(|s| s.wheat)
        .sum::<u32>();
    let flour = net
        .snapshot
        .industry_stocks
        .iter()
        .filter(|s| own(s.building_id))
        .map(|s| s.flour)
        .sum::<u32>();
    let adults = net
        .snapshot
        .horses
        .iter()
        .filter(|h| own(h.stable_id) && h.age >= 90)
        .count();
    let foals = net
        .snapshot
        .horses
        .iter()
        .filter(|h| own(h.stable_id) && h.age < 90)
        .count();
    egui::Area::new(egui::Id::new("industry_stock_hud"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10., 44.))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(215))
                .inner_margin(5.)
                .show(ui, |ui| {
                    ui.label(format!(
                        "Korn {wheat} · Mehl {flour} · Pferde {adults} (+{foals} Fohlen)"
                    ));
                });
        });
    let building = selection
        .selected
        .and_then(|id| {
            net.snapshot
                .buildings
                .iter()
                .find(|b| b.id == id && b.owned)
        })
        .cloned();
    let Some(b) = building.filter(|b| matches!(b.kind.as_str(), "stable" | "siege_workshop"))
    else {
        return Ok(());
    };
    let content = |ui: &mut egui::Ui| {
        if b.kind == "stable" {
            let horses = net
                .snapshot
                .horses
                .iter()
                .filter(|h| h.stable_id == b.id)
                .collect::<Vec<_>>();
            ui.label(format!("{} / 8 gezüchtete Pferde", horses.len()));
            ui.small("Die zwei Zuchtpferde bleiben im Stall. Alle 45 Arbeitssekunden kommt ein Fohlen; nach 90 Sekunden ist es ausgewachsen.");
            for h in horses {
                ui.label(if h.age >= 90 {
                    "Erwachsen · bereit für die Reiterei".into()
                } else {
                    format!("Fohlen · {} / 90 Sekunden", h.age)
                });
            }
            let progress = net
                .snapshot
                .producers
                .iter()
                .find(|p| p.building_id == b.id)
                .map_or(0, |p| p.progress);
            ui.add(egui::ProgressBar::new(progress as f32 / 45.).text("Nächstes Fohlen"));
            let price = stronghold_map::civic::horse_quote(
                net.snapshot
                    .prices
                    .iter()
                    .find(|p| p.resource == 10)
                    .map_or(0, |p| p.pressure),
                1,
            );
            if ui.add_enabled(horses_adult(&net,b.id)>0,egui::Button::new(format!("1 Pferd verkaufen · {price} Sesterzen"))).on_hover_text("Erfordert einen eigenen geöffneten Markt oder Hafen. Fohlen und Zuchtpferde bleiben im Stall.").clicked(){net.send(Action::SellHorses{stable_id:b.id,amount:1});}
        } else if let Some(order) = net
            .snapshot
            .factory_orders
            .iter()
            .find(|o| o.building_id == b.id)
        {
            let name = stronghold_map::units::UnitKind::from_name(&order.kind)
                .map(|k| k.label())
                .or_else(|| {
                    stronghold_map::advancement::Equipment::parse(&order.kind).map(|k| k.label())
                })
                .unwrap_or("Fertigung");
            ui.label(name);
            ui.add(
                egui::ProgressBar::new(order.progress as f32 / order.duration as f32)
                    .text(format!("{} / {} Sekunden", order.progress, order.duration)),
            );
            ui.small(format!("Eine besetzte, geöffnete Fabrica arbeitet am Auftrag. {} Bauern sind als Bedienmannschaft eingeplant.",stronghold_map::units::UnitKind::from_name(&order.kind).map_or(0,|k|k.crew())));
        } else {
            ui.label("Belagerungswaffen für Verteidigung und Angriff");
            for kind in stronghold_map::units::UnitKind::SIEGE {
                ui.separator();
                ui.strong(kind.label());
                resources::cost_icons(ui, texture, kind.cost());
                ui.small(format!(
                    "{} freie Bauern · {} Sekunden",
                    kind.crew(),
                    kind.manufacture_seconds()
                ));
                ui.small(kind.description());
                if ui.button("Bauauftrag erteilen").clicked() {
                    net.send(Action::QueueSiege {
                        building_id: b.id,
                        kind: kind.name().into(),
                    });
                }
            }
        }
        if let Some(error) = &net.snapshot.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    };
    egui::Window::new(if b.kind == "stable" {
        "Equile · Pferdezucht"
    } else {
        "Fabrica · Belagerungswerkstatt"
    })
    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12., 96.))
    .default_width(285.)
    .resizable(false)
    .max_height((ctx.viewport_rect().height() - 210.).max(160.))
    .vscroll(true)
    .show(ctx, content);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        animation::AnimationPlugin, asset::AssetPlugin, gltf::GltfPlugin, image::ImagePlugin,
        mesh::MeshPlugin, world_serialization::WorldSerializationPlugin,
    };
    use std::time::Duration;
    #[test]
    fn horse_engine_and_crop_assets_load_and_foals_grow_without_human_rigs() {
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
        .init_asset::<StandardMaterial>()
        .init_resource::<IndustryAssets>()
        .init_resource::<Multiplayer>()
        .add_systems(Startup, setup)
        .add_systems(Update, (load_assets, visuals).chain());
        app.finish();
        app.cleanup();
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings
            .push(network::Building {
                id: 1,
                kind: "stable".into(),
                rotation: 0,
                color: 0,
                owned: true,
                health: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            });
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .horses
            .push(network::HorseView {
                id: 1,
                stable_id: 1,
                age: 0,
            });
        let unit = app
            .world_mut()
            .spawn((
                Unit {
                    kind: "ballista".into(),
                    ..default()
                },
                Transform::default(),
            ))
            .id();
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            app.update();
            if app.world().resource::<IndustryAssets>().models.len() == PATHS.len()
                && app
                    .world_mut()
                    .query::<&AnimationPlayer>()
                    .iter(app.world())
                    .count()
                    >= 3
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Industry assets/horse rigs load"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            app.world().resource::<IndustryAssets>().models[0]
                .clips
                .len(),
            3
        );
        assert_eq!(
            app.world_mut()
                .query::<&SiegeVisual>()
                .iter(app.world())
                .filter(|v| v.owner == unit)
                .count(),
            1
        );
        let small = app
            .world_mut()
            .query::<(&HorseVisual, &Transform)>()
            .iter(app.world())
            .find(|(h, _)| h.id == 1)
            .unwrap()
            .1
            .scale
            .x;
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .horses[0]
            .age = 90;
        app.update();
        let grown = app
            .world_mut()
            .query::<(&HorseVisual, &Transform)>()
            .iter(app.world())
            .find(|(h, _)| h.id == 1)
            .unwrap()
            .1
            .scale
            .x;
        assert!(grown > small * 2.);
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .buildings
            .clear();
        app.world_mut()
            .resource_mut::<Multiplayer>()
            .snapshot
            .horses
            .clear();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&HorseVisual>()
                .iter(app.world())
                .count(),
            0
        );
    }
}

fn horses_adult(net: &Multiplayer, id: u64) -> usize {
    net.snapshot
        .horses
        .iter()
        .filter(|h| h.stable_id == id && h.age >= 90)
        .count()
}
