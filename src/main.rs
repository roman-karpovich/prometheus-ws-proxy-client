extern crate websocket;

use clap::{arg, value_parser, ArgAction, Command};
use log::{debug, info};
use names::Generator;

use tokio;

mod config;
mod tests;
mod utils;
mod worker;
mod ws_request;
mod ws_response;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
pub async fn main() {
    env_logger::init();

    let matches = Command::new("Prometheus websocket proxy")
        .version(VERSION)
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
            arg!(--sentry_dsn ... "sentry DSN")
                .action(ArgAction::Set)
                .value_parser(value_parser!(String)),
        ])
        .get_matches();

    let sentry_dsn = matches.get_one::<String>("sentry_dsn");
    let _guard;
    match sentry_dsn {
        Some(sentry_dsn) => {
            debug!("got {} as sentry dsn", sentry_dsn);
            _guard = sentry::init((
                sentry_dsn.clone(),
                sentry::ClientOptions {
                    release: sentry::release_name!(),
                    attach_stacktrace: true,
                    ..Default::default()
                },
            ));
            info!("Sentry configured");
        }
        None => {
            info!("Sentry not configured");
        }
    };

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
