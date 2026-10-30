use crate::*;
use stronghold_map::BuildingKind;

pub(crate) const BUILD_MENU: [BuildingKind; 33] = [
    BuildingKind::WoodenWall,
    BuildingKind::TowerWooden,
    BuildingKind::Market,
    BuildingKind::AppleFarm,
    BuildingKind::DairyFarm,
    BuildingKind::Housing,
    BuildingKind::Barracks,
    BuildingKind::Stockpile,
    BuildingKind::Granary,
    BuildingKind::TowerRoundShort,
    BuildingKind::TowerRoundTall,
    BuildingKind::TowerGreat,
    BuildingKind::StoneWall,
    BuildingKind::StoneWallLow,
    BuildingKind::StoneWallHigh,
    BuildingKind::StoneWallCrenellated,
    BuildingKind::GatehouseSmall,
    BuildingKind::GatehouseLarge,
    BuildingKind::GatehouseWooden,
    BuildingKind::DrawbridgeGate,
    BuildingKind::TowerLookout,
    BuildingKind::TowerSquare,
    BuildingKind::TowerStairwell,
    BuildingKind::Quarry,
    BuildingKind::IronMine,
    BuildingKind::SawPit,
    BuildingKind::OxTether,
    BuildingKind::Fishpond,
    BuildingKind::HuntersPost,
    BuildingKind::Well,
    BuildingKind::Apothecary,
    BuildingKind::Inn,
    BuildingKind::GardenersHut,
];

fn uv(x: f32, y: f32, width: f32, height: f32) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(x / 1024., y / 1024.),
        egui::pos2((x + width) / 1024., (y + height) / 1024.),
    )
}

fn tile(row: u8, column: u8) -> egui::Rect {
    uv(
        (column - 1) as f32 * 56.,
        44. + (row - 1) as f32 * 56.,
        54.,
        54.,
    )
}

pub(crate) fn image_button(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    uv: egui::Rect,
    size: egui::Vec2,
    active: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().image(texture, rect, uv, egui::Color32::WHITE);
    if active || response.hovered() {
        ui.painter().rect_stroke(
            rect.shrink(1.),
            2.,
            egui::Stroke::new(
                2.,
                if active {
                    egui::Color32::GOLD
                } else {
                    egui::Color32::WHITE
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(crate) fn category_button(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    category: &str,
    active: bool,
) -> bool {
    if category == "services" {
        return image_button(ui, texture, tile(2, 14), egui::vec2(28., 20.), active)
            .on_hover_text("Bedürfnisse: Brunnen, Apotheke, Wirtshaus und Garten")
            .clicked();
    }
    let (x, y, label) = match category {
        "fortifications" => (81., 932., "Befestigungen"),
        "military" => (115., 960., "Militär"),
        _ => (115., 932., "Versorgung"),
    };
    image_button(
        ui,
        texture,
        uv(x, y, 34., 28.),
        egui::vec2(28., 20.),
        active,
    )
    .on_hover_text(label)
    .clicked()
}

pub(crate) fn leave_button(ui: &mut egui::Ui, texture: egui::TextureId) -> bool {
    image_button(
        ui,
        texture,
        uv(0., 803., 32., 26.),
        egui::vec2(32., 26.),
        false,
    )
    .on_hover_text("Spiel verlassen")
    .clicked()
}

pub(crate) fn remove_button(ui: &mut egui::Ui, texture: egui::TextureId) -> bool {
    image_button(
        ui,
        texture,
        uv(181., 879., 32., 26.),
        egui::vec2(32., 26.),
        false,
    )
    .on_hover_text("Gebäude abreißen")
    .clicked()
}

pub(crate) fn building_button(
    ui: &mut egui::Ui,
    texture: egui::TextureId,
    kind: BuildingKind,
    active: bool,
) -> egui::Response {
    use BuildingKind::*;
    let (row, column) = match kind {
        Housing => (6, 5),
        Barracks => (2, 7),
        Stockpile => (4, 5),
        Granary => (3, 6),
        WoodenWall | StoneWall | StoneWallLow | StoneWallHigh | StoneWallCrenellated => (1, 6),
        DrawbridgeGate => (1, 4),
        TowerLookout => (1, 11),
        TowerSquare => (1, 13),
        TowerStairwell => (1, 9),
        GatehouseWooden => (1, 1),
        GatehouseSmall => (1, 2),
        GatehouseMedium => (1, 3),
        GatehouseLarge => (1, 4),
        Sallyport => (2, 4),
        Watchpost => (1, 18),
        TowerWooden => (1, 10),
        TowerRoundShort => (1, 12),
        TowerRoundTall => (1, 14),
        TowerGreat => (1, 15),
        FlagTower => (2, 6),
        TowerWalk => (2, 8),
        BridgeEnd => (1, 6),
        BridgeMiddle => (1, 8),
        BridgeWooden => (3, 1),
        AppleFarm => (3, 10),
        DairyFarm => (3, 11),
        WheatFarm => (3, 12),
        HopsFarm => (3, 15),
        Vineyard => (3, 18),
        PigFarm => (3, 17),
        SheepFarm => (3, 16),
        HuntersPost => (3, 9),
        Fishpond => (3, 7),
        Dovecote => (5, 16),
        GardenersHut => (3, 6),
        VegetableGarden => (4, 5),
        Beehive => (5, 6),
        Windmill => (3, 13),
        Bakery => (3, 14),
        Brewery => (4, 1),
        VintnersWorkshop => (4, 3),
        Kitchen => (4, 11),
        ServantsPantry => (4, 12),
        Inn => (4, 13),
        Market => (2, 10),
        SawPit => (4, 6),
        Quarry => (4, 7),
        IronMine => (4, 8),
        PitchRig => (4, 8),
        OxTether => (2, 11),
        CartersPost => (4, 7),
        WeaversWorkshop => (4, 6),
        ChandlersWorkshop => (4, 10),
        Well => (2, 14),
        WaterPot => (4, 18),
        GongPit => (5, 15),
        Apothecary => (4, 2),
        FalconersPost => (5, 17),
        Keep => (1, 15),
        Treasury => (3, 8),
    };
    image_button(ui, texture, tile(row, column), egui::vec2(40., 40.), active)
}
