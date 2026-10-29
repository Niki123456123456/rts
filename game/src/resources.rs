use crate::*;
use bevy_egui::EguiTextureHandle;
use stronghold_map::{self as map, BuildingKind};

#[derive(Resource)]
pub(crate) struct ResourceAtlas(Handle<Image>);
impl ResourceAtlas {
    pub(crate) fn texture(&self, contexts: &mut EguiContexts) -> egui::TextureId {
        contexts.add_image(EguiTextureHandle::Strong(self.0.clone()))
    }
}
pub(crate) fn setup_resource_ui(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(ResourceAtlas(server.load("ui/ingameui.png")));
}
pub(crate) fn employed(multiplayer: &Multiplayer, color: u32) -> u32 {
    multiplayer
        .snapshot
        .workers
        .iter()
        .filter(|w| {
            multiplayer
                .snapshot
                .buildings
                .iter()
                .any(|b| b.id == w.source_building && b.color == color)
        })
        .count() as u32
}
pub(crate) fn population(multiplayer: &Multiplayer) -> map::population::Population {
    let used = multiplayer
        .snapshot
        .buildings
        .iter()
        .find(|b| b.owned)
        .map_or(0, |b| employed(multiplayer, b.color));
    let capacity = multiplayer
        .snapshot
        .keep_population
        .iter()
        .find(|k| {
            multiplayer
                .snapshot
                .buildings
                .iter()
                .any(|b| b.id == k.building_id && b.owned)
        })
        .map_or(used, |k| k.arrived.max(used));
    map::population::Population { used, capacity }
}
// One-based row 9, columns 6–8, on the original 1024 px atlas. Tile pitch 56;
// the first icon row starts at y=44. Trim the surrounding gap by two pixels.
fn resource_uv(index: usize) -> egui::Rect {
    // Food: row 10, columns 5–8 (apples, bread, cheese, meat).
    let (x, y) = if index < 3 {
        (280. + index as f32 * 56., 494.)
    } else {
        (224. + (index - 3) as f32 * 56., 550.)
    };
    egui::Rect::from_min_max(
        egui::pos2((x + 2.) / 1024., y / 1024.),
        egui::pos2((x + 54.) / 1024., (y + 52.) / 1024.),
    )
}
pub(crate) fn resource_ui(
    mut contexts: EguiContexts,
    multiplayer: Res<Multiplayer>,
    atlas: Res<ResourceAtlas>,
) -> Result {
    if !multiplayer.snapshot.connected || multiplayer.snapshot.game_id.is_none() {
        return Ok(());
    }
    let texture = atlas.texture(&mut contexts);
    let ctx = contexts.ctx_mut()?;
    egui::Area::new("resource_stock".into())
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-6., 6.))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(215))
                .stroke(egui::Stroke::new(1., egui::Color32::from_rgb(150, 125, 75)))
                .inner_margin(4.)
                .corner_radius(2.5)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.;
                        ui.spacing_mut().interact_size.y = 18.;
                        let stock = multiplayer.snapshot.resources;
                        for (index, (name, value)) in [
                            ("Holz", stock.map(|s| s.wood)),
                            ("Stein", stock.map(|s| s.stone)),
                            ("Eisen", stock.map(|s| s.iron)),
                            ("Äpfel", stock.map(|s| s.apples)),
                            ("Brot", stock.map(|s| s.bread)),
                            ("Käse", stock.map(|s| s.cheese)),
                            ("Fleisch", stock.map(|s| s.meat)),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if index > 0 {
                                ui.separator();
                            }
                            ui.add(
                                egui::Image::new((texture, egui::vec2(18., 18.)))
                                    .uv(resource_uv(index)),
                            )
                            .on_hover_text(name);
                            ui.label(
                                egui::RichText::new(
                                    value.map_or_else(|| "…".into(), |v| v.to_string()),
                                )
                                .size(10.5)
                                .color(egui::Color32::WHITE),
                            )
                            .on_hover_text(name);
                        }
                        ui.separator();
                        let peasants = population(&multiplayer);
                        let room=map::population::Population::from_buildings(multiplayer.snapshot.buildings.iter()
                            .filter(|b|b.owned).filter_map(|b|BuildingKind::from_name(&b.kind))).capacity;
                        let tooltip = format!("Bauern: {} frei am Lagerfeuer · {} arbeiten · {} Bewohner / {} Wohnplätze. Produktionsgebäude anklicken und Bauer zuweisen wählen.",peasants.capacity.saturating_sub(peasants.used),peasants.used,peasants.capacity,room);
                        satisfaction_icon(ui,&multiplayer);
                        ui.separator();
                        ui.add(egui::Image::new((texture, egui::vec2(14., 18.))).uv(
                            egui::Rect::from_min_max(
                                egui::pos2(837. / 1024., 2. / 1024.),
                                egui::pos2(859. / 1024., 30. / 1024.),
                            ),
                        )).on_hover_text(&tooltip);
                        ui.label(egui::RichText::new(format!("{} frei", peasants.capacity.saturating_sub(peasants.used)))
                            .size(10.5).color(if peasants.used > peasants.capacity {
                                egui::Color32::LIGHT_RED
                            } else { egui::Color32::WHITE }))
                            .on_hover_text(tooltip);
                    });
                });
        });
    Ok(())
}

