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
pub(crate) fn population(multiplayer: &Multiplayer) -> map::population::Population {
    map::population::Population::from_buildings(
        multiplayer
            .snapshot
            .buildings
            .iter()
            .filter(|b| b.owned)
            .filter_map(|b| BuildingKind::from_name(&b.kind)),
    )
}
// One-based row 9, columns 6–8, on the original 1024 px atlas. Tile pitch 56;
// the first icon row starts at y=44. Trim the surrounding gap by two pixels.
fn resource_uv(index: usize) -> egui::Rect {
    let x = 280. + index as f32 * 56.;
    egui::Rect::from_min_max(
        egui::pos2((x + 2.) / 1024., 494. / 1024.),
        egui::pos2((x + 54.) / 1024., 546. / 1024.),
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
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12., 12.))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(215))
                .stroke(egui::Stroke::new(1., egui::Color32::from_rgb(150, 125, 75)))
                .inner_margin(8.)
                .corner_radius(5.)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.;
                        let stock = multiplayer.snapshot.resources;
                        for (index, (name, value)) in [
                            ("Holz", stock.map(|s| s.wood)),
                            ("Stein", stock.map(|s| s.stone)),
                            ("Eisen", stock.map(|s| s.iron)),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            if index > 0 {
                                ui.separator();
                            }
                            ui.add(
                                egui::Image::new((texture, egui::vec2(36., 36.)))
                                    .uv(resource_uv(index)),
                            )
                            .on_hover_text(name);
                            ui.label(
                                egui::RichText::new(
                                    value.map_or_else(|| "…".into(), |v| v.to_string()),
                                )
                                .size(21.)
                                .color(egui::Color32::WHITE),
                            )
                            .on_hover_text(name);
                        }
                        ui.separator();
                        let peasants = population(&multiplayer);
                        let tooltip = format!(
                            "Bauern: {} von {} belegt. Jede Unterkunft bietet vier Bauern; nur Produktionsgebäude benötigen einen Bauern.",
                            peasants.used, peasants.capacity
                        );
                        ui.add(egui::Image::new((texture, egui::vec2(28., 36.))).uv(
                            egui::Rect::from_min_max(
                                egui::pos2(837. / 1024., 2. / 1024.),
                                egui::pos2(859. / 1024., 30. / 1024.),
                            ),
                        )).on_hover_text(&tooltip);
                        ui.label(egui::RichText::new(format!("{}/{}", peasants.used, peasants.capacity))
                            .size(21.).color(if peasants.used > peasants.capacity {
                                egui::Color32::LIGHT_RED
                            } else { egui::Color32::WHITE }))
                            .on_hover_text(tooltip);
                    });
                });
        });
    Ok(())
}
