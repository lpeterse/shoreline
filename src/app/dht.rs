use crate::mmdb::MMDB;
use eframe::egui;
use egui::*;
use egui_extras::{Column, TableBuilder};
use shoreline_dht::{DHT};
use std::sync::Arc;

pub struct DhtApp {
    dht: Arc<DHT>,
    interface: Option<String>,
    mmdb: MMDB,
}

impl DhtApp {
    pub fn new(dht: Arc<DHT>, mmdb: MMDB) -> Self {
        Self { dht, mmdb, interface: None }
    }
}

impl eframe::App for DhtApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dht.nodes().is_empty() {
            self.interface = None;
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
                // Any interface
                let count = self.dht.peers().values().count();
                let label = format!("All ({})", count);
                if ui.selectable_label(self.interface.is_none(), label).clicked() {
                    self.interface = None;
                }
                for interface in self.dht.nodes().keys() {
                    let count = self.dht.peers().values().map(|p| p.links().values().filter(|l| l.node().name() == interface).count()).sum::<usize>();
                    let label = format!("{} ({})", interface, count);
                    let selected = self.interface.as_ref() == Some(interface);
                    if ui.selectable_label(selected, label).clicked() {
                        self.interface = Some(interface.clone());
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
                .column(Column::auto())
                .column(Column::remainder())
                .min_scrolled_height(0.0)
                .max_scroll_height(height);

            let mut peers = { self.dht.peers().values().cloned().collect::<Vec<_>>() };
            peers.sort_by(|a, b| b.id().similarity(self.dht.id()).cmp(&a.id().similarity(self.dht.id())));

            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.add_space(10.0);
                        ui.strong("Peer ID");
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
                        ui.strong("Interface");
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
                    for (i, node) in self.dht.nodes().values().filter(|n| self.interface.is_none() || self.interface.as_deref() == Some(n.name())).enumerate() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                // Black text on colored background
                                ui.add_space(10.0);
                                if i == 0 {
                                    ui.style_mut().visuals.override_text_color = Some(Color32::GRAY.additive());
                                } else {
                                    ui.style_mut().visuals.override_text_color = Some(Color32::DARK_GRAY.gamma_multiply(0.5).additive());
                                }
                                paint_bg(ui, Color32::PURPLE.gamma_multiply(0.5));
                                ui.label(RichText::new(self.dht.id().to_string()).monospace());
                                ui.add_space(0.0);
                            });
                            row.col(|ui| {
                                ui.with_layout(center, |ui| {
                                    ui.label("0");
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
                                ui.label(node.name());
                            });
                            row.col(|ui| {
                                ui.label(node.addr().to_string());
                            });
                            row.col(|ui| {
                                ui.label(node.stat().error.map(|e| e.to_string()).unwrap_or_default());
                            });
                        });
                    }

                    for peer in peers {
                        for (i, conn) in peer.links().values().filter(|c| self.interface.is_none() || self.interface.as_deref() == Some(c.node().name())).enumerate() {
                            let stat = { conn.stat().borrow().clone() };
                            let rtt = stat.rtt.map(|x| x.as_secs_f32()).unwrap_or(5.0) * 1000.0;
                            let rtt = rtt.log10();
                            let rtt = (rtt / 3.0).min(1.0).max(0.0);
                            let bg = match stat.status {
                                shoreline_dht::Status::Init => Color32::BLUE.gamma_multiply(0.5),
                                shoreline_dht::Status::Good => Color32::GREEN.gamma_multiply(1.0 - rtt),
                                shoreline_dht::Status::Fail => {
                                    let t = stat.rx_last.elapsed().as_secs_f32();
                                    let t = 1.0 - (t / 300.0).min(1.0);
                                    let bg = Color32::RED;
                                    let fg = Color32::YELLOW.gamma_multiply(t);
                                    bg.blend(fg).gamma_multiply(0.5)
                                }
                                shoreline_dht::Status::Term => Color32::RED.gamma_multiply(0.5),
                            };
                            body.row(18.0, |mut row| {
                                row.col(|ui| {
                                    // Black text on colored background
                                    ui.add_space(10.0);
                                    if i == 0 {
                                        ui.style_mut().visuals.override_text_color = Some(Color32::GRAY.additive());
                                    } else {
                                        ui.style_mut().visuals.override_text_color = Some(Color32::DARK_GRAY.gamma_multiply(0.5).additive());
                                    }
                                    paint_bg(ui, bg);
                                    ui.label(RichText::new(conn.peer().id().to_string()).monospace());
                                    ui.add_space(0.0);
                                });
                                row.col(|ui| {
                                    ui.with_layout(center, |ui| {
                                        ui.label(conn.peer().id().distance(self.dht.id()).to_string());
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(center, |ui| {
                                        ui.label(self.mmdb.lookup_iso(*conn.addr().ip()).unwrap_or_default());
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(
                                            if matches!(stat.status, shoreline_dht::Status::Fail | shoreline_dht::Status::Init) {
                                                stat.rx_last.elapsed().as_secs().to_string() + " s"
                                            } else {
                                            stat.rtt.map(|x| format!("{} ms", x.as_millis())).unwrap_or_default() 
                                            }
                                        );
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(human_bytes::human_bytes(conn.stat().borrow().tx_bytes as f64));
                                    });
                                });
                                row.col(|ui| {
                                    ui.with_layout(right, |ui| {
                                        ui.label(human_bytes::human_bytes(conn.stat().borrow().rx_bytes as f64));
                                    });
                                });
                                row.col(|ui| {
                                    ui.add_space(5.0);
                                    ui.label(stat.version.map(|v| v.to_string()).unwrap_or_default());
                                });
                                row.col(|ui| {
                                    ui.label(conn.node().name());
                                });
                                row.col(|ui| {
                                    ui.label(conn.addr().to_string());
                                });
                                row.col(|ui| {
                                    ui.label(stat.error.map(|e| e.to_string()).unwrap_or_default());
                                });
                            });
                        }
                    }
                });
        });
    }
}
