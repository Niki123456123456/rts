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
    mut needs_open: Local<bool>,
) -> Result {
    if !multiplayer.snapshot.connected || multiplayer.snapshot.game_id.is_none() {
        *needs_open = false;
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
                            ("Fisch",Some(multiplayer.snapshot.fish_stocks.iter().filter(|s|multiplayer.snapshot.buildings.iter().any(|b|b.id==s.building_id&&b.owned)).map(|s|s.amount).sum())),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if index > 0 {
                                ui.separator();
                            }
                            if index==7 {ui.small("Fisch");}else{
                                ui.add(egui::Image::new((texture, egui::vec2(18.,18.))).uv(resource_uv(index))).on_hover_text(name);
                            }
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
                        let tooltip = format!("Bauern: {} frei am Lagerfeuer · {} arbeiten · {} Bewohner / {} Wohnplätze. Freie Bauern besetzen offene Produktionsgebäude automatisch. Ein Gebäude zum Pausieren schließen.",peasants.capacity.saturating_sub(peasants.used),peasants.used,peasants.capacity,room);
                        if satisfaction_icon(ui,&multiplayer) { *needs_open = !*needs_open; }
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
    if *needs_open {
        egui::Window::new("Zufriedenheit")
            .open(&mut needs_open)
            .default_pos(egui::pos2(24., 60.))
            .default_width(370.)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| satisfaction_details(ui, &multiplayer));
    }
    Ok(())
}

fn satisfaction_icon(ui: &mut egui::Ui, multiplayer: &Multiplayer) -> bool {
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(19., 19.), egui::Sense::click());
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
    let clicked = response
        .on_hover_text("Zufriedenheit: Für Einflussfaktoren anklicken")
        .clicked();
    let label = ui
        .add(
            egui::Label::new(
                egui::RichText::new(
                    state.map_or_else(|| "…".into(), |n| format!("{}%", n.happiness)),
                )
                .size(10.5)
                .color(color),
            )
            .sense(egui::Sense::click()),
        )
        .on_hover_text("Für Einflussfaktoren anklicken");
    clicked || label.clicked()
}

fn satisfaction_details(ui: &mut egui::Ui, multiplayer: &Multiplayer) {
    let count = population(multiplayer).capacity;
    let stage = map::needs::NeedsStage::for_population(count);
    let state = multiplayer.snapshot.settlement_needs.iter().find(|n| {
        multiplayer
            .snapshot
            .buildings
            .iter()
            .any(|b| b.id == n.building_id && b.owned)
    });
    let Some(n) = state else {
        ui.label("Zufriedenheit wird ermittelt …");
        return;
    };
    ui.heading(format!("{}% · {} Bewohner", n.happiness, count));
    ui.label(format!("Grundzufriedenheit: {} Punkte", stage.base));
    ui.small("Mit wachsender Bevölkerung steigen die Ansprüche.");
    ui.separator();
    egui::Grid::new("needs_factors")
        .spacing(egui::vec2(18., 7.))
        .show(ui, |ui| {
            for (name, points, threshold) in [
                ("Nahrung", n.food, 0),
                ("Wohnraum", n.housing, 0),
                ("Wasser · Brunnen", n.water, 8),
                ("Nahrungsvielfalt", n.variety, 16),
                ("Arbeitsplätze", n.employment, 16),
                ("Gesundheit · Apotheke", n.health, 24),
                ("Freizeit · Wirtshaus / Garten", n.leisure, 32),
                ("Warenbestände", n.goods, 40),
            ] {
                let active = count >= threshold;
                ui.label(name);
                if active {
                    ui.colored_label(
                        if points < 0 {
                            egui::Color32::LIGHT_RED
                        } else if points > 0 {
                            egui::Color32::LIGHT_GREEN
                        } else {
                            egui::Color32::GRAY
                        },
                        format!("{points:+} Punkte"),
                    );
                } else {
                    ui.weak(format!("ab {threshold} Bewohnern"));
                }
                ui.end_row();
            }
        });
    ui.separator();
    if count <= 4 {
        ui.label("Starthilfe: Die ersten vier Bauern bleiben auch ohne Nahrung zufrieden. Baue in Ruhe Kornspeicher und Nahrungsproduktion auf.");
    }
    ui.small("Ab 5 Bewohnern zählt fehlende Nahrung. Nahrung wird alle 30 s aus dem Kornspeicher verbraucht.");
    let room = map::population::Population::from_buildings(
        multiplayer
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.owned)
            .filter_map(|b| BuildingKind::from_name(&b.kind)),
    )
    .capacity;
    if count >= room {
        ui.label("Kein freier Wohnraum: Baue eine Unterkunft für weiteren Zuzug.");
    }
    if n.happiness < 40 {
        ui.label("Freie Bauern wandern alle 14 s ab.");
    } else if n.immigration_seconds == 0 {
        ui.label("Zurzeit kein Zuzug.");
    } else {
        ui.label(format!(
            "Ein neuer Bauer alle {} s, solange Wohnraum frei ist.",
            n.immigration_seconds
        ));
    }
    ui.small(
        "Die Summe wird auf 0 bis 100% begrenzt. Versorgungsgebäude findest du unter Bedürfnisse.",
    );
}
