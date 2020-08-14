#![deny(unsafe_code)]

use common::clock::Clock;
use server::{Event, Input, Server, ServerSettings};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

use std::sync::{mpsc, Arc};
use worldsim::{
    job::JobManager,
    region::Region,
    regionmanager::{meta::RegionManagerMsg, RegionManager},
    server::meta::ServerMsg,
};
const TPS: u64 = 30;
const RUST_LOG_ENV: &str = "RUST_LOG";

fn main() {
    // Init logging
    let filter = match std::env::var_os(RUST_LOG_ENV).map(|s| s.into_string()) {
        Some(Ok(env)) => {
            let mut filter = EnvFilter::new("veloren_world::sim=info")
                .add_directive("veloren_world::civ=info".parse().unwrap())
                .add_directive(LevelFilter::INFO.into());
            for s in env.split(',').into_iter() {
                match s.parse() {
                    Ok(d) => filter = filter.add_directive(d),
                    Err(err) => println!("WARN ignoring log directive: `{}`: {}", s, err),
                };
            }
            filter
        },
        _ => EnvFilter::from_env(RUST_LOG_ENV)
            .add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive(LevelFilter::INFO.into()),
    };

    FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter(filter)
        .init();

    info!("Starting server...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();
    let server_port = &settings.gameserver_address.port();
    let metrics_port = &settings.metrics_address.port();

    let (region_manager_tx, region_manager_rx) = mpsc::channel::<RegionManagerMsg>();
    let (server_tx, server_rx) = mpsc::channel::<ServerMsg>();

    let mut region_manager = RegionManager::new(region_manager_tx, server_rx);
    let mut job_manager: Arc<JobManager> = Arc::new(JobManager::new());
    let mut server =
        worldsim::server::Server::new(server_tx, region_manager_rx, job_manager.clone());
    let mut region = Region::new((0, 0), job_manager.clone());

    job_manager.repeat(move || region_manager.work());
    job_manager.repeat(move || server.work());

    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    info!("Server is ready to accept connections.");
    info!(?metrics_port, "starting metrics at port");
    info!(?server_port, "starting server at port");

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity: _ } => info!("Client connected!"),
                Event::ClientDisconnected { entity: _ } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
