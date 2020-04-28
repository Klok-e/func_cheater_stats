use crate::error::MainError;
use crate::typed_db::TypedDb;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::identity;

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub struct ChatId(pub i64);

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Clone)]
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
    was_chat_imported: TypedDb<ChatName, bool>,
}

impl Persist {
    pub fn new(
        db: sled::Db,
        msg_db: sled::Db,
        imported_messages: sled::Db,
        was_chat_imported: sled::Db,
    ) -> Self {
        Self {
            users: TypedDb::new(db),
            messages: TypedDb::new(msg_db),
            imported_messages: TypedDb::new(imported_messages),
            was_chat_imported: TypedDb::new(was_chat_imported),
        }
    }

    pub fn add_message(&self, chat_id: ChatId, msg: ChatMessage) -> Result<(), MainError> {
        let mut messages = self.messages.get(&chat_id)?.map_or(Vec::new(), identity);
        messages.push(msg.clone());
        self.messages.insert(&chat_id, messages)?;
        log::info!("message {:?} added to chat {:?}", &msg, &chat_id);
        Ok(())
    }

    pub fn add_imported_message(
        &self,
        chat_name: ChatName,
        msg: ChatMessage,
    ) -> Result<(), MainError> {
        let mut messages = self
            .imported_messages
            .get(&chat_name)?
            .map_or(Vec::new(), identity);
        messages.push(msg.clone());
        self.imported_messages.insert(&chat_name, messages)?;
        log::info!("imported message {:?} added to chat {:?}", &msg, &chat_name);
        Ok(())
    }

    pub fn clear_messages(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.messages.insert(&chat_id, Vec::<ChatMessage>::new())?;
        log::info!("messages cleared in chat {:?}", &chat_id);
        Ok(())
    }

    pub fn clear_imported_messages(&self, chat: ChatName) -> Result<(), MainError> {
        self.imported_messages
            .insert(&chat, Vec::<ChatMessage>::new())?;
        log::info!("imported messages cleared in chat {:?}", &chat);
        Ok(())
    }

    pub fn get_messages(&self, chat_id: ChatId) -> Result<Vec<ChatMessage>, MainError> {
        Ok(self.messages.get(&chat_id)?.map_or(Vec::new(), identity))
    }

    //pub fn get_imported_messages(
    //    &self,
    //    chat_name: ChatName,
    //) -> Result<Vec<ChatMessage>, MainError> {
    //    Ok(self
    //        .imported_messages
    //        .get(&chat_name)?
    //        .map_or(Vec::new(), identity))
    //}

    pub fn messages_imported_to_regular(
        &self,
        chat_name: ChatName,
        chat_id: ChatId,
    ) -> Result<(), MainError> {
        let imported = self.imported_messages.get(&chat_name)?;
        match imported {
            Some(v) => self.messages.insert(&chat_id, v)?,
            None => (),
        };
        self.was_chat_imported.insert(&chat_name, true)?;
        log::info!(
            "converted imported messages from chat {:?} to chat {:?}",
            &chat_name,
            &chat_id
        );
        Ok(())
    }

    pub fn is_chat_imported(&self, chat_name: ChatName) -> Result<bool, MainError> {
        Ok(self
            .was_chat_imported
            .get(&chat_name)?
            .map_or(false, identity))
    }

    pub fn reset_imported(&self, chat_name: ChatName) -> Result<(), MainError> {
        log::info!("reset is_imported for chat {:?}", chat_name);
        Ok(self.was_chat_imported.insert(&chat_name, false)?)
    }

    pub fn add_user(&self, chat_id: ChatId, user: CodeUser) -> Result<(), MainError> {
        let mut map = match self.users.get(&chat_id)? {
            None => HashMap::new(),
            Some(val) => val,
        };
        let user1 = user.clone();
        map.insert(user1.telegram_id, user1);
        self.users.insert(&chat_id, map)?;
        log::info!("user {:?} added in chat {:?}", &user, &chat_id);
        Ok(())
    }

    pub fn remove_user(&self, chat_id: ChatId, user_to_remove: UserId) -> Result<(), MainError> {
        let mut users: HashMap<UserId, CodeUser> =
            self.users.get(&chat_id)?.map_or(HashMap::new(), identity);
        users.remove(&user_to_remove);
        self.users.insert(&chat_id, users)?;
        log::info!("user {:?} removed in chat {:?}", &user_to_remove, &chat_id);
        Ok(())
    }

    pub fn clear_users(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.users
            .insert(&chat_id, HashMap::<UserId, CodeUser>::new())?;
        log::info!("users cleared in chat {:?}", &chat_id);
        Ok(())
    }

    pub fn get_users(&self, chat_id: ChatId) -> Result<HashMap<UserId, CodeUser>, MainError> {
        Ok(self.users.get(&chat_id)?.map_or(HashMap::new(), identity))
    }
}
