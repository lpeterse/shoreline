use crate::identity::IdentityProvider;
use eframe::egui;
use egui::*;
use egui_extras::{Column, TableBuilder};

pub struct IdentityApp {
    provider: IdentityProvider,
}

impl IdentityApp {
    pub fn new() -> Self {
        Self { provider: IdentityProvider::new() }
    }
}

impl eframe::App for IdentityApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(_identity) = self.provider.get() {
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(Vec2::new(670.0, 400.0));
                        ui.set_max_size(Vec2::new(670.0, 400.0));

                        ui.vertical(|ui| {
                            ui.label(RichText::new("Unknown identity").heading().strong());
                            ui.separator();
                            let table = TableBuilder::new(ui)
                                .striped(false)
                                .resizable(false)
                                .cell_layout(Layout::left_to_right(Align::Center))
                                .column(Column::auto().at_least(150.0))
                                .column(Column::remainder());

                            table.body(|mut body| {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(RichText::new("Public Key").strong());
                                    });
                                    row.col(|ui| {
                                        let text = RichText::new("0F8D71E4A35B9C2056F28A1C4B0E37956DCA901F2B4E837A5C0D1B48F9E63275").monospace().color(Color32::LIGHT_GREEN);
                                        ui.label(text);
                                    });
                                });
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(RichText::new("Secret Key Location").strong());
                                    });
                                    row.col(|ui| {
                                        let text = RichText::new("~/.shoreline/identity/secret.key").monospace();
                                        ui.label(text);
                                    });
                                });
                            });
                        });
                    });
                });
            });
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_size(Vec2::new(300.0, 100.0));
                        ui.set_max_size(Vec2::new(300.0, 100.0));

                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No Identity Present").heading().strong());
                            ui.separator();
                            ui.label("You do not have an identity yet. Please create one to get started.");
                            ui.add_space(10.0);
                            if ui
                                .button(RichText::new("Create Identity").strong().color(Color32::LIGHT_GREEN))
                                .clicked()
                            {
                                self.provider.create();
                            }
                        });
                    });
                });
            });
        }
    }
}
