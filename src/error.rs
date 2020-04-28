use derive_more::{Display, Error, From};
use std::error::Error;
use tokio::prelude::*;

#[derive(Error, From, Debug, Display)]
pub enum MainError {
    LogFile(io::Error),
    LogInit(log::SetLoggerError),
    Sled(sled::Error),
    Serde(serde_json::Error),
    Network(reqwest::Error),
    CodewarsApi(CodewarsApiError),
}

#[derive(Debug, Display)]
pub enum CodewarsApiError {
    NotFound(String),
}

impl Error for CodewarsApiError {}
