use crate::*;
use stronghold_map::BuildingKind;

pub(crate) const BUILD_MENU: [BuildingKind; 39] = [
    BuildingKind::Stable,
    BuildingKind::SiegeWorkshop,
    BuildingKind::Harbor,
    BuildingKind::WheatFarm,
    BuildingKind::Windmill,
    BuildingKind::Bakery,
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
    if matches!(
        kind,
        BuildingKind::Stable
            | BuildingKind::SiegeWorkshop
            | BuildingKind::Harbor
            | BuildingKind::Windmill
            | BuildingKind::Bakery
    ) {
        return industry_button(ui, kind, active);
    }
    if kind.is_wall() {
        return wall_button(ui, kind, active);
    }
    use BuildingKind::*;
    let (row, column) = match kind {
        Stable => (3, 16),
        SiegeWorkshop => (2, 7),
        Harbor => (2, 11),
        StoneWallCrenellatedLow | StoneWallCrenellatedMedium | StoneWallCrenellatedWoodHeight => {
            (1, 6)
        }
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

/// Small material/height silhouettes stay readable at the actual forty-pixel menu size.
fn wall_button(ui: &mut egui::Ui, kind: BuildingKind, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40., 40.), egui::Sense::click());
    let p = ui.painter();
    let point = |x: f32, y: f32| rect.min + egui::vec2(x, y);
    p.rect_filled(rect, 3., egui::Color32::from_rgb(39, 33, 25));
    let poly = |points: Vec<egui::Pos2>, color| {
        p.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::new(0.6, egui::Color32::from_black_alpha(140)),
        ))
    };
    if kind == BuildingKind::WoodenWall {
        for i in 0..6 {
            let x = 4. + i as f32 * 5.;
            let top = 11. + (i % 2) as f32 * 2.;
            poly(
                vec![
                    point(x, 34.),
                    point(x, top),
                    point(x + 2., top - 5.),
                    point(x + 4., top),
                    point(x + 4., 34.),
                ],
                egui::Color32::from_rgb(151 + (i % 2) * 18, 92, 42),
            );
        }
        for y in [22., 29.] {
            p.line_segment(
                [point(3., y), point(36., y)],
                egui::Stroke::new(2., egui::Color32::from_rgb(85, 48, 22)),
            );
        }
    } else {
        let height = match kind {
            BuildingKind::StoneWallLow => 10.,
            BuildingKind::StoneWallHigh | BuildingKind::StoneWallCrenellated => 26.,
            BuildingKind::StoneWallCrenellatedLow => 10.,
            BuildingKind::StoneWallCrenellatedWoodHeight => 12.,
            _ => 18.,
        };
        let top = 34. - height;
        poly(
            vec![
                point(29., top),
                point(36., top - 5.),
                point(36., 29.),
                point(29., 34.),
            ],
            egui::Color32::from_rgb(120, 116, 103),
        );
        p.rect_filled(
            egui::Rect::from_min_max(point(4., top), point(29., 34.)),
            0.,
            egui::Color32::from_rgb(184, 179, 158),
        );
        poly(
            vec![
                point(4., top),
                point(11., top - 5.),
                point(36., top - 5.),
                point(29., top),
            ],
            egui::Color32::from_rgb(218, 211, 185),
        );
        let mut y = 34. - 5.;
        let mut row = 0;
        while y > top {
            p.line_segment(
                [point(4., y), point(29., y)],
                egui::Stroke::new(0.8, egui::Color32::from_rgb(102, 100, 90)),
            );
            for i in 0..4 {
                let x = 6. + i as f32 * 8. + (row % 2) as f32 * 4.;
                if x < 29. {
                    p.line_segment(
                        [point(x, y), point(x, (y + 5.).min(34.))],
                        egui::Stroke::new(0.7, egui::Color32::from_rgb(102, 100, 90)),
                    );
                }
            }
            row += 1;
            y -= 5.;
        }
        if kind.is_crenellated() {
            for x in [4., 14., 24.] {
                p.rect_filled(
                    egui::Rect::from_min_max(point(x, top - 6.), point(x + 5., top + 1.)),
                    0.,
                    egui::Color32::from_rgb(210, 203, 177),
                );
                poly(
                    vec![
                        point(x, top - 6.),
                        point(x + 5., top - 9.),
                        point(x + 10., top - 9.),
                        point(x + 5., top - 6.),
                    ],
                    egui::Color32::from_rgb(233, 225, 198),
                );
            }
        }
    }
    p.rect_stroke(
        rect.shrink(0.5),
        3.,
        egui::Stroke::new(
            if active || response.hovered() { 2. } else { 1. },
            if active {
                egui::Color32::GOLD
            } else if response.hovered() {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(102, 84, 51)
            },
        ),
        egui::StrokeKind::Inside,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn industry_button(ui: &mut egui::Ui, kind: BuildingKind, active: bool) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(egui::vec2(40., 40.), egui::Sense::click());
    let p = ui.painter();
    let q = |x, y| r.min + egui::vec2(x, y);
    let stone = egui::Color32::from_rgb(197, 176, 130);
    let wood = egui::Color32::from_rgb(152, 98, 51);
    let red = egui::Color32::from_rgb(168, 76, 45);
    p.rect_filled(r, 3., egui::Color32::from_rgb(39, 33, 25));
    match kind {
        BuildingKind::Harbor => {
            for y in [27., 31., 35.] {
                p.line_segment(
                    [q(3., y), q(37., y)],
                    egui::Stroke::new(2., egui::Color32::from_rgb(57, 133, 160)),
                );
            }
            p.rect_filled(egui::Rect::from_min_max(q(5., 14.), q(24., 28.)), 1., stone);
            p.line_segment([q(8., 27.), q(36., 27.)], egui::Stroke::new(4., wood));
            p.line_segment([q(30., 27.), q(30., 10.)], egui::Stroke::new(2., wood));
            p.line_segment([q(24., 11.), q(36., 11.)], egui::Stroke::new(2., wood));
        }
        BuildingKind::Stable => {
            p.rect_filled(egui::Rect::from_min_max(q(5., 15.), q(23., 33.)), 1., stone);
            p.add(egui::Shape::convex_polygon(
                vec![q(3., 16.), q(14., 7.), q(26., 16.)],
                red,
                egui::Stroke::NONE,
            ));
            p.circle_filled(q(29., 22.), 4., wood);
            p.line_segment([q(28., 22.), q(34., 14.)], egui::Stroke::new(4., wood));
            p.circle_filled(q(35., 13.), 3., wood);
            for x in [26., 31.] {
                p.line_segment([q(x, 24.), q(x, 34.)], egui::Stroke::new(2., wood));
            }
        }
        BuildingKind::SiegeWorkshop => {
            p.line_segment([q(9., 30.), q(31., 30.)], egui::Stroke::new(4., wood));
            p.line_segment([q(20., 29.), q(20., 11.)], egui::Stroke::new(4., wood));
            p.line_segment([q(5., 17.), q(35., 17.)], egui::Stroke::new(3., stone));
            p.line_segment([q(6., 17.), q(20., 24.)], egui::Stroke::new(1., stone));
            p.line_segment([q(34., 17.), q(20., 24.)], egui::Stroke::new(1., stone));
            for x in [9., 31.] {
                p.circle_filled(q(x, 32.), 4., wood);
            }
        }
        BuildingKind::Windmill => {
            p.rect_filled(egui::Rect::from_min_max(q(5., 29.), q(35., 34.)), 1., stone);
            p.circle_filled(q(20., 24.), 10., stone);
            p.circle_stroke(q(20., 24.), 7., egui::Stroke::new(2., wood));
            p.line_segment([q(8., 10.), q(32., 10.)], egui::Stroke::new(3., wood));
            p.line_segment([q(20., 10.), q(20., 22.)], egui::Stroke::new(3., wood));
        }
        _ => {
            p.rect_filled(egui::Rect::from_min_max(q(4., 22.), q(36., 34.)), 1., stone);
            p.circle_filled(q(20., 22.), 14., red);
            p.rect_filled(egui::Rect::from_min_max(q(4., 22.), q(36., 34.)), 1., red);
            p.rect_filled(
                egui::Rect::from_min_max(q(13., 21.), q(27., 34.)),
                3.,
                egui::Color32::from_rgb(32, 23, 17),
            );
            p.circle_filled(q(19., 29.), 3., egui::Color32::GOLD);
            p.circle_filled(q(23., 30.), 2., egui::Color32::from_rgb(240, 117, 26));
        }
    }
    p.rect_stroke(
        r.shrink(1.),
        2.,
        egui::Stroke::new(
            if active || response.hovered() { 2. } else { 1. },
            if active {
                egui::Color32::GOLD
            } else {
                egui::Color32::from_gray(95)
            },
        ),
        egui::StrokeKind::Inside,
    );
    response
}
