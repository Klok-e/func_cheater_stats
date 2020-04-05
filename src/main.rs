use crate::parsing_types::{Text, TextData};
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

mod parsing_types;

#[derive(Error, From, Debug, Display)]
enum MainError {
    LogFile(io::Error),
    LogInit(log::SetLoggerError),
    Sled(sled::Error),
    Serde(serde_json::Error),
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display help.")]
    Help,
    #[command(description = "add a user")]
    AddMe,
    #[command(description = "delete me")]
    DeleteMe,
    #[command(description = "clear users")]
    Clear,
    #[command(description = "show stats")]
    ShowStats,
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
struct ChatId(i64);

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
struct UserId(i32);

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CodeUser {
    username: Option<String>,
    firstname: String,
    telegram_id: UserId,
    codewars_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    id: i32,
    text: String,
    from: UserId,
}

struct Persist {
    db: sled::Db,
    messages: sled::Db,
}

impl Persist {
    fn new(db: sled::Db, msg_db: sled::Db) -> Self {
        Self {
            db,
            messages: msg_db,
        }
    }

    fn add_message(&self, chat_id: ChatId, msg: ChatMessage) -> Result<(), MainError> {
        let mut messages = match self
            .messages
            .get(serde_json::to_string(&chat_id)?.as_bytes())
            .unwrap()
        {
            None => Vec::new(),
            Some(vec) => serde_json::from_slice(vec.as_ref())?,
        };
        messages.push(msg.clone());
        self.db
            .insert(
                serde_json::to_string(&chat_id)?.as_bytes(),
                serde_json::to_string(&messages)?.as_bytes(),
            )
            .unwrap();
        log::info!("message {:?} added to chat {:?}", &msg, &chat_id);
        Ok(())
    }

    fn add_user(&self, chat_id: ChatId, user: CodeUser) -> Result<(), MainError> {
        let mut map = match self
            .db
            .get(serde_json::to_string(&chat_id)?.as_bytes())
            .unwrap()
        {
            None => HashMap::new(),
            Some(val) => serde_json::from_slice(val.as_ref())?,
        };
        let user1 = user.clone();
        map.insert(user1.telegram_id, user1.codewars_name);
        self.db
            .insert(
                serde_json::to_string(&chat_id)?.as_bytes(),
                serde_json::to_string(&map)?.as_bytes(),
            )
            .unwrap();
        log::info!("user {:?} added in chat {:?}", &user, &chat_id);
        Ok(())
    }

    fn remove_user(&self, chat_id: ChatId, user_to_remove: UserId) -> Result<(), MainError> {
        let mut users: HashMap<UserId, String> = self
            .db
            .get(serde_json::to_string(&chat_id)?.as_bytes())
            .unwrap()
            .map_or(Ok(HashMap::new()), |v| -> Result<_, serde_json::Error> {
                Ok(serde_json::from_slice(v.as_ref())?)
            })?;
        users.remove(&user_to_remove);
        self.db
            .insert(
                serde_json::to_string(&chat_id)?.as_bytes(),
                serde_json::to_string(&users)?.as_bytes(),
            )
            .unwrap();
        log::info!("user {:?} removed in chat {:?}", &user_to_remove, &chat_id);
        Ok(())
    }

    fn clear_users(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.db.insert(
            serde_json::to_string(&chat_id)?.as_bytes(),
            serde_json::to_string(&HashMap::<UserId, String>::new())?.as_bytes(),
        )?;
        log::info!("users cleared in chat {:?}", &chat_id);
        Ok(())
    }

    fn clear_messages(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.db.insert(
            serde_json::to_string(&chat_id)?.as_bytes(),
            serde_json::to_string(&Vec::<ChatMessage>::new())?.as_bytes(),
        )?;
        log::info!("messages cleared in chat {:?}", &chat_id);
        Ok(())
    }

    fn get_users(&self, chat_id: ChatId) -> Result<HashMap<UserId, String>, MainError> {
        Ok(self
            .db
            .get(serde_json::to_string(&chat_id)?.as_bytes())
            .unwrap()
            .map_or(Ok(HashMap::new()), |v| -> Result<_, serde_json::Error> {
                Ok(serde_json::from_slice(v.as_ref())?)
            })?)
    }
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("logs.log")?)
        .apply()?;

    let messages = sled::open("messages")?;
    let db = sled::open("users")?;
    let persist = Arc::new(Persist::new(db, messages));

    let data_path = Path::new("exported_messages.json");
    if data_path.exists() {
        use parsing_types::ExportedData;
        let messages = std::fs::read_to_string(data_path).unwrap();
        let data: ExportedData = serde_json::from_str(messages.as_str()).unwrap();
        for chat in data.chats.list.iter() {
            persist.clear_messages(ChatId(chat.id)).unwrap();
            for msg in chat.messages.iter().filter(|msg| msg.msg_type == "message") {
                let msg_text = match msg.text.as_ref().unwrap() {
                    Text::String(s) => s.clone(),
                    Text::Links(vec) => vec
                        .iter()
                        .map(|t| {
                            match t {
                                TextData::String(s) => s,
                                TextData::Typed { text, .. } => text,
                            }
                            .clone()
                        })
                        .collect::<Vec<_>>()
                        .join(""),
                };

                if is_codewars_solution(msg_text.as_str()) {
                    persist
                        .add_message(
                            ChatId(chat.id),
                            ChatMessage {
                                id: msg.id,
                                from: UserId(msg.from_id.unwrap()),
                                text: msg_text,
                            },
                        )
                        .unwrap();
                }
            }
        }
        std::fs::rename(
            data_path,
            format!("used_{}", data_path.file_name().unwrap().to_str().unwrap()),
        )
        .unwrap();
    }

