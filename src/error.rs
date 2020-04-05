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

#[derive(Error, From, Debug, Display)]
pub enum MainError {
    LogFile(io::Error),
    LogInit(log::SetLoggerError),
    Sled(sled::Error),
    Serde(serde_json::Error),
}
