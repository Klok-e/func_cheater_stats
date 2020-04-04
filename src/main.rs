use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::MessageKind;
use teloxide::utils::command::BotCommand;
use tokio::prelude::*;

#[derive(Error, From, Debug, Display)]
enum MainError {
    LogFile(io::Error),
    LogInit(log::SetLoggerError),
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

#[derive(Serialize, Deserialize, Hash)]
struct ChatId(i64);

#[derive(Serialize, Deserialize)]
struct CodeUser {
    telegram_name: String,
    codewars_name: String,
}

#[derive(Serialize, Deserialize, Default)]
struct AddedUsers {
    chats: HashMap<ChatId, Vec<CodeUser>>,
}

struct Persist {
    users: AddedUsers,
    db: sled::Db,
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
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("logs.log")?)
        .apply()?;

    let db = sled::open("users")?;
    let mut persist = Persist {
        db,
        users: load(&db)?,
    };

    let token = std::env::var("TELEGRAM_TOKEN")
        .expect("TELEGRAM_TOKEN env variable expected but wasn't found");
    let bot = Bot::new(token);
    Dispatcher::new(bot)
        .messages_handler(move |rx| handle_commands(rx, persist))
        .dispatch()
        .await;

    Ok(())
}

async fn handle_commands(rx: DispatcherHandlerRx<Message>, mut db: Persist) {
    rx.commands("CodeWarsCheatBot")
        .for_each_concurrent(
            None,
            |(cx, command, strings): (DispatcherHandlerCx<Message>, Command, _)| async move {
                answer(cx, command, &mut db).await.log_on_error().await;
            },
        )
        .await;
}

async fn answer(
    cx: DispatcherHandlerCx<Message>,
    command: Command,
    db: &mut Persist,
) -> ResponseResult<()> {
    Command::pa
    match command {
        Command::Help => cx.answer(Command::descriptions()).send().await?,
        Command::DeleteMe => {}
        Command::AddMe => {
            if let MessageKind::Common { from, .. } = cx.update.kind {
                if let Some(user) = from {
                    db.users
                        .chats
                        .entry(ChatId(cx.update.chat.id))
                        .or_default()
                        .push();
                    cx.answer(format!(
                        "Added user {}",
                        user.username.unwrap_or("Unknown".to_owned())
                    ))
                }
            }
        }
        Command::ShowStats => {}
        Command::Clear => {}
    }
    Ok(())
}

fn load(db: &sled::Db) -> sled::Result<AddedUsers> {
    let str = db.get(0)?;
    if let Some(str) = str {
        OK(serde_json::from_slice(str.as_ref()))
    } else {
        Ok(Default::default())
    }
}

fn save(data: &AddedUsers, db: &sled::Db) -> sled::Result<()> {
    db.insert(0, serde_json::to_string(data))?;
    Ok(())
}
