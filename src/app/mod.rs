mod dht;

use crate::{app::dht::DhtApp, mmdb::MMDB};
use eframe::App;
use eframe::egui;
use egui::*;
use shoreline_dht::DHT;
use std::sync::Arc;
use tokio::{runtime::Runtime, task::JoinHandle};

pub struct MainApp {
    #[allow(dead_code)]
    rt: tokio::runtime::Runtime,
    tab: &'static str,
    tab_dht: DhtApp,
    task: JoinHandle<()>,
}

impl MainApp {
    pub const NAME: &'static str = "Shoreline";
    pub const SIZE: [f32; 2] = [800.0, 600.0];

    pub const TAB_DASHBOARD: &'static str = "dashboard";
    pub const TAB_DASHBOARD_DISPLAY: &'static str = "Dashboard";
    pub const TAB_IDENTITY: &'static str = "identity";
    pub const TAB_IDENTITY_DISPLAY: &'static str = "Identity";
    pub const TAB_CLUSTER: &'static str = "cluster";
    pub const TAB_CLUSTER_DISPLAY: &'static str = "Cluster";
    pub const TAB_CIRCLES: &'static str = "circles";
    pub const TAB_CIRCLES_DISPLAY: &'static str = "Circles";
    pub const TAB_SETTINGS: &'static str = "settings";
    pub const TAB_SETTINGS_DISPLAY: &'static str = "Settings";
    pub const TAB_DHT: &'static str = "dht";
    pub const TAB_DHT_DISPLAY: &'static str = "DHT";
    pub const TAB_DEFAULT: &'static str = Self::TAB_DHT;

    pub fn new(ctx: Context, rt: Runtime, dht: Arc<DHT>, mmdb: MMDB) -> Self {
        let ctx = ctx.clone();
        let task = rt.spawn(async move {
            let mut intvl = tokio::time::interval(std::time::Duration::from_millis(100));
            loop {
                intvl.tick().await;
                ctx.request_repaint();
            }
        });
        Self { rt: rt, tab_dht: DhtApp::new(dht, mmdb), tab: Self::TAB_DEFAULT, task }
    }
}

impl Drop for MainApp {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(Theme::Dark);
        TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Self::TAB_DASHBOARD, Self::TAB_DASHBOARD_DISPLAY);
                ui.selectable_value(&mut self.tab, Self::TAB_IDENTITY, Self::TAB_IDENTITY_DISPLAY);
                ui.selectable_value(&mut self.tab, Self::TAB_CLUSTER, Self::TAB_CLUSTER_DISPLAY);
                ui.selectable_value(&mut self.tab, Self::TAB_CIRCLES, Self::TAB_CIRCLES_DISPLAY);
                ui.selectable_value(&mut self.tab, Self::TAB_DHT, Self::TAB_DHT_DISPLAY);
                ui.selectable_value(&mut self.tab, Self::TAB_SETTINGS, Self::TAB_SETTINGS_DISPLAY);
            });
            ui.add_space(1.0);
        });

        match self.tab {
            Self::TAB_DHT => {
                self.tab_dht.update(ctx, _frame);
            }
            _ => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.centered_and_justified(|ui| {
                        let text = RichText::new("Not implemented yet").color(Color32::LIGHT_GRAY).heading();
                        ui.add(Label::new(text));
                    });
                });
            }
        }
    }
}
