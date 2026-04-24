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
        let interfaces = {
            let mut map = std::collections::HashMap::new();
            for interface in ctrl.interfaces() {
                map.insert(interface.index, interface);
            }
            map
        };

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
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
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
                    ui.strong("Interface");
                });
                header.col(|ui| {
                    ui.strong("Local");
                });
                header.col(|_| {});
                header.col(|ui| {
                    ui.strong("Remote");
                });
                header.col(|_| {});
                header.col(|ui| {
                    ui.with_layout(right, |ui| {
                        ui.strong("\u{1F501}");
                    });
                });
                header.col(|ui| {
                    ui.with_layout(right, |ui| {
                        ui.strong("\u{26A1}");
                    });
                });
                header.col(|ui| {
                    ui.strong("Error");
                });
            })
            .body(|mut body| {
                let bg = Color32::DARK_GRAY.gamma_multiply(0.3);
                let dimmed = Color32::DARK_GRAY.gamma_multiply(0.5).additive();

                for peer in &peers {
                    let paths = { peer.paths.stats().borrow().paths.clone() };
                    if paths.is_empty() {
                        body.row(Self::HEIGHT_ROW, |mut row| {
                            row.col(|ui| {
                                ui.add_space(10.0);
                                ui.strong(&peer.config.name);
                            });
                            row.col(|ui| {
                                ui.colored_label(dimmed, "\u{2014}");
                            });
                            for _ in 0..2 {
                                row.col(|ui| {
                                    ui.colored_label(dimmed, "\u{2014}");
                                });
                                row.col(|_| {});
                            }
                            for _ in 0..2 {
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.colored_label(dimmed, "\u{2014}");
                                    });
                                });
                            }
                            row.col(|ui| {
                                ui.label("No known addresses");
                            });
                        });
                    }

                    for (i, (addr, stats)) in paths.iter().enumerate() {
                        let stats = { stats.borrow().clone() };
                        body.row(Self::HEIGHT_ROW, |mut row| {
                            row.col(|ui| {
                                ui.add_space(10.0);
                                if i == 0 {
                                    ui.strong(&peer.config.name);
                                } else {
                                    ui.colored_label(dimmed, &peer.config.name);
                                }
                            });
                            row.col(|ui| {
                                match interfaces.get(&addr.local.scope_id()) {
                                    Some(interface) => ui.label(&interface.name),
                                    None => ui.label(addr.local.scope_id().to_string()),
                                };
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", addr.local.ip()));
                            });
                            row.col(|ui| {
                                ui.label(format!(":{}", addr.local.port()));
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", addr.remote.ip()));
                            });
                            row.col(|ui| {
                                ui.label(format!(":{}", addr.remote.port()));
                            });
                            row.col(|ui| {
                                ui.with_layout(right, |ui| {
                                    if stats.rtt.as_millis() > 0 {
                                        ui.label(format!("{} ms", stats.rtt.as_millis()));
                                    } else if stats.rtt.as_micros() > 0 {
                                        ui.label(format!("{} µs", stats.rtt.as_micros()));
                                    } else {
                                        ui.colored_label(dimmed, "\u{2014}");
                                    }
                                });
                            });
                            row.col(|ui| {
                                ui.with_layout(right, |ui| {
                                    if stats.jitter.as_millis() > 0 {
                                        ui.label(format!("{} ms", stats.jitter.as_millis()));
                                    } else if stats.jitter.as_micros() > 0 {
                                        ui.label(format!("{} µs", stats.jitter.as_micros()));
                                    } else {
                                        ui.colored_label(dimmed, "\u{2014}");
                                    }
                                });
                            });
                            row.col(|ui| {
                                match stats.error {
                                    Some(ref e) => ui.label(e),
                                    None => ui.colored_label(dimmed, "\u{2014}"),
                                };
                            });
                        });
                    }
                }
            });
    }
}
