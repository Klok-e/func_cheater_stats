use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};
use sled::IVec;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::MessageKind;
use teloxide::utils::command::BotCommand;
use tokio::prelude::*;

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
    #[command(description = "display this text.")]
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

#[derive(Serialize, Deserialize, Debug)]
struct CodeUser {
    telegram_id: UserId,
    codewars_name: String,
}

type SledValue = HashMap<UserId, String>;
type SledKey = ChatId;

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

    fn add_user(&self, chat_id: ChatId, user: CodeUser) -> Result<(), MainError> {
        let mut map = match self
            .db
            .get(serde_json::to_string(&chat_id)?.as_bytes())
            .unwrap()
        {
            None => HashMap::new(),
            Some(val) => serde_json::from_slice(val.as_ref())?,
        };
        map.insert(user.telegram_id, user.codewars_name);
        self.db.insert(
            serde_json::to_string(&chat_id)?.as_bytes(),
            serde_json::to_string(&map)?.as_bytes(),
        );
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
        Ok(())
    }

    fn clear(&self, chat_id: ChatId) -> Result<(), MainError> {
        self.db.insert(
            serde_json::to_string(&chat_id)?.as_bytes(),
            serde_json::to_string(&HashMap::<UserId, String>::new())?.as_bytes(),
        );
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
    let mut persist = Arc::new(Persist::new(db, messages));

    let token = std::env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN env variable expected but wasn't found");
    let bot = Bot::new(token);
    Dispatcher::new(bot)
        .messages_handler(move |rx| handle_commands(rx, persist.clone()))
        .dispatch()
        .await;

    Ok(())
}

async fn handle_commands(rx: DispatcherHandlerRx<Message>, db: Arc<Persist>) {
    rx.commands("CodeWarsCheatBot")
        .for_each_concurrent(None, |(cx, command, args)| async {
            answer(cx, command, db.clone(), args)
                .await
                .log_on_error()
                .await;
        })
        .await;
}

async fn answer(
    cx: DispatcherHandlerCx<Message>,
    command: Command,
    db: Arc<Persist>,
    args: Vec<String>,
) -> ResponseResult<()> {
    if let MessageKind::Common { ref from, .. } = cx.update.kind {
        if let Some(from) = from {
            match command {
                Command::Help => {
                    cx.answer(Command::descriptions()).send().await?;
                }
                Command::DeleteMe => {
                    let mut answer_text;
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
                    let mut answer_text;
                    if args.len() == 1 {
                        let codewars_name = args.first().unwrap().to_string();
                        if !db
                            .add_user(
                                ChatId(cx.update.chat_id()),
                                CodeUser {
                                    telegram_id: UserId(from.id),
                                    codewars_name: codewars_name.clone(),
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
                    let mut text;
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
                    if !db.clear(ChatId(cx.update.chat_id())).is_ok() {
                        answer = "Couldn't clear users due to a serialization failure"
                    }
                    cx.answer(answer).send().await?;
                }
            }
        }
    }
    Ok(())
}
