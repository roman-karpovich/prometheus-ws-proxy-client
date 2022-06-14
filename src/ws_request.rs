use std::sync::mpsc::Sender;
use std::thread;
use crate::config::Config;
use crate::OwnedMessage;
use serde::Deserialize;
use serde_json::Value;

pub trait WSProxyRequest {
    fn handle(&self, config: &Config, tx: Sender<OwnedMessage>);
}

#[derive(Deserialize, Debug)]
pub struct WSProxyCallRequest {
    pub uid: String,
    pub resource: String,
}

impl WSProxyRequest for WSProxyCallRequest {
    fn handle(&self, config: &Config, tx: Sender<OwnedMessage>) {
        let uid = self.uid.clone();
        let resource_name = &self.resource;
        // println!("Handling request {}: {}", uid, resource_name);
        let resource_map = &config.resources;
        let resource_url = resource_map.get(resource_name);
        if resource_url.is_none() {
            println!("unable to find resource with name {}", resource_name);
            return;
        }

        let resource_url = resource_url.unwrap().to_string();

        // println!("resource_url: {}", resource_url);

        thread::spawn(|| crate::worker::handle_request(uid, resource_url, tx));
    }
}

#[derive(Deserialize, Debug)]
pub struct WSProxyPing {}

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

pub struct WSProxyUnknownRequest {}

impl WSProxyRequest for WSProxyUnknownRequest {
    fn handle(&self, _config: &Config, _tx: Sender<OwnedMessage>) {
        println!("Unknown request")
    }
}

pub fn read_ws_message(value: String) -> Box<dyn WSProxyRequest> {
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
