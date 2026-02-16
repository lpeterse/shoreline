use crate::{app::MyUiExtensions as _, config::{AppConfig, ConfigCtrl, ConfigState}};
use eframe::egui;
use egui::*;

pub struct ConfigView {
    pub state: ConfigState,
    pub copy: Option<AppConfig>,
}

impl ConfigView {
    pub fn new() -> Self {
        Self {
                state: ConfigState::Loading,
            copy: None,
        }
    }

    pub fn update(&mut self, ctrl: &mut ConfigCtrl) {
        if ctrl.state().has_changed().unwrap_or_default() {
            self.state = ctrl.state().borrow().clone();
        }
    }

    pub fn show_top(&mut self, ui: &mut egui::Ui, ctrl: &mut ConfigCtrl) {
        ui.add_space(3.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            match &self.state {
                ConfigState::Result(Err(_)) => {
                    if ui.danger_button("Overwrite with default configuration").clicked() {
                        ctrl.reset();
                    }
                }
                ConfigState::Result(Ok(None)) => {
                    if ui.primary_button("Create default configuration").clicked() {
                        ctrl.reset();
                    }
                }
                ConfigState::Result(Ok(Some(config))) => {
                    if let Some(copy) = &self.copy {
                        if ui.primary_button("Save").clicked() {
                            ctrl.set(copy.clone());
                            self.copy = None;
                        }
                        if ui.add(Button::new("Cancel")).clicked() {
                            self.copy = None;
                        }
                        if ui.add(Button::new("Reset to defaults")).clicked() {
                            self.copy = Some(AppConfig::default());
                        }
                    } else {
                        if ui.primary_button("Edit").clicked() {
                            self.copy = Some(config.clone());
                        }
                    }
                }
                _ => ()
            }
        });
        ui.add_space(1.0);
    }

    pub fn show_center(&mut self, ui: &mut egui::Ui, ctrl: &mut ConfigCtrl) {
        match &mut self.state {
            ConfigState::Loading => {
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label("Loading configuration...");
                });
            },
            ConfigState::Result(Err(e)) => {
                ui.vertical(|ui: &mut Ui| {
                    ui.add(Label::new(RichText::new("Failed to load configuration: ").color(Color32::LIGHT_RED)).wrap());
                    ui.add_space(20.);
                    ui.add(Label::new(RichText::new(e.to_string()).monospace().color(Color32::LIGHT_RED)).wrap());
                    ui.add_space(20.);
                });
            },
            ConfigState::Result(Ok(None)) => {
                ui.centered_and_justified(|ui| {
                    ui.add(Label::new(RichText::new("No configuration found.").color(Color32::LIGHT_YELLOW)).wrap());
                });
            },
            ConfigState::Result(Ok(Some(config))) => {
                if let Some(copy) = &mut self.copy {
                    Self::show_config(ui, copy, true);
                } else {
                    Self::show_config(ui, config, false);
                }
            }
        }
    }

    pub fn show_config(ui: &mut egui::Ui, config: &mut AppConfig, edit: bool) {
        const GRID_SPACING: [f32; 2] = [20.0,10.0];
        const COL_0_MIN_WIDTH: f32 = 120.0;
        const COL_1_MIN_WIDTH: f32 = 250.0;

        CollapsingHeader::new("Identity").default_open(true).show(ui, |ui| {
            egui::Grid::new("Identity").num_columns(3).spacing(GRID_SPACING).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_width(COL_0_MIN_WIDTH);
                    ui.label("Name");
                });
                ui.horizontal(|ui| {
                    ui.set_min_width(COL_1_MIN_WIDTH);
                    if edit {
                        ui.add(egui::TextEdit::singleline(&mut config.identity.name));
                    } else {
                        ui.label(config.identity.name.clone());
                    }
                });
                ui.end_row();

                ui.label("User Key");
                let pubkey = RichText::new(config.identity.keypair.pubkey_as_pem())
                    .monospace()
                    .color(Color32::LIGHT_GREEN);
                ui.add(Label::new(pubkey).wrap());
                ui.add(Label::new("The public key of your node. This is used to identify you on the network and is derived from your secret key.").wrap());
                ui.end_row();
            });
        });

        CollapsingHeader::new("DHT").default_open(true).show(ui, |ui| {
            egui::Grid::new("DHT").num_columns(3).spacing(GRID_SPACING).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_width(COL_0_MIN_WIDTH);
                    ui.label("Enabled");
                });
                ui.horizontal(|ui| {
                    ui.set_min_width(COL_1_MIN_WIDTH);
                    ui.add_enabled(edit, egui::Checkbox::new(&mut config.dht.enabled, ""));
                });
                ui.label("Use the BitTorrent DHT to find peers.");
                ui.end_row();

                ui.label("Port");
                if edit {
                    ui.add(egui::DragValue::new(&mut config.dht.port).speed(1).range(1024..=65535));
                } else {
                    ui.label(config.dht.port.to_string());
                }
                ui.label("UDP port to listen for DHT traffic on. Default is 6881.");
                ui.end_row();

                ui.label("Bootstrap Nodes");
                if edit {
                    addable_list(ui, &mut config.dht.bootstrap_nodes);
                } else {
                    ui.label(RichText::new(config.dht.bootstrap_nodes.join("\n")).monospace());
                }
                ui.add(egui::Label::new("At least one bootstrap node is required to join the DHT. From there, you will automatically discover other nodes in the network. You can add more bootstrap nodes to improve reliability.").wrap());
                ui.end_row();
            })
        });
    }
}

pub fn addable_list(ui: &mut egui::Ui, items: &mut Vec<String>) {
    let state_id = ui.id().with("addable_list");

    let num_rows = items.len() + 1; // +1 for the input row
    let row_height = ui.spacing().interact_size.y; // Standard-Höhe für interaktive Elemente
    let spacing = ui.spacing().item_spacing.y;
    let total_h = (num_rows as f32 * row_height) + ((num_rows - 1) as f32 * spacing);

    let mut input = ui.data_mut(|d| d.get_temp::<String>(state_id).unwrap_or_default());
    let valid = (|| {
        let (host, port) = input.split_once(':')?;
        host.split('.').filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic() || c.is_ascii_digit() || c == '-')).count().checked_sub(1)?;
        port.parse::<u16>().ok()?;
        Some(())
    })().is_some();

    ui.add_sized([ui.available_width(), total_h], |ui: &mut egui::Ui| {
        egui::Grid::new("addable_list").show(ui, |ui| {
            let is = items.clone();
            for i in is {
                ui.label(RichText::new(i.clone()).monospace());
                if ui.add(Button::new("➖")).clicked() {
                    items.retain(|x| x != &i);
                }
                ui.end_row();
            }

            let rk = if valid {
                Some(egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Enter))
            } else {
                None
            };

            let res = ui.add(egui::TextEdit::singleline(&mut input).hint_text("example.com:6881").return_key(rk));
            let enter = res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let clicked = ui.add_enabled(valid, Button::new("➕")).clicked();

            if clicked || enter {
                items.push(input.clone());
                items.sort();
                ui.data_mut(|d| d.insert_temp(state_id, String::new()));
            } else {
                ui.data_mut(|d| d.insert_temp(state_id, input));
            }
        }).response
    });
}
