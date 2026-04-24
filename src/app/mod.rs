use crate::config::ConfigView;
use crate::config::ConfigCtrl;
use crate::dht::DhtView;
use crate::dht::DhtCtrl;
use crate::mdns::{MdnsView, MdnsCtrl};
use crate::peers::{PeersCtrl, PeersView};
use eframe::App;
use eframe::egui;
use egui::*;
use shoreline_dht::interval_skip;
use std::time::Duration;
use tokio::{runtime::Runtime, task::JoinHandle};
use crate::util::Netwatch;

pub struct MainApp {
    #[allow(dead_code)]
    rt: tokio::runtime::Runtime,
    state: AppState,
    task_paint: JoinHandle<()>,
}

impl MainApp {
    pub const NAME: &'static str = "Shoreline";
    pub const SIZE: [f32; 2] = [1024.0, 768.0];

    pub fn new(ctx: Context, rt: Runtime) -> Self {
        let task_paint = rt.spawn(Self::run_paint(ctx));
        let state = AppState::new(&rt);
        Self { rt: rt, state, task_paint }
    }

    async fn run_paint(ctx: Context) {
        let mut intvl = interval_skip(Duration::from_millis(1000));
        loop {
            intvl.tick().await;
            ctx.request_repaint();
        }
    }
}

impl Drop for MainApp {
    fn drop(&mut self) {
        self.task_paint.abort();
    }
}

impl App for MainApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.state.ui(ui, frame);
    }
}

pub struct AppState {
    pub active_tab: &'static str,

    pub config_view: ConfigView,
    pub config_ctrl: ConfigCtrl,

    pub dht_view: DhtView,
    pub dht_ctrl: DhtCtrl,

    pub mdns_view: MdnsView,
    pub mdns_ctrl: MdnsCtrl,

    pub peers_view: PeersView,
    pub peers_ctrl: PeersCtrl,
}

impl AppState {
    pub const TAB_DASHBOARD: &'static str = "dashboard";
    pub const TAB_DASHBOARD_DISPLAY: &'static str = "Dashboard";
    pub const TAB_CONFIG: &'static str = "config";
    pub const TAB_CONFIG_DISPLAY: &'static str = "⛭";
    pub const TAB_ID: &'static str = "identity";
    pub const TAB_ID_DISPLAY: &'static str = "Identity";
    pub const TAB_CLUSTER: &'static str = "cluster";
    pub const TAB_CLUSTER_DISPLAY: &'static str = "Cluster";
    pub const TAB_CIRCLES: &'static str = "circles";
    pub const TAB_CIRCLES_DISPLAY: &'static str = "Circles";
    pub const TAB_SETTINGS: &'static str = "settings";
    pub const TAB_SETTINGS_DISPLAY: &'static str = "Settings";
    pub const TAB_PEERS: &'static str = "peers";
    pub const TAB_PEERS_DISPLAY: &'static str = "Peers";
    pub const TAB_DHT: &'static str = "dht";
    pub const TAB_DHT_DISPLAY: &'static str = "DHT";
    pub const TAB_MDNS: &'static str = "mdns";
    pub const TAB_MDNS_DISPLAY: &'static str = "mDNS";
    pub const TAB_LOG: &'static str = "log";
    pub const TAB_LOG_DISPLAY: &'static str = "Log";
    pub const TAB_DEFAULT: &'static str = Self::TAB_PEERS;

    pub fn new(rt: &Runtime) -> Self {
        let config_view = ConfigView::new();
        let config_ctrl = ConfigCtrl::new(rt);

        let netwatch = Netwatch::new(rt);

        let mdns_view = MdnsView::new();
        let mdns_ctrl = MdnsCtrl::new(rt, config_ctrl.clone(), netwatch.clone());

        let dht_view = DhtView::new();
        let dht_ctrl = DhtCtrl::new(rt, config_ctrl.clone());

        let peers_view = PeersView::new();
        let peers_ctrl = PeersCtrl::new(rt, config_ctrl.clone(), netwatch.clone(), dht_ctrl.clone(), mdns_ctrl.clone());

        Self { config_view, config_ctrl, dht_view, dht_ctrl, mdns_view, mdns_ctrl, peers_view, peers_ctrl, active_tab: Self::TAB_DEFAULT }
    }
}

