use crate::db::{ChatId, ChatMessage, ChatName, CodeUser, Persist, UserId};
use crate::error::{CodewarsApiError, MainError};
use crate::message_parse::{is_codewars_solution, kata_name_link};
use crate::parsing_types::{Text, TextData};
use crate::stats::compute_stats;
use derive_more::{Display, Error, From};
use itertools::Itertools;
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
use teloxide::types::{ChatKind, InputFile, MessageKind, ParseMode};
use teloxide::utils::command::BotCommand;
use tokio::prelude::*;

mod codewars_requests;
mod db;
mod error;
mod message_parse;
mod parsing_types;
mod stats;
mod typed_db;
mod utils;

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
    #[command(description = "show solved")]
    ShowSolved,
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

    let was_imported = sled::open("was_imported")?;
    let imported = sled::open("imported_msgs")?;
    let messages = sled::open("messages")?;
    let db = sled::open("users")?;
    let persist = Arc::new(Persist::new(db, messages, imported, was_imported));

    // remove tmp dir
    let tmp = Path::new("tmp/");
    if tmp.exists() {
        std::fs::remove_dir_all(tmp).unwrap();
    }

    // import messages
    let data_path = Path::new("exported_messages.json");
    if data_path.exists() {
        use parsing_types::ExportedData;
        let messages = std::fs::read_to_string(data_path).unwrap();
        let data: ExportedData = serde_json::from_str(messages.as_str()).unwrap();
        for chat in data.chats.list.iter() {
            if let Some(ref chat_name) = chat.name {
                persist.clear_messages(ChatId(chat.id)).unwrap();
                persist
                    .clear_imported_messages(ChatName(chat_name.clone()))
                    .unwrap();
                persist.reset_imported(ChatName(chat_name.clone())).unwrap();
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
                            .add_imported_message(
                                ChatName(chat_name.clone()),
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

        //cx.answer("Registered!").send().await?;
        } else {
            log::info!("{} ----- isn't a codewars solution", text);
        }
    }
    Ok(())
}

async fn handle_messages(rx: DispatcherHandlerRx<Message>, db: Arc<Persist>) {
    rx.for_each_concurrent(None, |cx| async {
        async {
            if let Some(text) = cx.update.text() {
                // import messages for this chat
                match match cx.update.chat.kind.clone() {
                    ChatKind::NonPrivate {
                        title: Some(title), ..
                    } => Some(title),
                    ChatKind::Private {
                        first_name: Some(first_name),
                        ..
                    } => Some(first_name),
                    _ => None,
                } {
                    Some(chat_name) => {
                        if !db.is_chat_imported(ChatName(chat_name.clone()))? {
                            db.messages_imported_to_regular(
                                ChatName(chat_name),
                                ChatId(cx.chat_id()),
                            )?
                        }
                    }
                    None => (),
                };

                // handle message
                if let Some((command, args)) = Command::parse(text, "CodeWarsCheatStats_bot") {
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
            Result::<_, MainError>::Ok(())
        }
        .await
        .log_on_error()
        .await;
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
                        match db.add_user(
                            ChatId(cx.update.chat_id()),
                            CodeUser {
                                telegram_id: UserId(from.id),
                                codewars_name: codewars_name.clone(),
                                username: from.username.clone(),
                                firstname: from.first_name.clone(),
                            },
                        ) {
                            Err(e) => {
                                answer_text = format!(
                                    "Couldn't add user {} with codewars username {} because of a serialization failure",
                                    from.first_name,
                                    &codewars_name
                                );
                                log::warn!("Error {} while adding a new user", e);
                            }
                            Ok(_) => {
                                answer_text = format!(
                                    "Added user {} with codewars username {}",
                                    from.first_name, &codewars_name
                                );
                            }
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
                    if let Ok(us) = db.get_users(ChatId(cx.chat_id())) {
                        if let Ok(msg) = db.get_messages(ChatId(cx.chat_id())) {
                            match compute_stats(us, msg).await {
                                Ok(path) => {
                                    cx.answer_photo(InputFile::file(path)).send().await?;
                                }
                                Err(MainError::CodewarsApi(CodewarsApiError::NotFound(name))) => {
                                    cx.answer(format!("User not found in Codewars API: {}", name))
                                        .send()
                                        .await?;
                                }
                                Err(e) => {
                                    cx.answer(format!("Error while getting stats: {}", e))
                                        .send()
                                        .await?;
                                }
                            }
                        } else {
                            cx.answer("Internal error 1").send().await?;
                        }
                    } else {
                        cx.answer("Couldn't get user data due to an internal error")
                            .send()
                            .await?;
                    };
                }
                Command::Clear => {
                    let mut answer = "Cleared all users for this chat";
                    if !db.clear_users(ChatId(cx.update.chat_id())).is_ok() {
                        answer = "Couldn't clear users due to a serialization failure"
                    }
                    cx.answer(answer).send().await?;
                }
                Command::ShowSolved => {
                    let messages = match db.get_messages(ChatId(cx.chat_id())) {
                        Ok(msgs) => msgs,
                        Err(e) => {
                            log::warn!("Error while getting messages {}", e);
                            Vec::new()
                        }
                    };
                    let answer = if messages.is_empty() {
                        "No solved katas".to_owned()
                    } else {
                        let messages: Vec<_> = messages
                            .into_iter()
                            .map(|msg| kata_name_link(msg.text.as_str()))
                            .unique()
                            .sorted()
                            .collect();
                        format!(
                            "The following katas were solved:\n{}",
                            messages
                                .into_iter()
                                .map(|m| format!("[{}]({})", m.0, m.1))
                                .join("\n")
                        )
                    };
                    for answer in dbg!(utils::chunk_with_size(answer.as_str())) {
                        let mut m = cx.answer(answer);
                        if std::env::var("DONT_SEND_MARKDOWN").map_or(true, |_| false) {
                            m = m.parse_mode(ParseMode::MarkdownV2);
                        }
                        m.disable_web_page_preview(true).send().await?;
                    }
                }
            }
        }
    }
    Ok(())
}
