use eframe::egui;
use shoreline::app::MainApp;
use shoreline::{config::Config, mmdb::MMDB};
use shoreline_dht::DHT;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    let node: Result<(Arc<DHT>, MMDB), String> = rt.block_on(async {
        let dir = Config::dir().await.map_err(|e| e.to_string())?;
        let config = Config::load().await.map_err(|e| e.to_string())?;
        let dht = DHT::new(config.dht.node_id, config.dht.bind_port);
        let dht = Arc::new(dht);
        let mmdb = MMDB::new(dir.join("dbip-country.mmdb"));
        Ok((dht, mmdb))
    });

    let (dht, mmdb) = node.unwrap();

    let mut viewport = egui::ViewportBuilder::default().with_inner_size(MainApp::SIZE);
    viewport.icon = Some(std::sync::Arc::new(egui::IconData {
        rgba: image::load_from_memory(include_bytes!("../../assets/icon-white.png"))
            .unwrap()
            .to_rgba8()
            .to_vec(),
        width: 512,
        height: 512,
    }));

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        MainApp::NAME,
        options,
        Box::new(move |cc| Ok(Box::new(MainApp::new(cc.egui_ctx.clone(), rt, dht, mmdb)))),
    )?;

    Ok(())
}
