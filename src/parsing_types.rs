use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportedData {
    pub chats: ChatsData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatsData {
    pub list: Vec<ChatData>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ChatData {
    pub id: i64,
    pub messages: Vec<MessageData>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct MessageData {
    pub id: i32,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub text: Option<Text>,
    pub from_id: Option<i32>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Text {
    String(String),
    Links(Vec<TextData>),
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TextData {
    String(String),
    Typed {
        #[serde(rename = "type")]
        text_type: String,
        text: String,
    },
}
