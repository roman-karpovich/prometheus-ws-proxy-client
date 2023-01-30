extern crate websocket;

use clap::{arg, value_parser, ArgAction, Command};
use log::{error, info};
use names::Generator;
use std::thread;

mod config;
mod tests;
mod utils;
mod worker;
mod ws_request;
mod ws_response;

fn main() {
    env_logger::init();

    let matches = Command::new("Prometheus websocket proxy")
        .version("2.0.1")
        .author("Roman Karpovich <fpm.th13f@gmail.com>")
        .about("Connects to websocket server to call local resources")
        .args(&[
            arg!([config] "path to config").default_value("client_config.json"),
            arg!(-p --parallel ... "number of connections to use")
                .action(ArgAction::Set)
                .value_parser(value_parser!(u16))
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

    let connections_number = *matches.get_one::<u16>("parallel").unwrap();
    info!("Run {} workers", connections_number);

    let mut generator = Generator::default();
    let mut threads = Vec::new();

    for _i in 0..connections_number {
        let worker_name = generator.next().unwrap();
        let local_config_path = config_path.clone().to_string();

        threads.push(thread::spawn(move || {
            worker::run_worker(
                worker_name.clone(),
                local_config_path.as_str(),
                protocol_version,
            );
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
