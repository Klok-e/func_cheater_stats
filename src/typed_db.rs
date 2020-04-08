use crate::error::MainError;
use crate::parsing_types::{Text, TextData};
use derive_more::{Display, Error, From};
use lazy_static::lazy_static;
use regex;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::IVec;
use smart_default::SmartDefault;
use std::collections::HashMap;
use std::error::Error;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::MessageKind;
use teloxide::utils::command::BotCommand;
use tokio::prelude::*;

pub struct TypedDb<K, V> {
    inner: sled::Db,
    kv: PhantomData<(K, V)>,
}

impl<K, V> TypedDb<K, V>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    pub fn new(db: sled::Db) -> Self {
        Self {
            inner: db,
            kv: PhantomData::default(),
        }
    }

    pub fn get(&self, key: &K) -> Result<Option<V>, MainError> {
        self.inner
            .get(serde_json::to_vec(key)?.as_slice())?
            .map(|v| Ok(serde_json::from_slice(v.as_ref())?))
            .map_or(Ok(None), |r| r.map(Some))
    }

    pub fn insert(&self, key: &K, value: V) -> Result<(), MainError> {
        Ok(self
            .inner
            .insert(
                serde_json::to_vec(key)?.as_slice(),
                serde_json::to_vec(&value)?.as_slice(),
            )
            .map(|_| ())?)
    }
}
