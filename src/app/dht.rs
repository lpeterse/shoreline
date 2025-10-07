use crate::mmdb::MMDB;
use eframe::egui;
use egui::*;
use egui_extras::{Column, TableBuilder};
use shoreline_dht::{DHT, InterfaceAddr};
use std::sync::Arc;

pub struct DhtApp {
    dht: Arc<DHT>,
    net: Option<InterfaceAddr>,
    mmdb: MMDB,
}

impl DhtApp {
    pub fn new(dht: Arc<DHT>, mmdb: MMDB) -> Self {
        Self { dht, mmdb, net: None }
    }
}

impl eframe::App for DhtApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dht.nodes().borrow().is_empty() {
            self.net = None;
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    let text = "No suitable interfaces/addresses found";
                    let text = RichText::new(text).color(Color32::LIGHT_GRAY).heading();
                    ui.add(Label::new(text));
                });
            });
            return;
        }

        TopBottomPanel::top("submenu").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                let nodes = self.dht.nodes().borrow();
                for net in nodes.keys() {
                    if self.net.is_none() {
                        self.net = Some(net.clone());
                    }
                    let selected = self.net.as_ref() == Some(net);
                    if ui.selectable_label(selected, net.to_string()).clicked() {
                        self.net = Some(net.clone());
                    }
                }
            });
            ui.add_space(1.0);
        });

        let frame = Frame::default().inner_margin(Margin::ZERO).fill(ctx.style().visuals.window_fill());

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            let height = ui.available_height();
            let center = Layout::centered_and_justified(Direction::LeftToRight);
            let right = Layout::right_to_left(Align::Center);

            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::auto().resizable(true).clip(true))
                .column(Column::auto().at_most(30.0))
                .column(Column::auto().at_most(20.0))
                .column(Column::auto().at_most(50.0))
                .column(Column::auto().at_most(50.0))
                .column(Column::auto().at_most(50.0))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder())
                .min_scrolled_height(0.0)
                .max_scroll_height(height);

            if let Some(node) = self.net.as_ref().map(|n| self.dht.nodes().borrow().get(&n).cloned()).flatten() {
                table
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.add_space(10.0);
                            ui.strong(format!("{} peers", node.peers().borrow().len()));
                        });
                        header.col(|ui| {
                            ui.with_layout(center, |ui| {
                                ui.strong("\u{2316}");
                            });
                        });
                        header.col(|ui| {
                            ui.with_layout(center, |ui| {
                                ui.strong("\u{1F30E}");
                            });
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
                            ui.strong("Version");
                        });
                        header.col(|ui| {
                            ui.strong("Address");
                        });
                        header.col(|ui| {
                            ui.strong("Error");
                        });
                    })
                    .body(|mut body| {
                        let paint_bg = |ui: &mut egui::Ui, bg: Color32| {
                            let item_spacing = ui.spacing().item_spacing;
                            let gapless_rect = ui.max_rect().expand2(0.5 * item_spacing);
                            ui.painter().rect_filled(gapless_rect, 0.0, bg);
                        };
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                // Black text on colored background
                                ui.style_mut().visuals.override_text_color = Some(Color32::GRAY.additive());
                                paint_bg(ui, Color32::PURPLE.gamma_multiply(0.5));
                                ui.add_space(10.0);
                                ui.label(RichText::new(node.id().to_string()).monospace());
                                ui.add_space(0.0);
                            });
                            row.col(|ui| {
                                ui.with_layout(center, |ui| {
                                    ui.label("160");
                                });
                            });
                            row.col(|ui| {
                                ui.with_layout(center, |ui| {
                                    ui.label(self.mmdb.lookup_iso(*node.addr().ip()).unwrap_or_default());
                                });
                            });
                            row.col(|ui| {
                                ui.with_layout(right, |ui| {
                                    ui.label("0 ms");
                                });
                            });
                            row.col(|ui| {
                                ui.with_layout(right, |ui| {
                                    ui.label(human_bytes::human_bytes(node.stat().tx_bytes as f64));
                                });
                            });
                            row.col(|ui| {
                                ui.with_layout(right, |ui| {
                                    ui.label(human_bytes::human_bytes(node.stat().rx_bytes as f64));
                                });
                            });
                            row.col(|ui| {
                                ui.add_space(5.0);
                                ui.label(node.version().to_string());
                            });
                            row.col(|ui| {
                                ui.label(node.addr().to_string());
                            });
                            row.col(|ui| {
                                ui.label(node.error().map(|e| e.to_string()).unwrap_or_default());
                            });
                        });

                        let ps = node.peers().borrow();
                        for peer in ps.iter().filter_map(|(_, w)| w.upgrade()) {
                            let rtt = peer.rtt().map(|x| x.as_secs_f32()).unwrap_or(5.0) * 1000.0;
                            let rtt = rtt.log10();
                            let rtt = (rtt / 3.0).min(1.0).max(0.0);
                            let bg = match peer.status() {
                                shoreline_dht::Status::Init => Color32::BLUE.gamma_multiply(0.5),
                                shoreline_dht::Status::Good => Color32::GREEN.gamma_multiply(1.0 - rtt),
                                shoreline_dht::Status::Miss => Color32::YELLOW.gamma_multiply(0.5),
                                shoreline_dht::Status::Fail => Color32::RED.gamma_multiply(0.5),
                            };
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    // Black text on colored background
                                    ui.style_mut().visuals.override_text_color = Some(Color32::GRAY.additive());
                                    paint_bg(ui, bg);
                                    ui.add_space(10.0);
                                    ui.label(RichText::new(peer.id().to_string()).monospace());
                                    ui.add_space(0.0);
                                });
                                row.col(|ui| {
                                    ui.with_layout(center, |ui| {
                                        ui.label(peer.id().similarity(node.id()).to_string());
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(center, |ui| {
                                        ui.label(self.mmdb.lookup_iso(*peer.address().ip()).unwrap_or_default());
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(
                                            peer.rtt().map(|x| format!("{} ms", x.as_millis())).unwrap_or_default(),
                                        );
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(human_bytes::human_bytes(peer.stat().borrow().tx_bytes as f64));
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(human_bytes::human_bytes(peer.stat().borrow().rx_bytes as f64));
                                    });
                                });
                                row.col(|ui| {
                                    ui.add_space(5.0);
                                    ui.label(peer.version().map(|v| v.to_string()).unwrap_or_default());
                                });
                                row.col(|ui| {
                                    ui.label(peer.address().to_string());
                                });
                                row.col(|ui| {
                                    ui.label(peer.error().map(|e| e.to_string()).unwrap_or_default());
                                });
                            });
                        }
                    });
            }
        });
    }
}
