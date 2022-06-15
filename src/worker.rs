use std::error::Error;
use std::sync::mpsc::{channel, Sender};
use std::thread::sleep;
use std::time::Duration;
use websocket::ClientBuilder;
use std::thread;
use log::{debug, error, info, warn};
use rand::Rng;
use crate::{Headers, Message, OwnedMessage, ResourceResponse, ResponseMessage, ws_request};
use crate::config::Config;

fn call_resource(url: &String) -> Result<ResourceResponse, Box<dyn Error>> {
    debug!("{}", url);
    let response = reqwest::blocking::get(url)?;
    let r = ResourceResponse { status: response.status().as_u16(), body: response.text()? };
    Ok(r)
}

pub fn handle_request(request_id: String, url: String, tx: Sender<OwnedMessage>) {
    let response = call_resource(&url).unwrap();
    // send response
    let response_message = ResponseMessage {
        message_type: "response".to_string(),
        uid: request_id,
        body: response.body,
        status: response.status,
    };
    let response_json = serde_json::to_string(&response_message).unwrap();
    debug!("{:?}", response_json);
    match tx.send(OwnedMessage::Text(response_json)) {
        Ok(()) => (),
        Err(e) => {
            error!("Handle Request: {:?}", e);
        }
    }
}

fn connect_to_server(config_path: &str) -> Result<(), Box<dyn Error>> {
    let config = Config::from_file(config_path).unwrap();
    info!("Instance: {}. Connecting to target: {}", config.instance, config.target);
    if config.cf_access_enabled {
        debug!("Cloudflare headers config: enabled - {}, key - {}", config.cf_access_enabled, config.cf_access_key);
    } else {
        debug!("Cloudflare headers not enabled");
    }
    debug!("resources configured: {:?}", config.resources);

    let instance_name = config.instance.to_owned();

    let mut client_builder = ClientBuilder::new(config.target.as_str()).unwrap();
    if config.cf_access_enabled {
        let mut headers = Headers::new();
        headers.set_raw(
            "CF-Access-Client-Id",
            vec![config.cf_access_key.as_bytes().to_vec()],
        );
        headers.set_raw(
            "CF-Access-Client-Secret",
            vec![config.cf_access_secret.as_bytes().to_vec()],
        );
        client_builder = client_builder.custom_headers(&headers);
    }

    let client = client_builder.connect_insecure();

    if client.is_err() {
        Err("Unable to connect to the server")?;
    }
    let client = client.unwrap();

    info!("Successfully connected");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    error!("Send Loop: {:?}", e);
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&message);
                    // If it's a close message, just send it and then return.
                    return;
                }
                _ => (),
            }
            // Send the message
            match sender.send_message(&message) {
                Ok(()) => (),
                Err(e) => {
                    error!("Send Loop: {:?}", e);
                    let _ = sender.send_message(&Message::close());
                    return;
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        // Receive loop
        for message in receiver.incoming_messages() {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    debug!("Receive Loop: {:?}", e);
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    debug!("Close received");
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    debug!("Ping received");
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            error!("Receive Loop: {:?}", e);
                            continue;
                        }
                    }
                }
                OwnedMessage::Text(value) => {
                    debug!("Receive Loop: {:?}", value);
                    let request = ws_request::read_ws_message(value);
                    request.handle(&config, tx_1.clone());
                    continue;
                }
                _ => {
                    warn!("Receive Loop (unknown message): {:?}", message);
                    return;
                }
            }
        }
    });

    // register
    let register_message = format!(r#"
	{{
		"type": "register",
		"instance": "{}"
	}}"#, instance_name);
    match tx.send(OwnedMessage::Text(register_message)) {
        Ok(()) => (),
        Err(e) => {
            error!("Main Loop: {:?}", e);
        }
    }
    info!("Successfully registered");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    info!("Exited");

    Ok(())
}

pub fn run_worker(name: &str, config_path: &str) {
    info!("Worker {} starting", name);
    loop {
        match connect_to_server(config_path) {
            Ok(()) => (),
            Err(e) => {
                error!("Worker {} exited: {:?}", name, e);
            }
        }
        let mut rng = rand::thread_rng();
        sleep(Duration::from_secs(1 + rng.gen_range(0..5)));
    }
}
