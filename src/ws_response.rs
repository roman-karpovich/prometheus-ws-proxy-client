use serde::Serialize;

#[derive(Serialize)]
pub struct WSRegisterMessageV1 {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub instance: String,
}

#[derive(Serialize)]
pub struct WSRegisterMessageV2 {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub instance: String,
    pub worker: String,
    pub version: u16,
}

#[derive(Serialize)]
pub struct WSReadyMessage {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub uid: String,
    pub worker: String,
}

#[derive(Serialize)]
pub struct WSResponseMessage {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub uid: String,
    pub body: String,
    pub status: u16,
}
