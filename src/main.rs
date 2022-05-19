// fn main() {
//     println!("Hello, world!");
// }
extern crate websocket;

use std::borrow::Borrow;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, stdin};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use clap::{Arg, Command};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};
// use websocket::{Message, OwnedMessage};

// const CONNECTION: &'static str = "ws://127.0.0.1:2794";

// todo!(use name as key instead of strict naming)
#[derive(Deserialize, Debug, Clone)]
struct Resource {
    name: String,
    url: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    instance: String,
    target: String,
    resources: Vec<Resource>,
    // cf_access_enabled: String,
    // cf_access_key: String,
    // cf_access_secret: String,
}

impl Config {
    fn get_resource(&self, name: String) -> Option<Resource> {
        (*self.resources)[..].to_vec().into_iter().find(|x| x.name == name)
    }
}

impl Clone for Config {
    fn clone(&self) -> Config {
        Config {
            instance: (*self.instance).parse().unwrap(),
            target: (*self.target).parse().unwrap(),
            resources: (*self.resources)[..].to_vec(),
        }
    }
}

trait WSProxyRequest {
    fn handle(self, config: Config);
}

#[derive(Deserialize, Debug)]
struct WSProxyCallRequest {
    uid: String,
    resource: String,
}

impl WSProxyRequest for WSProxyCallRequest {
    fn handle(self, config: Config) {
        println!("Handling request {}: {}", self.uid, self.resource);
        let resource = config.get_resource(self.resource);
        if resource.is_some() {
            thread::spawn(|| handle_request(self.uid, resource.unwrap().url));
        }
    }
}

#[derive(Deserialize, Debug)]
struct WSProxyPing {}

impl WSProxyRequest for WSProxyPing {
    fn handle(self, _config: Config) {
        println!("Ping")
    }
}

struct WSProxyUnknownRequest {}

impl WSProxyRequest for WSProxyUnknownRequest {
    fn handle(self, _config: Config) {
        println!("Unknown request")
    }
}

#[derive(Serialize, Debug)]
struct ResourceResponse {
    status: i32,
    body: String,
}

fn read_config_from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let c = serde_json::from_reader(reader)?;
    Ok(c)
}

fn read_ws_message(value: String) -> Box<dyn WSProxyRequest> {
    let msg: Value = serde_json::from_str(value.as_str()).unwrap();
    let msg_type = msg["type"].as_str().unwrap();
    match msg_type {
        "ping" => {
            Box::new(WSProxyPing {})
        }
        "request" => {
            Box::new(WSProxyCallRequest {
                uid: msg["uid"].to_string(),
                resource: msg["resource"].to_string(),
            })
        }
        _ => {
            Box::new(WSProxyUnknownRequest {})
        }
    }
}

fn call_resource(request_id: String, url: String) -> Result<ResourceResponse, Box<dyn Error>> {
    // println!("{:?}", config.get_resource(resource_name));
    // let body = reqwest::get(config.get_resource(resource_name)).await?.text().await?;
    println!("{}: {}", request_id, url);

    let r = ResourceResponse { status: 200, body: "test".to_string() };
    Ok(r)
}

// fn send_answer() {
//
// }

fn handle_request(request_id: String, url: String) {
    let response = call_resource(request_id, url).unwrap();
    println!("{:?}", response);
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("Prometheus websocket proxy")
        .version("2.0.0")
        .author("Roman Karpovich <fpm.th13f@gmail.com>")
        .about("Connects to websocket server to call local resources")
        .arg(Arg::new("config").help("path to config").takes_value(true))
        .arg(Arg::new("parallel").long("parallel").help("number of connections to use").takes_value(true))
        .get_matches();

    let config_path = matches.value_of("config").unwrap_or("client_config.json");
    println!("The file passed is: {}", config_path);
    let config = read_config_from_file(config_path).unwrap();

    println!("instance: {} target: {}", config.instance, config.target);
    println!("{:?}", config.resources);

    println!("Connecting to {}", config.target);

    let instance_name = config.instance.to_owned();

    let client = ClientBuilder::new(config.target.as_str())
        .unwrap()
        // .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    println!("Successfully connected");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("Send Loop: {:?}", e);
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
                    println!("Send Loop: {:?}", e);
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
                    println!("Receive Loop: {:?}", e);
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    println!("Close received");
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    println!("Ping received");
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            println!("Receive Loop: {:?}", e);
                            return;
                        }
                    }
                }
                OwnedMessage::Text(value) => {
                    println!("Receive Loop: {:?}", value);
                    let request = read_ws_message(value);
                    request.handle(config);
                    return;
                }
                _ => {
                    println!("Receive Loop: {:?}", message);
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
            println!("Main Loop: {:?}", e);
        }
    }
    println!("Successfully registered");

    // loop {
    // 	let mut input = String::new();
    //
    // 	stdin().read_line(&mut input).unwrap();
    //
    // 	let trimmed = input.trim();
    //
    // 	let message = match trimmed {
    // 		"/close" => {
    // 			// Close the connection
    // 			let _ = tx.send(OwnedMessage::Close(None));
    // 			break;
    // 		}
    // 		// Send a ping
    // 		"/ping" => OwnedMessage::Ping(b"PING".to_vec()),
    // 		// Otherwise, just send text
    // 		_ => OwnedMessage::Text(trimmed.to_string()),
    // 	};
    //
    // 	match tx.send(message) {
    // 		Ok(()) => (),
    // 		Err(e) => {
    // 			println!("Main Loop: {:?}", e);
    // 			break;
    // 		}
    // 	}
    // }

    // We're exiting

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("Exited");

    Ok(())
}
