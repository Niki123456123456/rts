use crate::*;
use bevy_egui::EguiTextureHandle;
use stronghold_map::units::UnitKind;

#[derive(Resource)]
pub(crate) struct UnitPortraits(Handle<Image>);

pub(crate) fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(UnitPortraits(server.load("ui/ingameui2.png")));
}

impl UnitPortraits {
    pub(crate) fn texture(&self, contexts: &mut EguiContexts) -> egui::TextureId {
        contexts.add_image(EguiTextureHandle::Strong(self.0.clone()))
    }
}

pub(crate) fn portrait(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    kind: UnitKind,
    count: Option<usize>,
    recruit: bool,
) -> egui::Response {
    // Original 512×512 atlas: 57 px column pitch, 92 px row pitch.
    // One-based cells: peasant (1, 1), lord (4, 4).
    let (column, row) = match kind {
        UnitKind::ArmedPeasant => (0., 0.),
        UnitKind::Lord => (3., 3.),
        UnitKind::Archer => (2., 0.),
        UnitKind::LightSwordsman => (4., 0.),
        UnitKind::LightCavalry => (0., 1.),
    };
    let uv = egui::Rect::from_min_max(
        egui::pos2(column * 57. / 512., row * 92. / 512.),
        egui::pos2((column * 57. + 56.) / 512., (row * 92. + 91.) / 512.),
    );
    // Reserve the entire portrait once. A second layout allocation here would
    // shift the horizontal row and push the image below the panel's clip edge.
    let (id, space) = ui.allocate_space(egui::vec2(70., 88.));
    let rect = egui::Rect::from_min_size(space.min + egui::vec2(2., 10.), egui::vec2(48., 78.));
    ui.painter().image(texture, rect, uv, egui::Color32::WHITE);
    let response = ui
        .interact(
            rect,
            id,
            if recruit {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_text(kind.label());
    if let Some(count) = count {
        let badge = egui::Rect::from_center_size(
            rect.right_top(),
            egui::vec2(if count >= 100 { 30. } else { 24. }, 20.),
        );
        ui.painter()
            .rect_filled(badge, 4., egui::Color32::from_black_alpha(230));
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::proportional(16.),
            egui::Color32::WHITE,
        );
    }
    if recruit {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}
