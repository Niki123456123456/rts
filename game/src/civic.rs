//! Palace management, marketplace and consensual requests between active players.
use crate::*;

use stronghold_map::civic;

pub(crate) fn ui(
    mut contexts: EguiContexts,
    mut net: ResMut<Multiplayer>,
    mut selection: ResMut<BuildingInteraction>,
    mut diplomacy_open: Local<bool>,
    mut quantity: Local<u32>,
    mut resource: Local<u8>,
    mut exchange_resource: Local<u8>,
    mut target: Local<u32>,
    mut offer: Local<u64>,
    mut note: Local<String>,
) -> Result {
    if net.snapshot.game_id.is_none() {
        return Ok(());
    }

    let ctx = contexts.ctx_mut()?;

    egui::Area::new(egui::Id::new("treasury_hud"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10., 44.))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(215))
                .inner_margin(6.)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (coin,_) = ui.allocate_exact_size(egui::vec2(24.,24.),egui::Sense::hover());
                        ui.painter().circle_filled(coin.center(),10.,egui::Color32::from_rgb(196,145,36));
                        ui.painter().circle_stroke(coin.center(),8.,egui::Stroke::new(1.3,egui::Color32::from_rgb(255,224,128)));
                        ui.painter().text(coin.center(),egui::Align2::CENTER_CENTER,"S",egui::FontId::proportional(15.),egui::Color32::from_rgb(70,46,12));
                        ui.colored_label(
                            egui::Color32::GOLD,
                            format!("{} Sesterzen", net.snapshot.gold.unwrap_or(0)),
                        );

                        let policy=net.snapshot.buildings.iter().find(|b|b.owned&&b.kind=="keep").and_then(|b| net.snapshot.policies.iter().find(|p|p.building_id==b.id));
                        let residents=policy.and_then(|p|net.snapshot.keep_population.iter().find(|k|k.building_id==p.building_id)).map_or(0,|p|p.arrived);
                        let balance=civic::net_per_round(residents,policy.map_or(3,|p|p.taxes));
                        ui.colored_label(if balance>=0 {egui::Color32::LIGHT_GREEN}else{egui::Color32::LIGHT_RED},format!("{balance:+} / Runde")).on_hover_text("Netto je 30 Sekunden: Steuereinnahmen abzüglich laufender Ausgaben. Aktuell fallen keine Grundausgaben an.");
                        let pending = net
                            .snapshot
                            .requests
                            .iter()
                            .filter(|r| r.incoming && r.status == 0)
                            .count();

                        if ui
                            .button(format!("Mitspieler · {pending} Anfragen"))
                            .clicked()
                        {
                            *diplomacy_open = !*diplomacy_open;
                        }
                    });
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

    if let Some(b) = building.filter(|b| b.kind == "keep" || b.kind == "market") {
        let mut open = true;

        if b.kind == "keep" {
            let content = |ui: &mut egui::Ui| {
                let policy = net.snapshot.policies.iter().find(|p| p.building_id == b.id);
                let mut rations = policy.map_or(4, |p| p.rations);
                let mut taxes = policy.map_or(3, |p| p.taxes);
                let alarm = policy.is_some_and(|p| p.alarm);
                let pop = net
                    .snapshot
                    .keep_population
                    .iter()
                    .find(|p| p.building_id == b.id)
                    .map_or(0, |p| p.arrived);

                ui.heading("Sicherheit");
                ui.horizontal(|ui| {
                    if ui
                        .button(if alarm {
                            "Entwarnung geben"
                        } else {
                            "Alarm blasen · Bewohner in den Palast"
                        })
                        .clicked()
                    {
                        net.send(Action::PalaceAlarm { active: !alarm });
                    }
                    if ui.button("Alle Tore schließen").clicked() {
                        net.send(Action::CloseAllGates);
                    }
                });

                if alarm {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "Alarm aktiv · Produktion pausiert",
                    );
                    ui.small("Bewohner laufen über die Treppen in den Palast. Danach schließen die Tore; belegte Durchgänge warten, bis sie frei sind.");
                } else {
                    ui.small(
                        "Ein kräftiger Hornruf nach Art eines römischen Cornu warnt die Siedlung.",
                    );
                }

                ui.separator();
                ui.heading("Lebensmittelrationen");
                let ration_name = civic::RATION_LABELS[rations as usize];
                let ration_changed = ui
                    .add(
                        egui::Slider::new(&mut rations, 0..=7)
                            .integer()
                            .show_value(false)
                            .text(ration_name),
                    )
                    .changed();
                ui.label(format!(
                    "Verbrauch: {} % · etwa {:.2} Waren je 30 Sekunden",
                    civic::RATION_QUARTERS[rations as usize] * 25,
                    pop as f32 * civic::RATION_QUARTERS[rations as usize] as f32 / 16.
                ));
                ui.label(format!(
                    "Zufriedenheit: {:+} Punkte",
                    civic::RATION_HAPPINESS[rations as usize]
                ));
                ui.small(
                    "Großzügige Rationen geben ihren Bonus nur bei vorhandenen Lebensmitteln.",
                );

                ui.separator();
                ui.heading("Steuern");
                let tax_name = civic::TAX_LABELS[taxes as usize];
                let tax_changed = ui
                    .add(
                        egui::Slider::new(&mut taxes, 0..=7)
                            .integer()
                            .show_value(false)
                            .text(tax_name),
                    )
                    .changed();
                ui.label(format!(
                    "{} Sesterzen pro Bewohner · {} je 30 Sekunden",
                    civic::TAX_RATES[taxes as usize],
                    civic::TAX_RATES[taxes as usize] * u64::from(pop)
                ));
                ui.label(format!(
                    "Zufriedenheit: {:+} Punkte",
                    civic::TAX_HAPPINESS[taxes as usize]
                ));

                if ration_changed || tax_changed {
                    net.send(Action::SetCivicPolicy { rations, taxes });
                }

                if let Some(e) = &net.snapshot.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
            };
            egui::Window::new("Palastverwaltung")
                .open(&mut open)
                .default_width(440.)
                .default_pos(egui::pos2(18., 110.))
                .resizable(false)
                .show(ctx, content);
        } else {
            let content = |ui: &mut egui::Ui| {
                goods_picker(ui, &mut resource);
                *quantity = (*quantity).clamp(1, 200);
                ui.add(egui::Slider::new(&mut *quantity, 1..=200).text("Menge"));
                let p = net
                    .snapshot
                    .prices
                    .iter()
                    .find(|p| p.resource == *resource)
                    .map_or(0, |p| p.pressure);
                let buy = civic::quote(*resource, p, *quantity, true);
                let sell = civic::quote(*resource, p, *quantity, false);
                ui.label(format!(
                    "Bestand: {} · Schatzkammer: {} Sesterzen",
                    owned_amount(&net, *resource),
                    net.snapshot.gold.unwrap_or(0)
                ));
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            net.snapshot.gold.unwrap_or(0) >= buy,
                            egui::Button::new(format!("Kaufen · {buy} Sesterzen")),
                        )
                        .clicked()
                    {
                        net.send(Action::MarketTrade {
                            resource: *resource,
                            amount: *quantity,
                            buy: true,
                        });
                    }
                    if ui
                        .add_enabled(
                            owned_amount(&net, *resource) >= *quantity,
                            egui::Button::new(format!("Verkaufen · {sell} Sesterzen")),
                        )
                        .clicked()
                    {
                        net.send(Action::MarketTrade {
                            resource: *resource,
                            amount: *quantity,
                            buy: false,
                        });
                    }
                });
                ui.separator();
                ui.label("Waren direkt tauschen gegen:");
                ui.push_id("barter_target", |ui| {
                    goods_picker(ui, &mut exchange_resource)
                });
                let target_pressure = net
                    .snapshot
                    .prices
                    .iter()
                    .find(|p| p.resource == *exchange_resource)
                    .map_or(0, |p| p.pressure);
                let (received, change) =
                    civic::barter(*resource, p, *quantity, *exchange_resource, target_pressure);
                if ui
                    .add_enabled(
                        received > 0 && owned_amount(&net, *resource) >= *quantity,
                        egui::Button::new(format!(
                            "{} {} → {} {} (+{} Sesterzen)",
                            *quantity,
                            civic::GOODS[*resource as usize],
                            received,
                            civic::GOODS[*exchange_resource as usize],
                            change
                        )),
                    )
                    .clicked()
                {
                    net.send(Action::MarketExchange {
                        sell: *resource,
                        amount: *quantity,
                        buy: *exchange_resource,
                    });
                }

                ui.small("Angebot und Nachfrage im gemeinsamen Spiel verändern den Kurs. Die angezeigten Preise gelten für die gesamte Menge einschließlich Handelsspanne.");
                if let Some(e) = &net.snapshot.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
            };
            egui::Window::new("Römischer Marktplatz")
                .open(&mut open)
                .default_width(440.)
                .default_pos(egui::pos2(18., 110.))
                .resizable(false)
                .show(ctx, content);
        }

        if !open {
            selection.selected = None;
        }
    }

    if *diplomacy_open {
        let content = |ui: &mut egui::Ui| {
            let players = net
                .snapshot
                .players
                .iter()
                .filter(|p| !p.owned)
                .cloned()
                .collect::<Vec<_>>();
            if players.is_empty() {
                ui.label("Noch kein anderer Spieler in dieser Welt.");
            } else {
                if !players.iter().any(|p| p.color == *target) {
                    *target = players[0].color;
                }

                egui::ComboBox::from_id_salt("request_target")
                    .selected_text(format!("Spieler {}", *target + 1))
                    .show_ui(ui, |ui| {
                        for p in &players {
                            ui.selectable_value(
                                &mut *target,
                                p.color,
                                format!("Spieler {}", p.color + 1),
                            );
                        }
                    });
                goods_picker(ui, &mut resource);
                *quantity = (*quantity).clamp(1, 1000);
                ui.add(egui::Slider::new(&mut *quantity, 1..=1000).text("Erbetene Waren"));
                ui.horizontal(|ui| {
                    ui.label("Angebotene Sesterzen:");
                    ui.add(egui::DragValue::new(&mut *offer).range(0..=1_000_000));
                });
                ui.add(
                    egui::TextEdit::singleline(&mut *note)
                        .char_limit(200)
                        .hint_text("Nachricht an den Mitspieler"),
                );
                if ui.button("Anfrage stellen").clicked() {
                    net.send(Action::RequestGoods {
                        target_color: *target,
                        resource: *resource,
                        amount: *quantity,
                        offered_gold: *offer,
                        note: note.clone(),
                    });
                }
                ui.small("Der Mitspieler entscheidet selbst. Waren und Geld wechseln erst bei Annahme den Besitzer.");
            }

            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(280.)
                .show(ui, |ui| {
                    for r in net.snapshot.requests.clone().into_iter().rev().take(40) {
                        ui.group(|ui| {
                            ui.label(format!(
                                "Spieler {} → Spieler {}: {} {} gegen {} Sesterzen",
                                r.from_color + 1,
                                r.to_color + 1,
                                r.amount,
                                civic::GOODS[r.resource.min(7) as usize],
                                r.offered_gold
                            ));
                            if !r.note.is_empty() {
                                ui.label(&r.note);
                            }
                            if r.status == 0 && r.incoming {
                                ui.horizontal(|ui| {
                                    if ui.button("Annehmen").clicked() {
                                        net.send(Action::RespondRequest {
                                            request_id: r.id,
                                            accept: true,
                                        });
                                    }
                                    if ui.button("Ablehnen").clicked() {
                                        net.send(Action::RespondRequest {
                                            request_id: r.id,
                                            accept: false,
                                        });
                                    }
                                });
                            } else {
                                ui.small(match r.status {
                                    1 => "Angenommen und übertragen",
                                    2 => "Abgelehnt",
                                    _ => {
                                        if r.outgoing {
                                            "Deine Anfrage ist offen"
                                        } else {
                                            "Anfrage ist offen"
                                        }
                                    }
                                });
                            }
                        });
                    }
                });
            if let Some(e) = &net.snapshot.error {
                ui.colored_label(egui::Color32::LIGHT_RED, e);
            }
        };
        egui::Window::new("Anfragen an Mitspieler")
            .open(&mut diplomacy_open)
            .default_width(460.)
            .resizable(true)
            .show(ctx, content);
    }

    Ok(())
}

fn goods_picker(ui: &mut egui::Ui, resource: &mut u8) {
    *resource = (*resource).min(7);

    egui::ComboBox::from_id_salt("trade_resource")
        .selected_text(civic::GOODS[*resource as usize])
        .show_ui(ui, |ui| {
            for (i, name) in civic::GOODS.iter().enumerate() {
                ui.selectable_value(resource, i as u8, *name);
            }
        });
}

fn owned_amount(net: &Multiplayer, r: u8) -> u32 {
    if r == 7 {
        return net
            .snapshot
            .fish_stocks
            .iter()
            .filter(|f| {
                net.snapshot
                    .buildings
                    .iter()
                    .any(|b| b.id == f.building_id && b.owned)
            })
            .map(|f| f.amount)
            .sum();
    }

    net.snapshot.resources.map_or(0, |s| match r {
        0 => s.wood,
        1 => s.stone,
        2 => s.iron,
        3 => s.apples,
        4 => s.bread,
        5 => s.cheese,
        6 => s.meat,
        _ => 0,
    })
}
