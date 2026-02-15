mod dht;
//mod id;
mod config;

use crate::app::config::ConfigApp;
use crate::app::dht::DhtApp;
use crate::config::ConfigCtrl;
use crate::dht::DhtCtrl;
//use crate::app::id::IdApp;
use eframe::App;
use eframe::egui;
use egui::*;
use shoreline_dht::interval_skip;
use std::time::Duration;
use tokio::{runtime::Runtime, task::JoinHandle};

pub struct MainApp {
    #[allow(dead_code)]
    rt: tokio::runtime::Runtime,
    state: AppState,
    task_paint: JoinHandle<()>,
}

impl MainApp {
    pub const NAME: &'static str = "Shoreline";
    pub const SIZE: [f32; 2] = [800.0, 600.0];

    pub fn new(ctx: Context, rt: Runtime) -> Self {
        let task_paint = rt.spawn(Self::run_paint(ctx));
        let state = AppState::new(&rt);
        Self { rt: rt, state, task_paint }
    }

    async fn run_paint(ctx: Context) {
        let mut intvl = interval_skip(Duration::from_millis(100));
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.state.update(ctx, frame);
    }
}

pub struct AppState {
    pub selected: &'static str,

    pub config_app: ConfigApp,
    pub config_ctrl: ConfigCtrl,

    pub dht_app: DhtApp,
    pub dht_ctrl: DhtCtrl,
}

impl AppState {
    pub const TAB_DASHBOARD: &'static str = "dashboard";
    pub const TAB_DASHBOARD_DISPLAY: &'static str = "Dashboard";
    pub const TAB_CONFIG: &'static str = "config";
    pub const TAB_CONFIG_DISPLAY: &'static str = "Config";
    pub const TAB_ID: &'static str = "identity";
    pub const TAB_ID_DISPLAY: &'static str = "Identity";
    pub const TAB_CLUSTER: &'static str = "cluster";
    pub const TAB_CLUSTER_DISPLAY: &'static str = "Cluster";
    pub const TAB_CIRCLES: &'static str = "circles";
    pub const TAB_CIRCLES_DISPLAY: &'static str = "Circles";
    pub const TAB_SETTINGS: &'static str = "settings";
    pub const TAB_SETTINGS_DISPLAY: &'static str = "Settings";
    pub const TAB_DHT: &'static str = "dht";
    pub const TAB_DHT_DISPLAY: &'static str = "DHT";
    pub const TAB_DEFAULT: &'static str = Self::TAB_CONFIG;

    pub fn new(rt: &Runtime) -> Self {
        let config_ctrl = ConfigCtrl::new(rt);
        let config_app = ConfigApp::new(config_ctrl.clone());

        let dht_ctrl = DhtCtrl::new(rt, config_ctrl.clone());
        let dht_app = DhtApp::new(dht_ctrl.clone());

        Self { config_app, config_ctrl, dht_app, dht_ctrl, selected: Self::TAB_DEFAULT }
    }
}

impl App for AppState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.set_theme(Theme::Dark);
        TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected, Self::TAB_CONFIG, Self::TAB_CONFIG_DISPLAY);
                // ui.selectable_value(&mut self.selected, Self::TAB_ID, Self::TAB_ID_DISPLAY);
                if self.dht_ctrl.dht().is_some() {
                    ui.selectable_value(&mut self.selected, Self::TAB_DHT, Self::TAB_DHT_DISPLAY);
                }
            });
            ui.add_space(1.0);
        });

        match self.selected {
            Self::TAB_CONFIG => self.config_app.update(ctx, frame),
            Self::TAB_DHT => self.dht_app.update(ctx, frame),
            _ => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        let text = RichText::new("Not implemented yet").color(Color32::LIGHT_GRAY).heading();
                        ui.add(Label::new(text));
                    });
                });
            }
        }

        TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.label(env!("CARGO_PKG_VERSION"));
        });
    }
}
