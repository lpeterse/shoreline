use crate::peers::PeersCtrl;
use eframe::egui;
use egui::*;
use egui_extras::{Column, TableBuilder};

pub struct PeersView;

impl PeersView {
    pub const HEIGHT_ROW: f32 = 18.0;
    pub const HEIGHT_HEADER: f32 = 20.0;

    pub fn new() -> Self {
        Self
    }

    pub fn show_top(&self, ui: &mut egui::Ui, ctrl: &PeersCtrl) {
        ui.add_space(3.0);
        let count = ctrl.peers().len();
        ui.label(format!("{} peers", count));
        ui.add_space(1.0);
    }

    pub fn show_central(&self, ui: &mut egui::Ui, ctrl: &PeersCtrl) {
        let peers = ctrl.peers();

        let height = ui.available_height();
        let right = Layout::right_to_left(Align::Center);
        let paint_bg = |ui: &mut egui::Ui, bg: Color32| {
            let item_spacing = ui.spacing().item_spacing;
            let gapless_rect = ui.max_rect().expand2(0.5 * item_spacing);
            ui.painter().rect_filled(gapless_rect, 0.0, bg);
        };

        TableBuilder::new(ui)
            .striped(false)
            .resizable(false)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::auto().resizable(true).clip(true))
            .column(Column::auto().clip(true))
            .column(Column::auto())
            .column(Column::auto().at_most(80.0))
            .column(Column::auto().at_most(60.0))
            .column(Column::auto().at_most(60.0))
            .column(Column::auto().at_most(60.0))
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .header(Self::HEIGHT_HEADER, |mut header| {
                header.col(|ui| {
                    ui.add_space(10.0);
                    ui.strong("Peer");
                });
                header.col(|ui| {
                    ui.strong("Public Key");
                });
                header.col(|ui| {
                    ui.strong("Address");
                });
                header.col(|ui| {
                    ui.strong("Status");
                });
                header.col(|ui| {
                    ui.with_layout(right, |ui| {
                        ui.strong("\u{23F1}");
                    });
                });
                header.col(|ui| {
                    ui.with_layout(right, |ui| {
                        ui.strong("\u{2b06}");
                    });
                });
                header.col(|ui| {
                    ui.with_layout(right, |ui| {
                        ui.strong("\u{2b07}");
                    });
                });
                header.col(|ui| {
                    ui.strong("Error");
                });
            })
            .body(|mut body| {
                let bg = Color32::DARK_GRAY.gamma_multiply(0.3);
                for peer in &peers {
                    for (i, addr) in peer.addresses.iter().enumerate() {
                        let dimmed = Color32::DARK_GRAY.gamma_multiply(0.5).additive();
                        body.row(Self::HEIGHT_ROW, |mut row| {
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.add_space(10.0);
                                if i == 0 {
                                    ui.strong(&peer.name);
                                } else {
                                    ui.colored_label(dimmed, &peer.name);
                                }
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                let pubkey_text = RichText::new(peer.pubkey.to_string()).monospace();
                                if i == 0 {
                                    ui.label(pubkey_text);
                                } else {
                                    ui.label(pubkey_text.color(dimmed));
                                }
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.label(format!("{}", addr));
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.colored_label(Color32::GRAY, "\u{2014}");
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.with_layout(right, |ui| {
                                    ui.colored_label(Color32::GRAY, "\u{2014}");
                                });
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.with_layout(right, |ui| {
                                    ui.colored_label(Color32::GRAY, "\u{2014}");
                                });
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                                ui.with_layout(right, |ui| {
                                    ui.colored_label(Color32::GRAY, "\u{2014}");
                                });
                            });
                            row.col(|ui| {
                                paint_bg(ui, bg);
                            });
                        });
                    }
                }
            });
    }
}