    let token = std::env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN env variable expected but wasn't found");
    let bot = Bot::new(token);
    Dispatcher::new(bot)
        .messages_handler(move |rx| handle_messages(rx, persist.clone()))
        .dispatch()
        .await;

    Ok(())
}

async fn store_message(cx: DispatcherHandlerCx<Message>, db: Arc<Persist>) -> ResponseResult<()> {
    if let (Some(text), Some(from)) = (cx.update.text(), cx.update.from()) {
        if is_codewars_solution(text) {
            log::info!("{} ----- is a codewars solution", text);
            match db.add_message(
                ChatId(cx.chat_id()),
                ChatMessage {
                    from: UserId(from.id),
                    text: text.to_owned(),
                    id: cx.update.id,
                },
            ) {
                Ok(_) => (),
                Err(e) => log::warn!("Error while processing messages: {}", e),
            }

            cx.answer("Registered!").send().await?;
        } else {
            log::info!("{} ----- isn't a codewars solution", text);
        }
    }
    Ok(())
}

lazy_static! {
    static ref IS_SOLUTION_REGEX: regex::Regex =
        regex::Regex::new(r"^\d\D*https://pastebin.com/").unwrap();
}

fn is_codewars_solution(msg: &str) -> bool {
    IS_SOLUTION_REGEX.is_match(msg)
}

async fn handle_messages(rx: DispatcherHandlerRx<Message>, db: Arc<Persist>) {
    rx.for_each_concurrent(None, |cx| async {
        if let Some(text) = cx.update.text() {
            if let Some((command, args)) = Command::parse(text, "CodeWarsCheatStatsBot") {
                // handle commands
                answer_command(&cx, command, db.clone(), args)
                    .await
                    .log_on_error()
                    .await;
            } else {
                // handle messages
                store_message(cx, db.clone()).await.log_on_error().await;
            }
        }
    })
    .await;
}

async fn answer_command(
    cx: &DispatcherHandlerCx<Message>,
    command: Command,
    db: Arc<Persist>,
    args: Vec<&str>,
) -> ResponseResult<()> {
    if let MessageKind::Common { ref from, .. } = cx.update.kind {
        if let Some(from) = from {
            match command {
                Command::Help => {
                    cx.answer(Command::descriptions()).send().await?;
                }
                Command::DeleteMe => {
                    let answer_text;
                    if !db
                        .remove_user(ChatId(cx.chat_id()), UserId(from.id))
                        .map_err(|e| {
                            log::warn!("{}", e);
                            e
                        })
                        .is_ok()
                    {
                        answer_text = format!(
                            "Couldn't remove user {} due to a serialization error",
                            from.first_name
                        );
                    } else {
                        answer_text = format!("Removed user {} successfully", from.first_name)
                    }
                    cx.answer(answer_text).send().await?;
                }
                Command::AddMe => {
                    let answer_text;
                    if args.len() == 1 {
                        let codewars_name = args.first().unwrap().to_string();
                        if !db
                            .add_user(
                                ChatId(cx.update.chat_id()),
                                CodeUser {
                                    telegram_id: UserId(from.id),
                                    codewars_name: codewars_name.clone(),
                                    username: from.username.clone(),
                                    firstname: from.first_name.clone(),
                                },
                            )
                            .is_ok()
                        {
                            answer_text = format!(
                                "Couldn't add user {} with codewars username {} because of a serialization failure",
                                from.first_name,
                                &codewars_name
                            );
                        } else {
                            answer_text = format!(
                                "Added user {} with codewars username {}",
                                from.first_name, &codewars_name
                            );
                        }
                    } else {
                        answer_text = format!(
                            "Couldn't add user {} because codewars username wasn't supplied",
                            from.first_name,
                        );
                    }
                    cx.answer(answer_text).send().await?;
                }
                Command::ShowStats => {
                    let text;
                    if let Ok(us) = db.get_users(ChatId(cx.chat_id())) {
                        text = format!("Not implemented yet. Here's a list of all users to keep yourself entertained:\n{}",
                                       us.iter().map(|u| format!("{:?}", u)).collect::<Vec<_>>().join("\n"));
                    } else {
                        text = "Couldn't get user data due to an internal error".to_owned();
                    };

                    cx.answer(text).send().await?;
                }
                Command::Clear => {
                    let mut answer = "Cleared all users for this chat";
                    if !db.clear_users(ChatId(cx.update.chat_id())).is_ok() {
                        answer = "Couldn't clear users due to a serialization failure"
                    }
                    cx.answer(answer).send().await?;
                }
            }
        }
    }
    Ok(())
}
