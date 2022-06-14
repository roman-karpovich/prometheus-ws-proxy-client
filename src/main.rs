// fn main() {
//     println!("Hello, world!");
// }
extern crate websocket;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use clap::{Arg, Command};
use serde::{Serialize, Deserialize};
use serde_json::Value;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;
use names::Generator;
use rand::Rng;

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

#[derive(Serialize, Debug)]
struct ResourceResponse {
    status: u16,
    body: String,
}

#[derive(Serialize)]
struct ResponseMessage {
    #[serde(rename(serialize = "type"))]
    message_type: String,
    uid: String,
    body: String,
    status: u16,
}

impl Config {
    fn get_resource_map(&self) -> HashMap<&String, &String> {
        let mut result = HashMap::new();
        for resource in &self.resources[..] {
            result.insert(&resource.name, &resource.url);
        }
        result
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
    fn handle(&self, config: &Config, tx: Sender<OwnedMessage>);
}

#[derive(Deserialize, Debug)]
struct WSProxyCallRequest {
    uid: String,
    resource: String,
}

impl WSProxyRequest for WSProxyCallRequest {
    fn handle(&self, config: &Config, tx: Sender<OwnedMessage>) {
        let uid = self.uid.clone();
        let resource_name = &self.resource;
        // println!("Handling request {}: {}", uid, resource_name);
        let resource_map = config.get_resource_map();
        let resource_url = resource_map.get(resource_name);
        if resource_url.is_none() {
            println!("unable to find resource with name {}", resource_name);
            return;
        }

        let resource_url = resource_url.unwrap().to_string();

        // println!("resource_url: {}", resource_url);

        thread::spawn(|| handle_request(uid, resource_url, tx));
    }
}

#[derive(Deserialize, Debug)]
struct WSProxyPing {}

impl WSProxyRequest for WSProxyPing {
    fn handle(&self, _config: &Config, tx: Sender<OwnedMessage>) {
        match tx.send(OwnedMessage::Text("{\"type\": \"pong\"}".to_string())) {
            Ok(()) => (),
            Err(e) => {
                println!("Handle Request: {:?}", e);
            }
        }
    }
}

struct WSProxyUnknownRequest {}

impl WSProxyRequest for WSProxyUnknownRequest {
    fn handle(&self, _config: &Config, _tx: Sender<OwnedMessage>) {
        println!("Unknown request")
    }
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
                uid: msg["uid"].as_str().unwrap().to_string(),
                resource: msg["resource"].as_str().unwrap().to_string(),
            })
        }
        _ => {
            Box::new(WSProxyUnknownRequest {})
        }
    }
}

fn call_resource(url: &String) -> Result<ResourceResponse, Box<dyn Error>> {
    // println!("{}: {}", request_id, url);
    let response = reqwest::blocking::get(url)?;
    let r = ResourceResponse { status: response.status().as_u16(), body: response.text()? };
    Ok(r)
}

fn handle_request(request_id: String, url: String, tx: Sender<OwnedMessage>) {
    let response = call_resource(&url).unwrap();
    // send response
    let response_message = ResponseMessage {
        message_type: "response".to_string(),
        uid: request_id,
        body: response.body,
        status: response.status,
    };
    let response_json = serde_json::to_string(&response_message).unwrap();
    // println!("{:?}", response_json);
    match tx.send(OwnedMessage::Text(response_json)) {
        Ok(()) => (),
        Err(e) => {
            println!("Handle Request: {:?}", e);
        }
    }
}

fn main() {
    let matches = Command::new("Prometheus websocket proxy")
        .version("2.0.0")
        .author("Roman Karpovich <fpm.th13f@gmail.com>")
        .about("Connects to websocket server to call local resources")
        .arg(Arg::new("config").help("path to config").takes_value(true))
        .arg(Arg::new("parallel").long("parallel").help("number of connections to use").takes_value(true))
        .get_matches();

    let config_path = matches.value_of("config").unwrap_or("client_config.json");
    println!("Using config {}", config_path);

    let connections_number: usize = matches.value_of_t("parallel").unwrap_or(3);
    println!("Run {} workers", connections_number);

    let mut generator = Generator::default();
    let mut threads: Vec<_> = Vec::new();

    for _i in 0..connections_number {
        let worker_name = generator.next().unwrap();
        let local_config_path = config_path.clone().to_string();

        threads.push(thread::spawn(move || {
            run_worker(worker_name.as_str(), local_config_path.as_str());
        }));
    }

    for handle in threads {
        match handle.join() {
            Ok(()) => (),
            Err(e) => {
                println!("Main Loop: {:?}", e);
            }
        }
    }
}

fn connect_to_server(config_path: &str) -> Result<(), Box<dyn Error>> {
    let config = read_config_from_file(config_path).unwrap();
    println!("Instance: {}. Connecting to target: {}", config.instance, config.target);
    // println!("{:?}", config.resources);

    let instance_name = config.instance.to_owned();

    let client = ClientBuilder::new(config.target.as_str())
        .unwrap()
        .connect_insecure();
    if client.is_err() {
        Err("Unable to connect to the server")?;
    }
    let client = client.unwrap();

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
                            continue;
                        }
                    }
                }
                OwnedMessage::Text(value) => {
                    // println!("Receive Loop: {:?}", value);
                    let request = read_ws_message(value);
                    request.handle(&config, tx_1.clone());
                    continue;
                }
                _ => {
                    println!("Receive Loop (unknown message): {:?}", message);
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

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("Exited");

    Ok(())
}

fn run_worker(name: &str, config_path: &str) {
    println!("Worker {} starting", name);
    loop {
        match connect_to_server(config_path) {
            Ok(()) => (),
            Err(e) => {
                println!("Worker {} exited: {:?}", name, e);
            }
        }
        let mut rng = rand::thread_rng();
        sleep(Duration::from_secs(1 + rng.gen_range(0..5)));
    }
}
