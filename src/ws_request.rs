use crate::config::Config;
use crate::ws_response::{WSReadyMessage, WSResponseMessage};
use async_trait::async_trait;
use futures::channel::mpsc::UnboundedSender;
use futures_util::SinkExt;
use log::{debug, error, warn};
use serde::Deserialize;
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

#[async_trait]
pub trait WSProxyRequest {
    async fn handle(&self, worker: String, config: Config, mut tx: UnboundedSender<Message>);
}

#[derive(Deserialize, Debug)]
pub struct WSProxyReadyRequest {
    pub uid: String,
}

#[async_trait]
impl WSProxyRequest for WSProxyReadyRequest {
    async fn handle(&self, worker: String, _config: Config, mut tx: UnboundedSender<Message>) {
        let response_message = WSReadyMessage {
            message_type: "ready".to_string(),
            uid: self.uid.clone(),
            worker,
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
}

#[derive(Deserialize, Debug)]
pub struct WSProxyCallRequest {
    pub uid: String,
    pub resource: String,
}

#[async_trait]
impl WSProxyRequest for WSProxyCallRequest {
    async fn handle(&self, _worker: String, config: Config, mut tx: UnboundedSender<Message>) {
        let uid = self.uid.clone();
        let resource_name = self.resource.to_string();
        debug!("Handling request {}: {}", uid, resource_name);
        let resource_map = &config.resources;
        let resource_url = resource_map.get(resource_name.as_str());
        if resource_url.is_none() {
            warn!("unable to find resource with name {}", resource_name);
            let response_message = WSResponseMessage {
                message_type: "response".to_string(),
                uid,
                body: "No such resource".to_string(),
                status: 404,
            };
            let response_json = serde_json::to_string(&response_message).unwrap();
            debug!("{:?}", response_json);
            match tx.send(Message::Text(response_json)).await {
                Ok(()) => (),
                Err(e) => {
                    error!("Handle Request: {:?}", e);
                }
            }
            return;
        }

        let resource_url = resource_url.unwrap().to_string();

        debug!("resource_url: {}", resource_url);

        crate::worker::handle_request(uid, resource_url, tx).await;
    }
}

#[derive(Deserialize, Debug)]
pub struct WSProxyPing {}

#[async_trait]
impl WSProxyRequest for WSProxyPing {
    async fn handle(&self, _worker: String, _config: Config, mut tx: UnboundedSender<Message>) {
        match tx
            .send(Message::Text("{\"type\": \"pong\"}".to_string()))
            .await
        {
            Ok(()) => (),
            Err(e) => {
                error!("Handle Request: {:?}", e);
            }
        }
    }
}

pub struct WSProxyUnknownRequest {}

#[async_trait]
impl WSProxyRequest for WSProxyUnknownRequest {
    async fn handle(&self, _worker: String, _config: Config, _tx: UnboundedSender<Message>) {
        warn!("Unknown request")
    }
}

pub fn read_ws_message(value: String) -> Box<dyn WSProxyRequest> {
    let msg: Value = serde_json::from_str(value.as_str()).unwrap();
    let msg_type = msg["type"].as_str().unwrap();
    match msg_type {
        "ping" => Box::new(WSProxyPing {}),
        "request" => Box::new(WSProxyCallRequest {
            uid: msg["uid"].as_str().unwrap().to_string(),
            resource: msg["resource"].as_str().unwrap().to_string(),
        }),
        "ready" => Box::new(WSProxyReadyRequest {
            uid: msg["uid"].as_str().unwrap().to_string(),
        }),
        _ => Box::new(WSProxyUnknownRequest {}),
    }
}
