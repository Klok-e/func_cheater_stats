use crate::error::MainError;
use crate::parsing_types::{Text, TextData};
use crate::typed_db::TypedDb;
use derive_more::{Display, Error, From};
use lazy_static::lazy_static;
use regex;
use serde::{Deserialize, Serialize};
use sled::IVec;
use smart_default::SmartDefault;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::MessageKind;
use teloxide::utils::command::BotCommand;
use tokio::prelude::*;

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct ChatId(pub i64);

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct ChatName(pub String);

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct UserId(pub i32);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CodeUser {
    pub username: Option<String>,
    pub firstname: String,
    pub telegram_id: UserId,
    pub codewars_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub id: i32,
    pub text: String,
    pub from: UserId,
}

pub struct Persist {
    users: TypedDb<ChatId, HashMap<UserId, CodeUser>>,
    messages: TypedDb<ChatId, Vec<ChatMessage>>,
    imported_messages: TypedDb<ChatName, Vec<ChatMessage>>,
}

impl Persist {
    pub fn new(db: sled::Db, msg_db: sled::Db, imported_messages: sled::Db) -> Self {
        Self {
            users: TypedDb::new(db),
            messages: TypedDb::new(msg_db),
            imported_messages: TypedDb::new(imported_messages),
        }
    }

    pub fn add_message(&self, chat_id: ChatId, msg: ChatMessage) -> Result<(), MainError> {
        let mut messages = match self.messages.get(&chat_id).unwrap() {
            None => Vec::new(),
            Some(vec) => serde_json::from_slice(vec.as_ref())?,
        };
        messages.push(msg.clone());
        self.messages.insert(&chat_id, messages).unwrap();
        log::info!("message {:?} added to chat {:?}", &msg, &chat_id);
        Ok(())
    }

    pub fn add_user(&self, chat_id: ChatId, user: CodeUser) -> Result<(), MainError> {
        let mut map = match self.users.get(&chat_id).unwrap() {
            None => HashMap::new(),
            Some(val) => serde_json::from_slice(val.as_ref())?,
        };
        let user1 = user.clone();
        map.insert(user1.telegram_id, user1);
        self.users.insert(&chat_id, map).unwrap();
        log::info!("user {:?} added in chat {:?}", &user, &chat_id);
        Ok(())
    }

    pub fn remove_user(&self, chat_id: ChatId, user_to_remove: UserId) -> Result<(), MainError> {
        let mut users: HashMap<UserId, CodeUser> = self
            .users
            .get(&chat_id)
            .unwrap()
            .map_or(Ok(HashMap::new()), |v| -> Result<_, serde_json::Error> {
                Ok(serde_json::from_slice(v.as_ref())?)
            })?;
        users.remove(&user_to_remove);
        self.users.insert(&chat_id, users).unwrap();
        log::info!("user {:?} removed in chat {:?}", &user_to_remove, &chat_id);
        Ok(())
    }

    pub fn clear_users(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.users
            .insert(&chat_id, HashMap::<UserId, CodeUser>::new())?;
        log::info!("users cleared in chat {:?}", &chat_id);
        Ok(())
    }

    //pub fn clear_imported_messages(&self, chat_id: ChatId) -> Result<(), MainError> {
    //    self.imported_messages
    //        .insert(&chat_id, Vec::<ChatMessage>::new())?;
    //    log::info!("messages cleared in chat {:?}", &chat_id);
    //    Ok(())
    //}

    pub fn clear_messages(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.messages.insert(&chat_id, Vec::<ChatMessage>::new())?;
        log::info!("messages cleared in chat {:?}", &chat_id);
        Ok(())
    }

    pub fn get_users(&self, chat_id: ChatId) -> Result<HashMap<UserId, CodeUser>, MainError> {
        Ok(self
            .users
            .get(&chat_id)
            .unwrap()
            .map_or(Ok(HashMap::new()), |v| -> Result<_, serde_json::Error> {
                Ok(serde_json::from_slice(v.as_ref())?)
            })?)
    }

    pub fn get_messages(&self, chat_id: ChatId) -> Result<Vec<ChatMessage>, MainError> {
        Ok(match self.messages.get(&chat_id).unwrap() {
            Some(vec) => serde_json::from_slice(vec.as_ref())?,
            None => Vec::new(),
        })
    }
}