fn satisfaction_icon(ui: &mut egui::Ui, multiplayer: &Multiplayer) {
    let state = multiplayer.snapshot.settlement_needs.iter().find(|n| {
        multiplayer
            .snapshot
            .buildings
            .iter()
            .any(|b| b.id == n.building_id && b.owned)
    });
    let score = state.map_or(50, |s| s.happiness);
    let color = if score >= 65 {
        egui::Color32::from_rgb(111, 210, 107)
    } else if score >= 50 {
        egui::Color32::from_rgb(240, 205, 89)
    } else {
        egui::Color32::from_rgb(233, 107, 86)
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(19., 19.), egui::Sense::hover());
    let center = rect.center();
    let ink = egui::Color32::from_rgb(35, 31, 22);
    ui.painter().circle_filled(center, 8., color);
    for x in [-2.8, 2.8] {
        ui.painter()
            .circle_filled(center + egui::vec2(x, -2.), 1., ink);
    }
    let middle = if score >= 65 {
        4.8
    } else if score >= 50 {
        3.1
    } else {
        1.4
    };
    for (a, b) in [((-3.5, 3.1), (0., middle)), ((0., middle), (3.5, 3.1))] {
        ui.painter().line_segment(
            [center + egui::vec2(a.0, a.1), center + egui::vec2(b.0, b.1)],
            egui::Stroke::new(1.2, ink),
        );
    }
    let detail=state.map_or_else(||"Zufriedenheit wird ermittelt …".to_owned(),|n| {
        let movement=if n.happiness<40 {"Freie Bauern wandern alle 14 s ab".into()}
            else if n.immigration_seconds==0 {"Zurzeit kein Zuzug".into()}
            else {format!("Ein Zuwanderer alle {} s, solange Wohnraum frei ist",n.immigration_seconds)};
        format!("Zufriedenheit: {} / 100\nGrundwert: 50\nNahrung: {:+}\nNahrungsvielfalt: {:+}\nWohnraum: {:+}\nWasser (Brunnen): {:+}\nGesundheit (Apotheke): {:+}\nFreizeit (Wirtshaus / Garten): {:+}\nWarenbestände: {:+}\nArbeitsplätze: {:+}\n\n{}\nNahrung wird alle 30 s aus dem Kornspeicher verbraucht.\nVersorgungsgebäude findest du unter Bedürfnisse.",n.happiness,n.food,n.variety,n.housing,n.water,n.health,n.leisure,n.goods,n.employment,movement)
    });
    response.on_hover_text(&detail);
    ui.label(
        egui::RichText::new(state.map_or_else(|| "…".into(), |n| format!("{}%", n.happiness)))
            .size(10.5)
            .color(color),
    )
    .on_hover_text(detail);
}
