use shoreline::{config::Config, SEEDS};
use shoreline_dht::DHT;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new().filter_level(log::LevelFilter::Info).init();

    let seeds = watch::channel(
        SEEDS.iter()
            .filter_map(|s| s.parse().ok())
            .collect::<Vec<_>>(),
    ).1;
    let config = Config::load().await.map_err(|e| e.to_string())?;
    let dht = DHT::new(config.dht.node_id, config.dht.bind_port, seeds);

    loop {
        let len = dht.peers().len();
        log::info!("DHT is running ({} peers)...", len);
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
