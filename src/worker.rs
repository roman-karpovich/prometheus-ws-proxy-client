use crate::config::Config;
use crate::ws_request;
use crate::ws_response::{WSRegisterMessageV1, WSRegisterMessageV2, WSResponseMessage};
use log::{debug, error, info, warn};
use rand::Rng;
use serde::Serialize;
use std::error::Error;

use std::thread::sleep;
use std::time::Duration;

use futures::channel::mpsc::UnboundedSender;

use futures_util::{future, pin_mut, SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;

#[derive(Serialize, Debug)]
pub struct ResourceResponse {
    status: u16,
    body: String,
}

async fn call_resource(url: String) -> Result<ResourceResponse, Box<dyn Error>> {
    debug!("{}", url);
    let response = reqwest::get(url).await?;
    let r = ResourceResponse {
        status: response.status().as_u16(),
        body: response.text().await?,
    };
    Ok(r)
}

pub async fn handle_request(request_id: String, url: String, mut tx: UnboundedSender<Message>) {
    let response = call_resource(url).await.unwrap();
    // send response
    let response_message = WSResponseMessage {
        message_type: "response".to_string(),
        uid: request_id,
        body: response.body,
        status: response.status,
    };
    let response_json = serde_json::to_string(&response_message).unwrap();
    debug!("{:?}", response_json);
    match tx.send(Message::Text(response_json)).await {
        Ok(()) => (),
        Err(e) => {
            error!("Handle Request: {:?}", e);
        }
    }
}

async fn connect_to_server(
    worker_name: String,
    config_path: String,
    protocol_version: u16,
) -> Result<(), Box<dyn Error>> {
    let config = Config::from_file(config_path).expect("Unable to read config file");
    info!(
        "Instance: {}. Connecting to target: {}",
        config.get_instance_name(),
        config.target
    );
    if config.cf_access_enabled {
        debug!(
            "Cloudflare headers config: enabled - {}, key - {}",
            config.cf_access_enabled, config.cf_access_key
        );
    } else {
        debug!("Cloudflare headers not enabled");
    }
    debug!("resources configured: {:?}", config.resources);

    let instance_name = config.get_instance_name().to_owned();

    let mut request = config.target.clone().into_client_request()?;
    if config.cf_access_enabled {
        let headers = request.headers_mut();
        headers.insert("CF-Access-Client-Id", config.cf_access_key.parse().unwrap());
        headers.insert(
            "CF-Access-Client-Secret",
            config.cf_access_secret.parse().unwrap(),
        );
    }

    debug!("{:?}", request);

    let (tx, rx) = futures_channel::mpsc::unbounded();
    let (socket, _) = connect_async(request).await?;

    info!("Successfully connected");

    let (write, read) = socket.split();

    let rx_to_ws = rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|msg| async {
            let local_worker = worker_name.clone();
            let local_config = config.clone();
            let mut local_tx = tx.clone();

            let message = match msg {
                Ok(m) => m,
                Err(e) => {
                    debug!("Receive Loop: {:?}", e);
                    let _ = tx.clone().send(Message::Close(None));
                    return;
                }
            };
            debug!("message: {:?}", message.clone());
            match message {
                Message::Close(_) => {
                    debug!("Close received");
                    // Got a close message, so send a close message and return
                    let _ = local_tx.send(Message::Close(None));
                    return;
                }
                Message::Ping(data) => {
                    debug!("Ping received");
                    match local_tx.send(Message::Pong(data)).await {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            error!("Receive Loop: {:?}", e);
                            return;
                        }
                    }
                }
                Message::Text(value) => {
                    debug!("Receive Loop: {:?}", value);
                    let request = ws_request::read_ws_message(value);
                    request
                        .handle(local_worker.clone(), local_config, local_tx)
                        .await;
                    return;
                }
                _ => {
                    warn!("Receive Loop (unknown message): {:?}", message);
                    return;
                }
            }
        })
    };

    // register
    let register_json = match protocol_version {
        1 => {
            let register_message = WSRegisterMessageV1 {
                message_type: "register".to_string(),
                instance: instance_name,
            };
            serde_json::to_string(&register_message).unwrap()
        }
        _ => {
            let register_message = WSRegisterMessageV2 {
                message_type: "register".to_string(),
                instance: instance_name,
                worker: worker_name.clone(),
                version: 2,
            };
            serde_json::to_string(&register_message).unwrap()
        }
    };
    // match tx.send(OwnedMessage::Text(register_json)) {
    match tx.clone().send(Message::Text(register_json)).await {
        Ok(()) => (),
        Err(e) => {
            error!("Main Loop: {:?}", e);
        }
    }
    info!("Successfully registered");

    pin_mut!(rx_to_ws, ws_to_stdout);
    future::select(rx_to_ws, ws_to_stdout).await;

    info!("Exited");

    Ok(())
}

pub async fn run_worker(name: String, config_path: String, protocol_version: u16) {
    info!("Worker {} starting", name);
    loop {
        match connect_to_server(name.clone(), config_path.clone(), protocol_version).await {
            Ok(()) => (),
            Err(e) => {
                error!("Worker {} exited: {:?}", name.clone(), e);
            }
        }
        let mut rng = rand::thread_rng();
        sleep(Duration::from_secs(1 + rng.gen_range(0..5)));
    }
}
