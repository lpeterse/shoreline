use crate::mdns::MdnsCtrl;
use eframe::egui;
use egui::*;
use egui_extras::{Column, TableBuilder};

pub struct MdnsView;

impl MdnsView {
    pub const HEIGHT_ROW: f32 = 18.0;
    pub const HEIGHT_HEADER: f32 = 20.0;

    pub fn new() -> Self {
        Self
    }

    pub fn show_top(&self, ui: &mut egui::Ui, ctrl: &MdnsCtrl) {
        ui.add_space(3.0);
        let count = ctrl.entries().len();
        ui.label(format!("{} peers", count));
        ui.add_space(1.0);
    }

    pub fn show_central(&self, ui: &mut egui::Ui, ctrl: &MdnsCtrl) {
        let entries = ctrl.entries();

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
            .column(Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .header(Self::HEIGHT_HEADER, |mut header| {
                header.col(|ui| {
                    ui.add_space(10.0);
                    ui.strong("Name");
                });
                header.col(|ui| {
                    ui.strong("Pubkey");
                });
                header.col(|ui| {
                    ui.strong("Port");
                });
                header.col(|ui| {
                    ui.strong("Addresses");
                });
            })
            .body(|mut body| {
                let bg = Color32::DARK_GRAY.gamma_multiply(0.3);
                for entry in &entries {
                    let dimmed = Color32::DARK_GRAY.gamma_multiply(0.5).additive();
                    body.row(Self::HEIGHT_ROW, |mut row| {
                        row.col(|ui| {
                            paint_bg(ui, bg);
                            ui.add_space(10.0);
                            ui.strong(&entry.displayname);
                        });
                        row.col(|ui| {
                            paint_bg(ui, bg);
                            ui.label(format!("{}", entry.pubkey));
                        });
                        row.col(|ui| {
                            paint_bg(ui, bg);
                            ui.label(format!("{}", entry.port));
                        });
                        row.col(|ui| {
                            paint_bg(ui, bg);
                            ui.vertical(|ui| {
                                for addr in &entry.addrs {
                                    ui.label(format!("{} ({})", addr.addr(), addr.scope_id().name));
                                }
                            });
                        });
                    });
                }
            });
    }
}
