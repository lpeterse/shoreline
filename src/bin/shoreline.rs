use eframe::egui;
use shoreline::Error;
use shoreline::app::MainApp;

fn main() -> Result<(), Error> {
    egui_logger::builder().init()?;

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let mut viewport = egui::ViewportBuilder::default().with_inner_size(MainApp::SIZE);
    let icon = include_bytes!("../../assets/icon-white.png");
    viewport.icon = Some(std::sync::Arc::new(egui::IconData {
        rgba: image::load_from_memory(icon).unwrap().to_rgba8().to_vec(),
        width: 512,
        height: 512,
    }));

    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        MainApp::NAME,
        options,
        Box::new(move |cc| Ok(Box::new(MainApp::new(cc.egui_ctx.clone(), rt)))),
    ).map_err(|e| e.to_string())?;

    Ok(())
}
