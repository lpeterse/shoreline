use crate::config::{AppConfig, ConfigCtrl, ConfigState};
use eframe::egui;
use egui::*;

pub struct ConfigApp {
    pub ctrl: ConfigCtrl,
    pub config: Option<AppConfig>,
}

impl ConfigApp {
    pub fn new(config: ConfigCtrl) -> Self {
        Self {
            ctrl: config,
            config: None,
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        const GRID_SPACING: [f32; 2] = [20.0,10.0];
        const COL_0_MIN_WIDTH: f32 = 120.0;
        const COL_1_MIN_WIDTH: f32 = 250.0;

        if self.config.is_none() || self.ctrl.state().has_changed().unwrap_or_default() {
            match { self.ctrl.state().borrow_and_update().clone() } {
                ConfigState::Loading => {
                    CentralPanel::default().show(ctx, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label("Loading configuration...");
                        });
                    });
                },
                ConfigState::Result(Err(e)) => {
                    CentralPanel::default().show(ctx, |ui| {
                        ui.vertical(|ui: &mut Ui| {
                            ui.add(Label::new(RichText::new("Failed to load configuration: ").color(Color32::LIGHT_RED)).wrap());
                            ui.add_space(20.);
                            ui.add(Label::new(RichText::new(e.to_string()).monospace().color(Color32::LIGHT_RED)).wrap());
                            ui.add_space(20.);
                            ui.horizontal(|ui| {
                                if ui.add(Button::new("Reset to defaults").fill(Color32::DARK_BLUE)).clicked() {
                                    self.ctrl.reset();
                                }
                                if ui.add(Button::new("Reload")).clicked() {
                                    self.ctrl.reload();
                                }
                            })
                        }).response
                    });
                },
                ConfigState::Result(Ok(None)) => {
                    CentralPanel::default().show(ctx, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.vertical(|ui| {
                                ui.add(Label::new(RichText::new("No configuration found.").color(Color32::LIGHT_YELLOW)).wrap());
                                ui.add_space(20.);
                                if ui.add(Button::new("Create default configuration")).clicked() {
                                    self.ctrl.reset();
                                }
                            });
                        });
                    });
                },
                ConfigState::Result(Ok(Some(config))) => {
                    self.config = Some(config);
                }
            }
        }

        if let Some(config) = &mut self.config {
            CentralPanel::default().show(ctx, |ui| {
                CollapsingHeader::new("Identity").default_open(true).show(ui, |ui| {
                    egui::Grid::new("Identity").num_columns(3).spacing(GRID_SPACING).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_min_width(COL_0_MIN_WIDTH);
                            ui.label("Name");
                        });
                        ui.horizontal(|ui| {
                            ui.set_min_width(COL_1_MIN_WIDTH);
                            ui.label(config.identity.name.clone());
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
                            if toggle_ui(ui, &mut config.dht.enabled).changed() {
                                self.ctrl.set(config.clone());
                            }
                        });
                        ui.label("Use the BitTorrent DHT to find peers.");
                        ui.end_row();

                        ui.label("Port");
                        if ui.add(egui::DragValue::new(&mut config.dht.port).speed(10).range(1024..=65535)).changed() {
                            self.ctrl.set(config.clone());
                        }
                        ui.label("UDP port to listen for DHT traffic on. Default is 6881.");
                        ui.end_row();

                        ui.label("Bootstrap Nodes");
                        addable_list(ui, &mut config.dht.bootstrap_nodes);
                        ui.add(egui::Label::new("At least one bootstrap node is required to join the DHT. From there, you will automatically discover other nodes in the network. You can add more bootstrap nodes to improve reliability.").wrap());
                        ui.end_row();
                    })
                });
            });
        }
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

pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    // Widget code can be broken up in four steps:
    //  1. Decide a size for the widget
    //  2. Allocate space for it
    //  3. Handle interactions with the widget (if any)
    //  4. Paint the widget

    // 1. Deciding widget size:
    // You can query the `ui` how much space is available,
    // but in this example we have a fixed size widget based on the height of a standard button:
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);

    // 2. Allocating space:
    // This is where we get a region of the screen assigned.
    // We also tell the Ui to sense clicks in the allocated region.
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    // 3. Interact: Time to check for clicks!
    if response.clicked() {
        *on = !*on;
        response.mark_changed(); // report back that the value changed
    }

    // Attach some meta-data to the response which can be used by screen readers:
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, ""));

    // 4. Paint!
    // Make sure we need to paint:
    if ui.is_rect_visible(rect) {
        // Let's ask for a simple animation from egui.
        // egui keeps track of changes in the boolean associated with the id and
        // returns an animated value in the 0-1 range for how much "on" we are.
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        // We will follow the current style by asking
        // "how should something that is being interacted with be painted?".
        // This will, for instance, give us different colors when the widget is hovered or clicked.
        let visuals = ui.style().interact_selectable(&response, *on);
        // All coordinates are in absolute screen coordinates so we use `rect` to place the elements.
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(rect, radius, visuals.bg_fill, visuals.bg_stroke, egui::StrokeKind::Inside);
        // Paint the circle, animating it from left to right with `how_on`:
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter().circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    response
}