impl App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.config_view.update(&mut self.config_ctrl);
        ui.set_theme(Theme::Dark);

        Panel::top("menu").show_inside(ui, |ui| {
            ui.add_space(3.0);
            ui.columns(2, |cols| {
                cols[0].with_layout(Layout::left_to_right(Align::TOP), |ui| {
                    ui.selectable_value(&mut self.active_tab, Self::TAB_PEERS, Self::TAB_PEERS_DISPLAY);
                    if self.dht_ctrl.dht().borrow().is_some() {
                        ui.selectable_value(&mut self.active_tab, Self::TAB_DHT, Self::TAB_DHT_DISPLAY);
                    }
                    ui.selectable_value(&mut self.active_tab, Self::TAB_MDNS, Self::TAB_MDNS_DISPLAY);
                });
                cols[1].with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.selectable_value(&mut self.active_tab, Self::TAB_CONFIG, Self::TAB_CONFIG_DISPLAY);
                    ui.selectable_value(&mut self.active_tab, Self::TAB_LOG, Self::TAB_LOG_DISPLAY);
                });
            });
            ui.add_space(1.0);
        });

        match self.active_tab {
            Self::TAB_PEERS => {
                Panel::top("top").show_inside(ui, |ui| {
                    self.peers_view.show_top(ui, &self.peers_ctrl);
                });
                let frame = Frame::default().inner_margin(Margin::ZERO).fill(ui.style().visuals.window_fill());
                CentralPanel::default().frame(frame).show_inside(ui, |ui| {
                    self.peers_view.show_central(ui, &self.peers_ctrl);
                });
            }
            Self::TAB_CONFIG => {
                Panel::top("top").show_inside(ui, |ui| {
                    self.config_view.show_top(ui, &mut self.config_ctrl)
                });
                CentralPanel::default().show_inside(ui, |ui| {
                    self.config_view.show_center(ui, &mut self.config_ctrl)
                });
            },
            Self::TAB_LOG => {
                CentralPanel::default().show_inside(ui, |ui| {
                    egui_logger::logger_ui().show(ui);
                });
            }
            Self::TAB_DHT => {
                Panel::top("top").show_inside(ui, |ui| {
                    self.dht_view.show_top(ui, &self.dht_ctrl);
                });
                let frame = Frame::default().inner_margin(Margin::ZERO).fill(ui.style().visuals.window_fill());
                CentralPanel::default().frame(frame).show_inside(ui, |ui| {
                    self.dht_view.show_central(ui, &self.dht_ctrl);
                });
            }
            Self::TAB_MDNS => {
                Panel::top("top").show_inside(ui, |ui| {
                    self.mdns_view.show_top(ui, &self.mdns_ctrl);
                });
                let frame = Frame::default().inner_margin(Margin::ZERO).fill(ui.style().visuals.window_fill());
                CentralPanel::default().frame(frame).show_inside(ui, |ui| {
                    self.mdns_view.show_central(ui, &self.mdns_ctrl);
                });
            }
            _ => {
                CentralPanel::default().show_inside(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        let text = RichText::new("Not implemented yet").color(Color32::LIGHT_GRAY).heading();
                        ui.add(Label::new(text));
                    });
                });
            }
        }

        Panel::bottom("footer").show_inside(ui, |ui| {
            ui.label(env!("CARGO_PKG_VERSION"));
        });
    }
}

pub trait MyUiExtensions {
    fn primary_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn danger_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn outline_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
}

impl MyUiExtensions for egui::Ui {
    fn primary_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        self.add(
            egui::Button::new(text)
                .fill(egui::Color32::from_rgb(0, 123, 255).gamma_multiply(0.5))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 100, 200)))
        )
    }

    fn danger_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        self.add(
            egui::Button::new(text)
                .fill(egui::Color32::from_rgb(220, 53, 69).gamma_multiply(0.5))
        )
    }

    fn outline_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        let text: egui::WidgetText = text.into();
        let button = egui::Button::new(text)
            .frame(true)
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(1.0, self.visuals().widgets.active.bg_fill));
        
        self.add(button)
    }
}
