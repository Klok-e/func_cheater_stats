use crate::error::MainError;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

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
