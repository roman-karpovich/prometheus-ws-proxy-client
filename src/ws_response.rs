use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ResourceResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Serialize)]
pub struct ResponseMessage {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub uid: String,
    pub body: String,
    pub status: u16,
}
