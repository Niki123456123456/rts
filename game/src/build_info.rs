//! Contextual information for the currently selected building.
use crate::*;
use stronghold_map::BuildingKind;
pub(crate) fn ui(
    mut contexts: EguiContexts,
    state: Res<BuildingInteraction>,
    net: Res<Multiplayer>,
    atlas: Res<resources::ResourceAtlas>,
    panels: Res<SelectionPanel>,
) -> Result {
    let Some(kind) = state.placing.filter(|_| net.snapshot.game_id.is_some()) else {
        return Ok(());
    };
    let texture = atlas.texture(&mut contexts);
    let ctx = contexts.ctx_mut()?;
    let available = (panels.1.map_or(ctx.viewport_rect().bottom(), |r| r.top()) - 116.).max(120.);
    let content = |ui: &mut egui::Ui| {
        ui.set_max_width(268.);
        ui.label(description(kind));
        ui.separator();
        ui.strong("Baukosten");
        resources::cost_icons(ui, texture, kind.cost());
        if kind.is_wall() {
            ui.small("Je Mauerabschnitt");
            if state.wall_start.is_some() {
                let c = kind.cost();
                ui.label(format!("{} neue Abschnitte", state.wall_cost));
                resources::cost_icons(
                    ui,
                    texture,
                    stronghold_map::Cost::new(
                        c.wood * state.wall_cost,
                        c.stone * state.wall_cost,
                        c.iron * state.wall_cost,
                    ),
                );
            }
        }
        ui.separator();
        ui.strong("Bauplatz");
        ui.label(match kind {
   BuildingKind::SawPit=>"Auf Waldfläche. Bäume im Baufeld werden gerodet.",
BuildingKind::Quarry=>"Auf einem Steinvorkommen. Benötigt einen nahen Ochsenkarren für den Abtransport.",
BuildingKind::IronMine=>"Auf einem Eisenvorkommen. Benötigt einen nahen Ochsenkarren für den Abtransport.",
BuildingKind::Fishpond=>"Im Wasser bei einem Fischschwarm; freies Ufer für den Fischer erforderlich.",
BuildingKind::OxTether=>"Auf Land oder in Rohstofffeldern. Bedient passende Betriebe bis 80 Meter automatisch.",
_=>"Auf geeignetem Land. Rote Flächen sind blockiert. Direkt an Nachbarn anschließen ist möglich."});
        if kind.peasant_cost() > 0 {
            ui.label("1 Arbeitsplatz · wird automatisch mit einem freien Bauern besetzt.");
        }
        if let Some(roof) = kind.roof_profile() {
            ui.label(format!("Begehbare Höhe: {:.1} m", roof.height));
        }
        ui.separator();
        ui.strong("Steuerung");
        ui.label(if kind.is_wall() {
            "Linksklick: Startpunkt und Endpunkt\nRechtsklick / Esc: abbrechen"
        } else {
            "Linksklick: platzieren\nR: um 90° drehen\nRechtsklick / Esc: abbrechen"
        });
    };
    egui::Window::new(kind.label())
        .id(egui::Id::new("construction_details"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12., 96.))
        .default_width(268.)
        .max_height(available)
        .vscroll(true)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .show(ctx, content);
    Ok(())
}
fn description(kind: BuildingKind) -> &'static str {
    use BuildingKind::*;
    match kind {
 Housing=>"Bietet vier weitere Wohnplätze und ermöglicht zusätzlichen Zuzug.",
Stockpile=>"Lagert Holz, Stein und Eisen. Arbeiter und Karren liefern ihre Rohstoffe hier ab.",
Granary=>"Lagert Lebensmittel für die Bevölkerung. Rationen werden im Palast festgelegt.",
Market=>"Römischer Handelsplatz zum Kaufen, Verkaufen und Tauschen. Angebot und Nachfrage verändern die Preise.",
Barracks=>"Bildet aus freien Bauern bewaffnete Bauern, Bogenschützen, leichte Schwertkämpfer und leichte Reiter aus.",
SawPit=>"Holzfäller fällen und zerlegen Bäume. Nahe Ochsenkarren vergrößern die Transportladungen.",
Quarry=>"Gewinnt Stein aus einem endlichen Vorkommen.",
IronMine=>"Gewinnt Eisen aus einem endlichen Erzfeld.",
Fishpond=>"Fängt Fisch aus einem örtlichen Schwarm. Überfischte Bestände brauchen Zeit zur Erholung.",
HuntersPost=>"Jäger liefern Wildfleisch und verteidigen sich mit Pfeil und Bogen.",
OxTether=>"Stellt einen Ochsenkarren für den automatischen Rohstofftransport bereit.",
AppleFarm=>"Versorgt den Kornspeicher mit Äpfeln.",
DairyFarm=>"Produziert Käse als zusätzliche Nahrungsquelle.",
WoodenWall=>"Kleiner hölzerner Schutzwall mit begehbarem Wehrgang.",
StoneWallLow=>"Niedrige Steinmauer mit einem kurzen Aufstieg zum Wehrgang.",
StoneWallHigh=>"Hohe Steinmauer für einen erhöhten Wehrgang und größere Bogenreichweite.",
StoneWallCrenellated=>"Zinnen folgen der Mauerlinie und können Pfeile abfangen.",
StoneWall=>"Steinerner Wehrgang. Gleiche Höhen lassen sich miteinander verbinden.",
GatehouseSmall|GatehouseLarge|GatehouseWooden=>"Kontrollierbarer Durchgang durch die Befestigung. Bei Alarm vom Palast aus schließen.",
DrawbridgeGate=>"Befestigtes Tor mit beweglicher Zugbrücke über einen Fluss oder Graben.",
TowerWooden=>"Kompakter Holzturm mit begehbarem Dach.",
TowerLookout=>"Hoher, schmaler Aussichtsturm mit größerer Bogenreichweite.",
TowerGreat=>"Großer runder Bergfried mit Platz für Verteidiger auf dem Dach.",
TowerRoundShort|TowerRoundTall|TowerSquare|TowerStairwell=>"Begehbarer Verteidigungsturm. Benachbarte Türme verbinden sich; äußere Zinnen bleiben erhalten.",
Well=>"Verbessert die Wasserversorgung wachsender Siedlungen.",
Apothecary=>"Verbessert die Gesundheitsversorgung.",
Inn|GardenersHut=>"Erfüllt Freizeitbedürfnisse einer größeren Siedlung.",
_=>"Erweitert deine Siedlung. Baufläche und Rohstoffkosten werden vor dem Bau geprüft."}
}
