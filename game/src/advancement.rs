//! Research, equipment storage and the new Roman services.
use crate::*;
use stronghold_map::{advancement as tech, units::UnitKind};
pub(crate) fn ui(
    mut contexts: EguiContexts,
    mut net: ResMut<Multiplayer>,
    selection: Res<BuildingInteraction>,
    atlas: Res<resources::ResourceAtlas>,
    selected: Res<SelectedUnits>,
    units: Query<&Unit>,
) -> Result {
    if net.snapshot.game_id.is_none() {
        return Ok(());
    }
    let snapshot = net.snapshot.clone();
    let texture = atlas.texture(&mut contexts);
    let ctx = contexts.ctx_mut()?;
    let own = |id| snapshot.buildings.iter().any(|b| b.id == id && b.owned);
    let research = snapshot.research.iter().find(|r| own(r.keep_id));
    if let Some(b) = selection
        .selected
        .and_then(|id| snapshot.buildings.iter().find(|b| b.id == id && b.owned))
        .filter(|b| {
            matches!(
                b.kind.as_str(),
                "research_center"
                    | "arsenal"
                    | "weapon_smith"
                    | "tool_smith"
                    | "hospital"
                    | "baths"
                    | "temple"
                    | "aqueduct"
                    | "colosseum"
                    | "theater"
                    | "library"
                    | "statue"
                    | "stockpile"
                    | "market"
                    | "harbor"
            )
        })
    {
        let label = stronghold_map::BuildingKind::from_name(&b.kind)
            .unwrap()
            .label();
        egui::Window::new(label).id(egui::Id::new("roman_details")).anchor(egui::Align2::RIGHT_TOP,egui::vec2(-12.,96.)).default_width(300.).max_height((ctx.viewport_rect().height()-220.).max(150.)).vscroll(true).show(ctx,|ui|{
 match b.kind.as_str(){
 "research_center"=>{ui.label("Ein automatisch zugewiesener Gelehrter erforscht Waffen, Wirtschaft und Politiken. Forschung bleibt auch nach Abriss erhalten.");
 if let Some(o)=snapshot.research_orders.iter().find(|o|o.building_id==b.id){ui.strong(tech::research_label(&o.branch,o.level));ui.add(egui::ProgressBar::new(o.progress as f32/o.duration as f32).text(format!("{} / {} Sekunden",o.progress,o.duration)));}else{for branch in tech::BRANCHES{let level=research.map_or(0,|r|match branch{"weapons"=>r.weapons,"economy"=>r.economy,_=>r.politics});ui.separator();ui.strong(format!("{} · {level}/3",tech::branch_label(branch)));ui.small(tech::research_effect(branch));if level<3{ui.label(tech::research_label(branch,level+1));resources::cost_icons(ui,texture,tech::research_cost(level+1));ui.label(format!("{} Sesterzen · {} Sekunden",tech::research_gold(level+1),tech::research_seconds(level+1)));if ui.button("Erforschen").clicked(){net.send(Action::StartResearch{building_id:b.id,branch:branch.into()});}}}}
 }
 "arsenal"=>{let stock=snapshot.arsenals.iter().find(|s|s.building_id==b.id);ui.label("Waffenschmiede liefern automatisch hierher. Die Kaserne verwendet eingelagerte Ausrüstung und zieht deren Rohstoffe von den Rekrutierungskosten ab.");for (label,n)in [("Bögen",stock.map_or(0,|s|s.bows)),("Schwerter",stock.map_or(0,|s|s.swords)),("Schilde",stock.map_or(0,|s|s.shields))]{ui.label(format!("{label}: {n} / 40"));}ui.small("Fehlende Waffen können in der Kaserne weiterhin direkt aus Rohstoffen hergestellt werden.");}
 "weapon_smith"=>{ui.label("Automatische Waffenherstellung · benötigt einen Bauern und ein geöffnetes Waffenlager.");let recipe=snapshot.workshop_recipes.iter().find(|r|r.building_id==b.id).map_or("weapon_sword",|r|r.item.as_str());for item in tech::Equipment::ALL{if ui.selectable_label(recipe==item.name(),item.label()).clicked(){net.send(Action::SetWorkshopRecipe{building_id:b.id,item:item.name().into()});}resources::cost_icons(ui,texture,item.cost());}ui.small("Eine Waffe je 12 Arbeitssekunden; pausiert bei fehlenden Rohstoffen oder vollem Waffenlager.");}
 "tool_smith"=>{ui.label("Stellt automatisch Werkzeuge her: je 1 Holz + 1 Eisen, 12 Arbeitssekunden.");resources::cost_icons(ui,texture,stronghold_map::Cost::new(1,0,1));ui.small("Werkzeuge werden im Rohstofflager aufbewahrt. Sie beschleunigen Betriebe um bis zu 20 % und werden bei der Arbeit verschlissen.");}
 "hospital"=>{ui.label("Ein Sanitäter heilt nahe eigene Soldaten und Bewohner im Umkreis von 16 Metern. Während eines Nahkampfs wird die Heilung pausiert.");ui.small("+0,035 Gesundheit je Sekunde · verbessert die Gesundheitsversorgung.");}
 "baths"=>{ui.label("Öffentliche Thermen verbessern die Hygiene und Gesundheitsversorgung. +4 Zufriedenheit; der Bonus zählt einmal pro Siedlung.");}
 "temple"=>{ui.label("Ein römischer Tempel bietet religiöses Leben und Freizeit. +5 Zufriedenheit; zählt einmal pro Siedlung.");}
 "aqueduct"=>{ui.label("Das Aquädukt erfüllt die Wasserversorgung der Siedlung. Die Bögen bleiben für Einheiten durchgängig.");}
 "colosseum"=>{ui.label("Das Amphitheater bietet Unterhaltung für eine große Siedlung. +7 Zufriedenheit; zählt einmal.");}
 "theater"=>{ui.label("Römisches Theater: erfüllt Freizeitbedürfnisse wachsender Siedlungen.");}
 "library"=>{ui.label("Die Bibliotheca beschleunigt Forschung um 25 %. Mehrere Bibliotheken verstärken den Bonus nicht.");}
 "statue"=>{ui.label("Triumphstatue · +3 Zufriedenheit. Der Bonus zählt einmal pro Siedlung.");}
 _=>{if matches!(b.kind.as_str(),"market"|"harbor"){ui.label("Zusätzliche Handelswaren aus deinen Lagern");}else{ui.label("Werkzeuge und römische Handelsgüter");}
 let goods=snapshot.workshop_stocks.iter().filter(|s|own(s.building_id)).collect::<Vec<_>>();for (name,id,count)in [("Werkzeuge",11,goods.iter().map(|s|s.tools).sum::<u32>()),("Trauben",12,goods.iter().map(|s|s.grapes).sum()),("Oliven",13,goods.iter().map(|s|s.olives).sum()),("Wein",14,goods.iter().map(|s|s.wine).sum()),("Olivenöl",15,goods.iter().map(|s|s.oil).sum())]{ui.horizontal(|ui|{ui.label(format!("{name}: {count}"));if ui.add_enabled(count>0,egui::Button::new("1 verkaufen")).on_hover_text("Benötigt einen geöffneten Markt oder Hafen. Angebot und Nachfrage beeinflussen den Erlös.").clicked(){net.send(Action::SellWorkshopGoods{resource:id,amount:1});}});}}
 }
 if let Some(e)=&snapshot.error{ui.colored_label(egui::Color32::LIGHT_RED,e);}
 });
    }
    let infantry = selected
        .0
        .iter()
        .filter_map(|e| units.get(*e).ok())
        .filter(|u| {
            u.owned
                && !UnitKind::from_name(&u.kind)
                    .is_some_and(|k| k.is_siege() || k == UnitKind::LightCavalry)
        })
        .map(|u| u.id)
        .collect::<Vec<_>>();
    if !infantry.is_empty() && snapshot.siege_access.iter().any(|a| a.owned) {
        egui::Window::new("Mauern erstürmen")
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(12., 126.))
            .default_width(220.)
            .show(ctx, |ui| {
                ui.label("Eigene angelegte Leiter oder Belagerungsturm nutzen:");
                for a in snapshot.siege_access.iter().filter(|a| a.owned) {
                    let label = snapshot
                        .units
                        .iter()
                        .find(|u| u.id == a.unit_id)
                        .and_then(|u| UnitKind::from_name(&u.kind))
                        .map_or("Aufstieg", |k| k.label());
                    if ui.button(format!("{label} · Wehrgang erstürmen")).clicked() {
                        net.send(Action::AssaultRoof {
                            unit_ids: infantry.clone(),
                            access_id: a.unit_id,
                        });
                    }
                }
            });
    }
    Ok(())
}
