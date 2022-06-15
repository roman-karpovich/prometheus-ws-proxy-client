use serde::Serialize;

#[derive(Serialize)]
pub struct WSResponseMessage {
    #[serde(rename(serialize = "type"))]
    pub message_type: String,
    pub uid: String,
    pub body: String,
    pub status: u16,
}
