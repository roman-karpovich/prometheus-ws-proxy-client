extern crate websocket;

use clap::{Arg, Command};
use log::{error, info};
use names::Generator;
use std::thread;

mod config;
mod worker;
mod ws_request;
mod ws_response;

fn main() {
    env_logger::init();

    let matches = Command::new("Prometheus websocket proxy")
        .version("2.0.1")
        .author("Roman Karpovich <fpm.th13f@gmail.com>")
        .about("Connects to websocket server to call local resources")
        .arg(Arg::new("config").help("path to config").takes_value(true))
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .help("number of connections to use")
                .takes_value(true),
        )
        .get_matches();

    let config_path = matches.value_of("config").unwrap_or("client_config.json");
    info!("Using config {}", config_path);

    let connections_number: usize = matches.value_of_t("parallel").unwrap_or(3);
    info!("Run {} workers", connections_number);

    let mut generator = Generator::default();
    let mut threads = Vec::new();

    for _i in 0..connections_number {
        let worker_name = generator.next().unwrap();
        let local_config_path = config_path.clone().to_string();

        threads.push(thread::spawn(move || {
            worker::run_worker(worker_name.as_str(), local_config_path.as_str());
        }));
    }

    for handle in threads {
        match handle.join() {
            Ok(()) => (),
            Err(e) => {
                error!("Main Loop: {:?}", e);
            }
        }
    }
}
