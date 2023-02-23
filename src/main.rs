extern crate websocket;

use clap::{arg, value_parser, ArgAction, Command};
use log::info;
use names::Generator;

use tokio;

mod config;
mod tests;
mod utils;
mod worker;
mod ws_request;
mod ws_response;

#[tokio::main]
pub async fn main() {
    env_logger::init();

    let matches = Command::new("Prometheus websocket proxy")
        .version("2.0.1")
        .author("Roman Karpovich <fpm.th13f@gmail.com>")
        .about("Connects to websocket server to call local resources")
        .args(&[
            arg!([config] "path to config").default_value("client_config.json"),
            arg!(-p --parallel ... "number of connections to use")
                .action(ArgAction::Set)
                .value_parser(value_parser!(usize))
                .default_value("3"),
            arg!(-r --protocol ... "protocol version")
                .action(ArgAction::Set)
                .value_parser(value_parser!(u16))
                .default_value("2"),
        ])
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();
    info!("Using config {}", config_path);

    let protocol_version = *matches.get_one::<u16>("protocol").unwrap();
    info!("Protocol version {}", protocol_version);

    let connections_number = *matches.get_one::<usize>("parallel").unwrap();
    info!("Run {} workers", connections_number);

    let mut generator = Generator::default();
    let mut workers = Vec::with_capacity(connections_number);

    for _i in 0..connections_number {
        let worker_name = generator.next().unwrap();
        workers.push(worker::run_worker(
            worker_name.clone(),
            config_path.clone(),
            protocol_version,
        ));
    }

    futures_util::future::join_all(workers).await;
}
